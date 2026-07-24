// 感官词库单元测试
// ===================
// 测试 Vocab 的 YAML 加载、候选集生成、validate_selection 校验逻辑。

use novels::models::*;
use novels::vocab::{Vocab, validate};
use std::collections::HashSet;

fn sample_yaml() -> &'static str {
    r#"
visual:
  bloodstain:
    text: "血迹"
    tags: ["injury"]
auditory:
  footsteps:
    text: "脚步声"
    tags: ["movement"]
"#
}

#[test]
fn loads_all_senses() {
    // 验证 YAML 能正确加载所有感官类别
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    assert!(v.has("visual", "bloodstain"));
    assert!(v.has("auditory", "footsteps"));
    assert!(!v.has("visual", "footsteps")); // 跨类别不匹配
}

#[test]
fn candidates_filter_by_tag() {
    // 验证按标签过滤候选集
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let cands = v.candidates("visual", &["injury"]);
    assert_eq!(cands.len(), 1);
    assert_eq!(cands[0].as_str(), "visual.bloodstain");
}

#[test]
fn candidates_for_tags_include_semantic_metadata() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let candidates = v.candidates_for_tags(&["injury".to_string()]);

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].id, "visual.bloodstain");
    assert_eq!(candidates[0].sense, "visual");
    assert_eq!(candidates[0].text, "血迹");
    assert_eq!(candidates[0].tags, vec!["injury"]);
}

#[test]
fn candidates_for_tags_respects_total_cap() {
    let yaml = r#"
visual:
  a: { text: "a", tags: ["t"] }
  b: { text: "b", tags: ["t"] }
  c: { text: "c", tags: ["t"] }
emotion:
  d: { text: "d", tags: ["t"] }
  e: { text: "e", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let c = v.candidates_for_tags_limited(&["t".into()], 2, 3);
    assert!(c.len() <= 3);
    // 确定性：同输入多次结果一致（VocabularyCandidate 无 PartialEq，比 id 列表）
    let c2 = v.candidates_for_tags_limited(&["t".into()], 2, 3);
    let ids: Vec<_> = c.iter().map(|x| x.id.as_str()).collect();
    let ids2: Vec<_> = c2.iter().map(|x| x.id.as_str()).collect();
    assert_eq!(ids, ids2);
    // per_sense=2 且 sense 序 visual→…→emotion：先 visual.a/b，再 emotion.d，共 3
    assert_eq!(ids, vec!["visual.a", "visual.b", "emotion.d"]);
}

#[test]
fn known_tags_ranked_prefers_query_name() {
    let yaml = r#"
visual:
  a: { text: "x", tags: ["甲", "无关"] }
  b: { text: "y", tags: ["乙", "林黛玉"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked = v.known_tags_ranked_limited(&["林黛玉".into()], 2);
    assert_eq!(ranked[0], "林黛玉");
}

#[test]
fn candidates_ranked_voice_boost_prefers_named_character_tag() {
    let yaml = r#"
gesture:
  other: { text: "他冷笑一声", tags: ["t", "贾琏"] }
  self: { text: "宝玉点头", tags: ["t", "宝玉"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked = v.candidates_ranked_limited(&["t".into()], &["宝玉".into()], 2, 2);
    assert_eq!(ranked[0].id, "gesture.self");
}

#[test]
fn candidates_ranked_prefers_query_overlap_over_dict_order() {
    let yaml = r#"
visual:
  zzz_noise: { text: "无关景物", tags: ["t"] }
  aaa_hit: { text: "宝玉站在雨中", tags: ["t", "宝玉"] }
emotion:
  mid: { text: "心中一恸", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let selected = vec!["t".into()];
    let query = vec!["宝玉".into()];
    let ranked = v.candidates_ranked_limited(&selected, &query, 2, 2);
    let ids: Vec<_> = ranked.iter().map(|c| c.id.as_str()).collect();
    // "宝玉" hits aaa_hit hardest; must rank first (not zzz_noise dict order).
    assert_eq!(ids[0], "visual.aaa_hit");
    // Determinism
    let ranked2 = v.candidates_ranked_limited(&selected, &query, 2, 2);
    assert_eq!(
        ranked.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
        ranked2.iter().map(|c| c.id.as_str()).collect::<Vec<_>>()
    );
}

#[test]
fn bm25_ranks_repeated_term_higher() {
    // TF weight: same term twice in text should beat once (no voice-tag confound).
    let yaml = r#"
visual:
  once: { text: "雨夜独坐", tags: ["t"] }
  twice: { text: "雨夜又见雨夜", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked = v.candidates_ranked_limited(&["t".into()], &["雨夜".into()], 2, 2);
    assert_eq!(ranked[0].id, "visual.twice");
}

#[test]
fn bm25_rare_term_beats_common_term() {
    // IDF: rare term 「紫菱洲」 beats frequent 「宝玉」 when both are in the query.
    let yaml = r#"
visual:
  rare_hit: { text: "紫菱洲边", tags: ["t"] }
  common_hit: { text: "宝玉", tags: ["t"] }
  filler_a: { text: "宝玉走了", tags: ["t"] }
  filler_b: { text: "宝玉来了", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked =
        v.candidates_ranked_limited(&["t".into()], &["紫菱洲".into(), "宝玉".into()], 4, 4);
    assert_eq!(ranked[0].id, "visual.rare_hit");
}

#[test]
fn bm25_empty_query_falls_back_to_stable_order() {
    let yaml = r#"
visual:
  z: { text: "z", tags: ["t"] }
  a: { text: "a", tags: ["t"] }
emotion:
  m: { text: "m", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked = v.candidates_ranked_limited(&["t".into()], &[], 2, 3);
    let ids: Vec<_> = ranked.iter().map(|c| c.id.as_str()).collect();
    // Empty query → BM25=0, equal selected boost → SENSES then id: visual.a, visual.z, emotion.m
    assert_eq!(ids, vec!["visual.a", "visual.z", "emotion.m"]);
}

#[test]
fn bm25_preserves_voice_isolation() {
    // Exact name tag must still beat a BM25-only text hit for a different character.
    let yaml = r#"
gesture:
  other: { text: "宝玉宝玉宝玉", tags: ["t", "贾琏"] }
  self: { text: "点头", tags: ["t", "宝玉"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked = v.candidates_ranked_limited(&["t".into()], &["宝玉".into()], 2, 2);
    assert_eq!(ranked[0].id, "gesture.self");
}

#[test]
fn candidates_for_tags_fall_back_when_tags_have_no_entry_match() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let candidates = v.candidates_for_tags(&["known-but-unmatched".to_string()]);

    assert_eq!(candidates.len(), 2);
}

#[test]
fn known_tags_and_filter_known_tags_are_sorted_and_deduplicated() {
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let tags = [
        "movement".to_string(),
        "unknown".to_string(),
        "injury".to_string(),
        "movement".to_string(),
    ];

    assert_eq!(v.known_tags(), vec!["injury", "movement"]);
    assert_eq!(v.filter_known_tags(&tags), vec!["injury", "movement"]);
}

#[test]
fn candidates_for_tags_fall_back_when_filter_known_tags_drops_all() {
    // 回归测试：filter_known_tags 产出的空选择必须触发 candidates_for_tags 全量回退。
    let v = Vocab::load_from_str(sample_yaml()).unwrap();
    let absent = "absent-tag".to_string();
    let filtered = v.filter_known_tags(&[absent]);
    assert!(filtered.is_empty());

    let candidates = v.candidates_for_tags(&filtered);
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].id, "visual.bloodstain");
    assert_eq!(candidates[1].id, "auditory.footsteps");
}

#[test]
fn validate_strips_unknown_ids() {
    // 非法 ID 应被 validate 移除，合法的保留
    let _v = Vocab::load_from_str(sample_yaml()).unwrap();
    let mut set = HashSet::new();
    set.insert("visual.bloodstain".to_string());
    set.insert("auditory.footsteps".to_string());
    let mut sel = SensorySelection::default();
    sel.visual_ids
        .push(VocabularyId::new("visual.bloodstain").unwrap());
    sel.visual_ids
        .push(VocabularyId::new("visual.unknown").unwrap());
    let result = validate(&sel, &set);
    assert_eq!(result.cleaned.visual_ids.len(), 1);
    assert!(!result.stripped.is_empty());
}

#[test]
fn validate_rejects_wrong_sense() {
    // 全局候选集中存在、但属于其他感官类别的 ID 也必须被拒绝。
    let mut set = HashSet::new();
    set.insert("visual.bloodstain".to_string());
    set.insert("auditory.footsteps".to_string());
    let mut sel = SensorySelection::default();
    sel.visual_ids
        .push(VocabularyId::new("auditory.footsteps").unwrap());
    let result = validate(&sel, &set);
    assert_eq!(result.cleaned.visual_ids.len(), 0);
    assert_eq!(result.stripped, vec!["auditory.footsteps"]);
    assert!(result.all_empty);
}

#[test]
fn loads_and_validates_new_categories() {
    // emotion/gesture/atmosphere 三个新类别:加载、候选集、校验全链路
    let yaml = r#"
emotion:
  hlm-c001-01:
    text: "满纸荒唐言，一把辛酸泪"
    tags: ["hlm", "sorrow"]
gesture:
  zhz-c003-02:
    text: "她用帕子不断擦拭着脸上的泪水"
    tags: ["zhz", "sorrow"]
atmosphere:
  hlm-c005-03:
    text: "香烟缭绕，花影缤纷"
    tags: ["hlm", "indoor"]
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    assert!(v.has("emotion", "hlm-c001-01"));
    assert!(v.has("gesture", "zhz-c003-02"));
    assert!(v.has("atmosphere", "hlm-c005-03"));

    let set = v.candidate_set(&[]);
    assert_eq!(set.len(), 3);

    let mut sel = SensorySelection::default();
    sel.emotion_ids
        .push(VocabularyId::new("emotion.hlm-c001-01").unwrap());
    sel.gesture_ids
        .push(VocabularyId::new("emotion.hlm-c001-01").unwrap()); // 跨类别,应剥离
    let result = validate(&sel, &set);
    assert_eq!(result.cleaned.emotion_ids.len(), 1);
    assert!(result.cleaned.gesture_ids.is_empty());
    assert_eq!(result.stripped, vec!["emotion.hlm-c001-01"]);
    assert!(!result.all_empty);
}

#[test]
fn merge_overlays_distilled_vocab() {
    // merge 把蒸馏素材库叠加到基础词库,两边条目都可见
    let mut base = Vocab::load_from_str(sample_yaml()).unwrap();
    let distilled = Vocab::load_from_str(
        r#"
emotion:
  zhz-c001-01:
    text: "这一分别，我从此便生活在深宫之中"
    tags: ["zhz"]
"#,
    )
    .unwrap();
    base.merge(distilled);
    assert!(base.has("visual", "bloodstain"));
    assert!(base.has("emotion", "zhz-c001-01"));
    assert_eq!(base.candidate_set(&[]).len(), 3);
}

#[test]
fn load_runtime_vocab_merges_base_and_distilled_fixture() {
    use novels::vocab::load_runtime_vocab;
    use std::path::Path;

    let base = Path::new("assets/vocab.yaml");
    // 目录接口：fixture 放在 tests/fixtures/distilled_dir/
    let (v, report) =
        load_runtime_vocab(base, Some(Path::new("tests/fixtures/distilled_dir"))).unwrap();
    assert!(v.has("visual", "bloodstain")); // base
    assert!(v.has("emotion", "hlm-c001-01") || report.distilled_files >= 1);
    assert!(report.total_entries > report.base_entries);
}

#[test]
fn quota_gives_weak_sense_floor_under_tilted_pool() {
    // Tilted pool: 20 gesture + 20 emotion entries dominate, but weak senses
    // (auditory/olfactory/tactile/gustatory) have only 3 entries each.
    // Quota retrieval must still give each weak sense at least min(5, 3) = 3.
    let mut yaml = String::new();
    yaml.push_str("gesture:\n");
    for i in 0..20 {
        yaml.push_str(&format!("  g{i}: {{ text: \"g{i}\", tags: [\"t\"] }}\n"));
    }
    yaml.push_str("emotion:\n");
    for i in 0..20 {
        yaml.push_str(&format!("  e{i}: {{ text: \"e{i}\", tags: [\"t\"] }}\n"));
    }
    // Weak senses: only 3 each
    for sense in &["auditory", "olfactory", "tactile", "gustatory"] {
        yaml.push_str(&format!("{sense}:\n"));
        for i in 0..3 {
            yaml.push_str(&format!("  w{i}: {{ text: \"w{i}\", tags: [\"t\"] }}\n"));
        }
    }
    let v = Vocab::load_from_str(&yaml).unwrap();

    let ranked = v.candidates_ranked_limited_with_quotas(
        &["t".into()],
        &[],
        novels::vocab::DEFAULT_PER_SENSE_CAP,
        novels::vocab::DEFAULT_TOTAL_CAP,
    );

    // Each weak sense gets all 3 available (floor = min(5, 3) = 3).
    for sense in &["auditory", "olfactory", "tactile", "gustatory"] {
        let count = ranked.iter().filter(|c| c.sense == *sense).count();
        assert_eq!(count, 3, "weak sense {sense} should get floor of 3");
    }

    // gesture/emotion capped at 15 each.
    assert_eq!(ranked.iter().filter(|c| c.sense == "gesture").count(), 15);
    assert_eq!(ranked.iter().filter(|c| c.sense == "emotion").count(), 15);

    // Total <= 96.
    assert!(ranked.len() <= 96);
}

#[test]
fn quota_gives_weak_sense_floor_5_when_available() {
    // Enough weak sense entries: floor should be 5 each.
    let mut yaml = String::new();
    yaml.push_str("gesture:\n");
    for i in 0..10 {
        yaml.push_str(&format!("  g{i}: {{ text: \"g{i}\", tags: [\"t\"] }}\n"));
    }
    for sense in &["auditory", "olfactory", "tactile", "gustatory"] {
        yaml.push_str(&format!("{sense}:\n"));
        for i in 0..8 {
            yaml.push_str(&format!("  w{i}: {{ text: \"w{i}\", tags: [\"t\"] }}\n"));
        }
    }
    let v = Vocab::load_from_str(&yaml).unwrap();

    let ranked = v.candidates_ranked_limited_with_quotas(
        &["t".into()],
        &[],
        novels::vocab::DEFAULT_PER_SENSE_CAP,
        novels::vocab::DEFAULT_TOTAL_CAP,
    );

    for sense in &["auditory", "olfactory", "tactile", "gustatory"] {
        let count = ranked.iter().filter(|c| c.sense == *sense).count();
        assert!(
            count >= 5,
            "weak sense {sense} should get at least floor of 5, got {count}"
        );
    }
}

#[test]
fn quota_sense_focus_boosts_relevant_senses() {
    // Query "血" should boost visual/olfactory scores.
    let yaml = r#"
visual:
  blood: { text: "血迹", tags: ["t"] }
  noise: { text: "无关", tags: ["t"] }
olfactory:
  bloodsmell: { text: "血腥味", tags: ["t"] }
gesture:
  g1: { text: "摇头", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked = v.candidates_ranked_limited_with_quotas(&["t".into()], &["血".into()], 24, 96);
    // visual.blood and olfactory.bloodsmell should rank above visual.noise and gesture.g1
    let ids: Vec<_> = ranked.iter().map(|c| c.id.as_str()).collect();
    assert!(
        ids.iter().position(|&id| id == "visual.blood").unwrap()
            < ids.iter().position(|&id| id == "visual.noise").unwrap()
    );
    assert!(
        ids.iter()
            .position(|&id| id == "olfactory.bloodsmell")
            .unwrap()
            < ids.iter().position(|&id| id == "gesture.g1").unwrap()
    );
}

#[test]
fn focus_map_rain_keyword_boosts_rain_senses() {
    // Query "雨" should boost visual/auditory/tactile (expanded FOCUS_MAP entries).
    let yaml = r#"
visual:
  raindrop: { text: "雨丝", tags: ["t"] }
  noise: { text: "无关", tags: ["t"] }
auditory:
  rainsound: { text: "雨声", tags: ["t"] }
tactile:
  wet: { text: "湿冷", tags: ["t"] }
gesture:
  g1: { text: "摇头", tags: ["t"] }
"#;
    let v = Vocab::load_from_str(yaml).unwrap();
    let ranked = v.candidates_ranked_limited_with_quotas(&["t".into()], &["雨".into()], 24, 96);
    let ids: Vec<_> = ranked.iter().map(|c| c.id.as_str()).collect();
    // All rain-sense hits must rank above the unrelated visual.noise and gesture.g1.
    assert!(
        ids.iter().position(|&id| id == "visual.raindrop").unwrap()
            < ids.iter().position(|&id| id == "visual.noise").unwrap()
    );
    assert!(
        ids.iter()
            .position(|&id| id == "auditory.rainsound")
            .unwrap()
            < ids.iter().position(|&id| id == "gesture.g1").unwrap()
    );
    assert!(
        ids.iter().position(|&id| id == "tactile.wet").unwrap()
            < ids.iter().position(|&id| id == "gesture.g1").unwrap()
    );
}

#[test]
fn quota_total_never_exceeds_cap() {
    let mut yaml = String::new();
    for sense in &[
        "visual",
        "auditory",
        "olfactory",
        "tactile",
        "gustatory",
        "emotion",
        "gesture",
        "atmosphere",
    ] {
        yaml.push_str(&format!("{sense}:\n"));
        for i in 0..20 {
            yaml.push_str(&format!("  k{i}: {{ text: \"k{i}\", tags: [\"t\"] }}\n"));
        }
    }
    let v = Vocab::load_from_str(&yaml).unwrap();
    let ranked = v.candidates_ranked_limited_with_quotas(
        &["t".into()],
        &[],
        novels::vocab::DEFAULT_PER_SENSE_CAP,
        96,
    );
    assert!(ranked.len() <= 96);
}
