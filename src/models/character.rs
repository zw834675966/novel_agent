use super::CharacterId;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Character {
    pub id: CharacterId,
    pub name: String,
    pub personality: Vec<String>,
    pub skills: Vec<String>,
}
