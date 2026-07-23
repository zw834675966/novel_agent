use axum::{
    body::Body,
    http::{Request, StatusCode, header::CONTENT_TYPE},
};
use http_body_util::BodyExt;
use novels::{
    api,
    db::Db,
    llm::MockSenseGenerator,
    models::{Certainty, CharacterId, CharacterMemoryDraft, MemorySource, SensorySelection},
    scene::StoryService,
    vocab::Vocab,
};
use std::sync::Arc;
use tower::ServiceExt;

async fn test_service() -> StoryService {
    let db = Db::open_in_memory().await.unwrap();
    let vocab = Vocab::load_from_str("visual:\n  x:\n    text: x\n    tags: []\n").unwrap();
    let generator = Arc::new(MockSenseGenerator::new(
        novels::llm::LlmContextTagSelection::default(),
        novels::llm::LlmCharacterDerivation {
            sensory_analysis: String::new(),
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: "mock".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![],
            relationship_candidates: vec![],
        },
    ));
    StoryService::new(
        db,
        vocab,
        generator,
        Arc::new(novels::prose::MockProseGenerator::fallback()),
        Arc::new(novels::prose::MockScenePlanner::fallback()),
    )
}

#[tokio::test]
async fn create_scene_rejects_blank_objective_event() {
    let response = api::app(test_service().await)
        .oneshot(
            Request::post("/api/scenes")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"objective_event":"","participant_ids":[],"occurred_at":"2024-01-01T00:00:00Z"}"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn create_character_rejects_body_over_one_mebibyte_with_413() {
    let name = "x".repeat(1024 * 1024);
    let response = api::app(test_service().await)
        .oneshot(
            Request::post("/api/characters")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(format!(
                    r#"{{"name":"{name}","personality":[],"skills":[]}}"#
                )))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);
}

#[tokio::test]
async fn create_character_rejects_malformed_json_with_validation_error() {
    let response = api::app(test_service().await)
        .oneshot(
            Request::post("/api/characters")
                .header(CONTENT_TYPE, "application/json")
                .body(Body::from(r#"{"name":"broken"#))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn derive_scene_returns_derivations_and_errors() {
    let svc = test_service().await;
    let character_id = CharacterId(uuid::Uuid::new_v4());
    svc.db()
        .characters()
        .create(character_id, "actor", &[], &[])
        .await
        .unwrap();
    let scene_id = svc
        .create_scene(novels::models::CreateScene {
            objective_event: "test".into(),
            participant_ids: vec![character_id],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let response = api::app(svc)
        .oneshot(
            Request::post(format!("/api/scenes/{}/derive", scene_id.0))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);
    let results: Vec<serde_json::Value> =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert!(!results.is_empty());
}
