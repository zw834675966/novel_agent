// 全链路实证：BM25 检索贴戏度 + 装配回源检出 + KPI 数值
// =======================================================
// 加载真实 distilled 词库（assets/distilled/，~13.9k 条），用红楼梦/甄嬛传
// 真实场景 query 测 BM25 候选排序是否把"贴戏"片段排到 cap 内，
// 并验证装配后 provenance scan / quality report 的真实数值。

use novels::vocab::{DEFAULT_PER_SENSE_CAP, DEFAULT_TOTAL_CAP, Vocab};

/// 加载真实运行时词库（base + distilled 全量）
fn runtime_vocab() -> Vocab {
    let mut v = Vocab::load_from_path(std::path::Path::new("assets/vocab.yaml")).unwrap();
    v.load_dir_merged(std::path::Path::new("assets/distilled"))
        .unwrap();
    v
}

#[test]
fn real_bm25_ranks_scene_relevant_fragments_to_top() {
    let v = runtime_vocab();
    // 红楼梦场景：宝玉在怡红院，黛玉来访
    let selected = vec!["宝玉".to_string()];
    let query = vec!["宝玉".to_string(), "怡红院".to_string(), "黛玉".to_string()];
    let ranked =
        v.candidates_ranked_limited(&selected, &query, DEFAULT_PER_SENSE_CAP, DEFAULT_TOTAL_CAP);

    assert!(
        !ranked.is_empty(),
        "should have candidates from real corpus"
    );

    // Top-10 里应至少有一条 text 含「宝玉」或「黛玉」（贴戏）
    let top10 = &ranked[..ranked.len().min(10)];
    let hits = top10
        .iter()
        .filter(|c| c.text.contains("宝玉") || c.text.contains("黛玉"))
        .count();
    assert!(
        hits >= 1,
        "BM25 top-10 should surface scene-relevant (宝玉/黛玉) fragments; got {hits} in:\n{}",
        top10
            .iter()
            .map(|c| format!("  {} | {}", c.id, c.text))
            .collect::<Vec<_>>()
            .join("\n")
    );

    // 确定性
    let ranked2 =
        v.candidates_ranked_limited(&selected, &query, DEFAULT_PER_SENSE_CAP, DEFAULT_TOTAL_CAP);
    let ids1: Vec<_> = ranked.iter().map(|c| c.id.as_str()).collect();
    let ids2: Vec<_> = ranked2.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(ids1, ids2, "BM25 ranking must be deterministic");
}

#[test]
fn real_bm25_beats_dict_order_for_rare_character_query() {
    let v = runtime_vocab();
    // 甄嬛传：查「华妃」--稀有角色名，字典序 cap 会把它排到后面甚至 cap 外
    let selected = vec![];
    let query = vec!["华妃".to_string()];
    let ranked =
        v.candidates_ranked_limited(&selected, &query, DEFAULT_PER_SENSE_CAP, DEFAULT_TOTAL_CAP);

    // Top-10 应有含「华妃」的片段
    let top10 = &ranked[..ranked.len().min(10)];
    let hits = top10.iter().filter(|c| c.text.contains("华妃")).count();
    assert!(
        hits >= 1,
        "rare character 华妃 should surface in BM25 top-10, not be buried by dict order; got {hits}"
    );

    eprintln!("\n=== BM25 top-10 for query=华妃 (corpus 13.9k) ===");
    for (i, c) in top10.iter().enumerate() {
        eprintln!("{i:>2}. {} | {}", c.id, c.text);
    }
}

#[test]
fn real_bm25_vs_dict_quantitative() {
    let v = runtime_vocab();
    let selected = vec!["宝玉".to_string()];
    let query = vec!["宝玉".to_string(), "怡红院".to_string()];
    let ranked =
        v.candidates_ranked_limited(&selected, &query, DEFAULT_PER_SENSE_CAP, DEFAULT_TOTAL_CAP);

    let top24 = &ranked[..ranked.len().min(24)];
    let relevant: usize = top24
        .iter()
        .filter(|c| c.text.contains("宝玉") || c.text.contains("怡红院"))
        .count();
    eprintln!("\n=== BM25 top-24 relevance for query=宝玉,怡红院 ===");
    eprintln!("relevant (含宝玉/怡红院): {relevant}/24");
    for (i, c) in top24.iter().take(10).enumerate() {
        let mark = if c.text.contains("宝玉") || c.text.contains("怡红院") {
            "★"
        } else {
            " "
        };
        eprintln!("{i:>2}.{mark} {} | {}", c.id, c.text);
    }
    assert!(
        relevant >= 1,
        "BM25 should surface at least 1 relevant in top-24"
    );
}

#[tokio::test]
async fn real_provenance_scan_zero_on_clean_assemble() {
    use novels::db::Db;
    use novels::llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator};
    use novels::models::StoryError;
    use novels::models::*;
    use novels::prose::{LlmNarrative, NarrativeBeat, ProseGenerator};
    use novels::scene::StoryService;
    use std::sync::Arc;

    struct FixedProse(LlmNarrative);
    #[async_trait::async_trait]
    impl ProseGenerator for FixedProse {
        async fn narrate(
            &self,
            _req: &novels::prose::NarrateRequest,
        ) -> Result<LlmNarrative, StoryError> {
            Ok(self.0.clone())
        }
    }

    let v = runtime_vocab();
    // 从真实库挑一条 gesture 片段做注入
    let gid = v
        .entries("gesture")
        .and_then(|m| m.keys().next().cloned())
        .unwrap();
    let raw_id = format!("gesture.{gid}");
    let text = v
        .entries("gesture")
        .unwrap()
        .get(&gid)
        .unwrap()
        .text
        .clone();

    let db = Db::open_in_memory().await.unwrap();
    let sense = Arc::new(MockSenseGenerator::new(
        LlmContextTagSelection::default(),
        LlmCharacterDerivation {
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: "m".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![],
        },
    ));
    let cid = CharacterId(uuid::Uuid::new_v4());
    let cid_str = cid.0.to_string();
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid_str,
            action: "她转身。".into(),
            sensation_refs: vec![raw_id.clone()],
        }],
    };
    let svc = StoryService::new(db, v, sense, Arc::new(FixedProse(narrative)));

    svc.db()
        .characters()
        .create(cid, "甲", &[], &[])
        .await
        .unwrap();
    let sid = svc
        .create_scene(CreateScene {
            objective_event: "事件".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    let mut sens = SensorySelection::default();
    sens.gesture_ids.push(VocabularyId::new(&raw_id).unwrap());
    let d = CharacterDerivation {
        character_id: cid,
        scene_id: sid,
        sensations: sens,
        new_memory: CharacterMemoryDraft {
            content: "m".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };
    let prose = svc.narrate_scene(sid, &[d]).await.unwrap();

    assert!(prose.text.contains(&text), "injected quote should appear");
    assert_eq!(prose.unverified_quotes, 0, "clean assemble: 0 unverified");
    assert_eq!(prose.quality.unverified_quotes, 0);
    assert!(prose.quote_density() > 0.0, "should have quote chars");
    assert!(prose.quality.quote_density > 0.0);
}

#[tokio::test]
async fn real_quality_report_flags_sparse_prose() {
    use novels::db::Db;
    use novels::llm::{LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator};
    use novels::models::StoryError;
    use novels::models::*;
    use novels::prose::{LlmNarrative, NarrativeBeat, ProseGenerator};
    use novels::scene::StoryService;
    use std::sync::Arc;

    struct EmptyProse(LlmNarrative);
    #[async_trait::async_trait]
    impl ProseGenerator for EmptyProse {
        async fn narrate(
            &self,
            _req: &novels::prose::NarrateRequest,
        ) -> Result<LlmNarrative, StoryError> {
            Ok(self.0.clone())
        }
    }

    let v = runtime_vocab();
    // Select one primary sense so the derivation is NOT degraded; the beat
    // still carries no refs, so quote_chars stays 0 and low_quote_density
    // flags the action-only prose. Without this, the degraded fallback inject
    // (see prose_test::degraded_fallback_pool_from_ranked_candidates_injects_five_sense)
    // would add five-sense quotes from the ranked corpus pool, masking the signal.
    let visual_key = v
        .entries("visual")
        .and_then(|m| m.keys().next().cloned())
        .expect("real corpus has visual entries");
    let db = Db::open_in_memory().await.unwrap();
    let sense = Arc::new(MockSenseGenerator::new(
        LlmContextTagSelection::default(),
        LlmCharacterDerivation {
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: "m".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![],
        },
    ));
    let cid = CharacterId(uuid::Uuid::new_v4());
    let cid_str = cid.0.to_string();
    // Pure action, no refs -> quote_density 0 -> low_quote_density true
    let narrative = LlmNarrative {
        beats: vec![NarrativeBeat {
            pov: cid_str,
            action: "她独自转身离去。".into(),
            sensation_refs: vec![],
        }],
    };
    let svc = StoryService::new(db, v, sense, Arc::new(EmptyProse(narrative)));

    svc.db()
        .characters()
        .create(cid, "甲", &[], &[])
        .await
        .unwrap();
    let sid = svc
        .create_scene(CreateScene {
            objective_event: "事件".into(),
            participant_ids: vec![cid],
            occurred_at: chrono::Utc::now(),
        })
        .await
        .unwrap();

    // Select one primary sense so the derivation is NOT degraded; the beat
    // still carries no refs, so quote_chars stays 0 and low_quote_density
    // flags the action-only prose. (visual_key extracted above before `v` moved.)
    let mut sensations = SensorySelection::default();
    sensations
        .visual_ids
        .push(VocabularyId::new(&format!("visual.{visual_key}")).unwrap());

    let d = CharacterDerivation {
        character_id: cid,
        scene_id: sid,
        sensations,
        new_memory: CharacterMemoryDraft {
            content: "m".into(),
            source: MemorySource::Witnessed,
            certainty: Certainty::Certain,
        },
        plot_development: vec![],
    };
    let prose = svc.narrate_scene(sid, &[d]).await.unwrap();

    assert_eq!(prose.quote_chars, 0);
    assert!(
        prose.quality.low_quote_density,
        "action-only prose should flag low density"
    );
    assert_eq!(prose.unverified_quotes, 0);
    assert!(
        (prose.quality.action_only_rate - 1.0).abs() < 1e-9,
        "1 accepted beat, 1 action-only"
    );
}
