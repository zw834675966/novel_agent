use novels::models::*;

#[test]
fn vocabulary_id_roundtrip() {
    let id = VocabularyId::new("visual.bloodstain").unwrap();
    let s = serde_json::to_string(&id).unwrap();
    let back: VocabularyId = serde_json::from_str(&s).unwrap();
    assert_eq!(id, back);
    assert_eq!(id.sense(), "visual");
    assert_eq!(id.key(), "bloodstain");
}

#[test]
fn vocabulary_id_rejects_missing_dot() {
    assert!(VocabularyId::new("noseparator").is_err());
}

#[test]
fn vocabulary_id_rejects_empty_segment() {
    assert!(VocabularyId::new(".foo").is_err());
    assert!(VocabularyId::new("foo.").is_err());
}
