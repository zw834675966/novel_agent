use novels::models::{CandidateStatus, RelationshipType};

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
