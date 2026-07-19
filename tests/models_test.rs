// 模型层单元测试
// =================
// 测试 VocabularyId 的序列化/反序列化和格式校验逻辑。

use novels::models::*;

#[test]
fn vocabulary_id_roundtrip() {
    // 验证 VocabularyId 的 JSON 序列化/反序列化一致性
    let id = VocabularyId::new("visual.bloodstain").unwrap();
    let s = serde_json::to_string(&id).unwrap();
    let back: VocabularyId = serde_json::from_str(&s).unwrap();
    assert_eq!(id, back);
    assert_eq!(id.sense(), "visual");
    assert_eq!(id.key(), "bloodstain");
}

#[test]
fn vocabulary_id_rejects_missing_dot() {
    // 不包含 '.' 的字符串应被拒绝
    assert!(VocabularyId::new("noseparator").is_err());
}

#[test]
fn vocabulary_id_rejects_empty_segment() {
    // '.' 前后有空段的格式应被拒绝
    assert!(VocabularyId::new(".foo").is_err());
    assert!(VocabularyId::new("foo.").is_err());
}
