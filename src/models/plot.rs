use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlotDevelopmentKind {
    SuspicionRaised,
    ConflictEscalated,
    GoalChanged,
    RelationshipShifted,
    NewClue,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PlotDevelopment {
    pub kind: PlotDevelopmentKind,
    pub reason: String,
}
