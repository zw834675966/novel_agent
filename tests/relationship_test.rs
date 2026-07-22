use chrono::Utc;
use novels::db::{CandidateResolution, Db, PendingCandidate};
use novels::models::{CandidateStatus, CharacterId, RelationshipFactId, RelationshipType, SceneId};
use uuid::Uuid;

#[test]
fn relationship_type_uses_stable_snake_case_json_values() {
    assert_eq!(
        serde_json::to_string(&RelationshipType::Distrusts).unwrap(),
        "\"distrusts\""
    );
    assert_eq!(
        serde_json::from_str::<RelationshipType>("\"family_of\"").unwrap(),
        RelationshipType::FamilyOf
    );
}

#[test]
fn pending_candidate_cannot_be_resolved_twice() {
    assert!(CandidateStatus::Pending.can_transition_to(CandidateStatus::Accepted));
    assert!(!CandidateStatus::Accepted.can_transition_to(CandidateStatus::Rejected));
}

#[test]
fn validate_score_rejects_over_100() {
    use novels::models::validate_score;
    assert!(validate_score(Some(101)).is_err());
    assert!(validate_score(Some(100)).is_ok());
    assert!(validate_score(Some(0)).is_ok());
    assert!(validate_score(None).is_ok());
}

// ---- Test helpers ----

async fn seed_two_characters_and_two_scenes(
    db: &Db,
) -> (CharacterId, CharacterId, SceneId, SceneId) {
    let a = CharacterId(Uuid::new_v4());
    let b = CharacterId(Uuid::new_v4());
    db.characters().create(a, "黛玉", &[], &[]).await.unwrap();
    db.characters().create(b, "宝玉", &[], &[]).await.unwrap();

    let first_scene = SceneId(Uuid::new_v4());
    let second_scene = SceneId(Uuid::new_v4());
    let t1 = Utc::now();
    let t2 = t1 + chrono::Duration::minutes(1);
    db.scenes()
        .create(first_scene, "初遇", &[a, b], t1)
        .await
        .unwrap();
    db.scenes()
        .create(second_scene, "冲突", &[a, b], t2)
        .await
        .unwrap();

    (a, b, first_scene, second_scene)
}

fn candidate(
    from: CharacterId,
    to: CharacterId,
    scene: SceneId,
    relationship_type: RelationshipType,
) -> PendingCandidate {
    PendingCandidate {
        scene_id: scene,
        from,
        to,
        relationship_type,
        summary: "临时摘要".into(),
        tension_score: None,
        trust_score: None,
        affection_score: None,
        power_score: None,
        evidence_memory_id: None,
        confidence: 0.8,
    }
}

// ---- Task 2 repository tests ----

#[tokio::test]
async fn accepting_new_revision_supersedes_previous_revision() {
    let db = Db::open_in_memory().await.unwrap();
    let (a, b, first_scene, second_scene) = seed_two_characters_and_two_scenes(&db).await;

    let first = db
        .relationships()
        .insert_pending(candidate(a, b, first_scene, RelationshipType::AlliedWith))
        .await
        .unwrap();
    db.relationships()
        .resolve_candidate(
            first.id,
            CandidateResolution::accept("共同目标使他们结盟", None),
        )
        .await
        .unwrap();

    let second = db
        .relationships()
        .insert_pending(candidate(a, b, second_scene, RelationshipType::AlliedWith))
        .await
        .unwrap();
    db.relationships()
        .resolve_candidate(
            second.id,
            CandidateResolution::accept("证据引发不信任", None),
        )
        .await
        .unwrap();

    let graph = db
        .relationships()
        .graph_snapshot_through(second_scene)
        .await
        .unwrap();
    assert_eq!(graph.edges.iter().filter(|edge| edge.active).count(), 1);
    let fact_id: RelationshipFactId = graph.edges[0].fact_id;
    assert_eq!(db.relationships().history(fact_id).await.unwrap().len(), 2);
}

#[tokio::test]
async fn rejecting_candidate_creates_no_fact_or_revision() {
    let db = Db::open_in_memory().await.unwrap();
    let (a, b, scene, _) = seed_two_characters_and_two_scenes(&db).await;
    let candidate = db
        .relationships()
        .insert_pending(candidate(a, b, scene, RelationshipType::Distrusts))
        .await
        .unwrap();
    db.relationships()
        .resolve_candidate(candidate.id, CandidateResolution::reject())
        .await
        .unwrap();
    assert!(
        db.relationships()
            .graph_snapshot_through(scene)
            .await
            .unwrap()
            .edges
            .is_empty()
    );
}

#[tokio::test]
async fn resolving_non_pending_candidate_returns_error() {
    let db = Db::open_in_memory().await.unwrap();
    let (a, b, scene, _) = seed_two_characters_and_two_scenes(&db).await;
    let candidate = db
        .relationships()
        .insert_pending(candidate(a, b, scene, RelationshipType::Distrusts))
        .await
        .unwrap();
    db.relationships()
        .resolve_candidate(candidate.id, CandidateResolution::reject())
        .await
        .unwrap();
    // Second resolution must fail - candidate already resolved
    let result = db
        .relationships()
        .resolve_candidate(candidate.id, CandidateResolution::reject())
        .await;
    assert!(result.is_err());
}

#[tokio::test]
async fn graph_snapshot_through_excludes_future_revisions() {
    let db = Db::open_in_memory().await.unwrap();
    let (a, b, first_scene, second_scene) = seed_two_characters_and_two_scenes(&db).await;

    // Create and accept in second scene (later)
    let candidate = db
        .relationships()
        .insert_pending(candidate(a, b, second_scene, RelationshipType::AlliedWith))
        .await
        .unwrap();
    db.relationships()
        .resolve_candidate(candidate.id, CandidateResolution::accept("后期结盟", None))
        .await
        .unwrap();

    // Graph through first scene (earlier) should have no edges
    let early_graph = db
        .relationships()
        .graph_snapshot_through(first_scene)
        .await
        .unwrap();
    assert!(early_graph.edges.is_empty());

    // Graph through second scene should have the edge
    let late_graph = db
        .relationships()
        .graph_snapshot_through(second_scene)
        .await
        .unwrap();
    assert_eq!(late_graph.edges.len(), 1);
    assert!(late_graph.edges[0].active);
}

#[tokio::test]
async fn list_candidates_for_scene_returns_pending_only() {
    let db = Db::open_in_memory().await.unwrap();
    let (a, b, scene, _) = seed_two_characters_and_two_scenes(&db).await;

    let c1 = db
        .relationships()
        .insert_pending(candidate(a, b, scene, RelationshipType::Distrusts))
        .await
        .unwrap();
    let _c2 = db
        .relationships()
        .insert_pending(candidate(a, b, scene, RelationshipType::AlliedWith))
        .await
        .unwrap();

    // Accept one
    db.relationships()
        .resolve_candidate(c1.id, CandidateResolution::accept("接受", None))
        .await
        .unwrap();

    let candidates = db
        .relationships()
        .list_candidates_for_scene(scene)
        .await
        .unwrap();
    // All candidates are returned (accepted + pending)
    assert_eq!(candidates.len(), 2);
    // One is accepted, one is pending
    assert!(
        candidates
            .iter()
            .any(|c| c.status == CandidateStatus::Accepted)
    );
    assert!(
        candidates
            .iter()
            .any(|c| c.status == CandidateStatus::Pending)
    );
}
