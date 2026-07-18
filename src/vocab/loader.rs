use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Deserialize)]
pub struct VocabEntry {
    pub text: String,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct VocabFile {
    #[serde(default)]
    pub visual: HashMap<String, VocabEntry>,
    #[serde(default)]
    pub auditory: HashMap<String, VocabEntry>,
    #[serde(default)]
    pub olfactory: HashMap<String, VocabEntry>,
    #[serde(default)]
    pub tactile: HashMap<String, VocabEntry>,
    #[serde(default)]
    pub gustatory: HashMap<String, VocabEntry>,
}

#[derive(Debug, Clone)]
pub struct Vocab {
    pub file: VocabFile,
}

impl Vocab {
    pub fn load_from_str(s: &str) -> Result<Self, super::super::models::StoryError> {
        let file: VocabFile = serde_yaml::from_str(s)
            .map_err(|e| super::super::models::StoryError::VocabularyLoad(e.to_string()))?;
        Ok(Self { file })
    }

    pub fn load_from_path(p: &std::path::Path) -> Result<Self, super::super::models::StoryError> {
        let s = std::fs::read_to_string(p)
            .map_err(|e| super::super::models::StoryError::VocabularyLoad(e.to_string()))?;
        Self::load_from_str(&s)
    }

    pub fn entries(&self, sense: &str) -> Option<&HashMap<String, VocabEntry>> {
        match sense {
            "visual" => Some(&self.file.visual),
            "auditory" => Some(&self.file.auditory),
            "olfactory" => Some(&self.file.olfactory),
            "tactile" => Some(&self.file.tactile),
            "gustatory" => Some(&self.file.gustatory),
            _ => None,
        }
    }

    pub fn has(&self, sense: &str, key: &str) -> bool {
        self.entries(sense)
            .map(|m| m.contains_key(key))
            .unwrap_or(false)
    }

    pub fn candidates(&self, sense: &str, tags: &[&str]) -> Vec<crate::models::VocabularyId> {
        let Some(map) = self.entries(sense) else {
            return vec![];
        };
        map.iter()
            .filter_map(|(k, e)| {
                if tags.is_empty() || tags.iter().any(|t| e.tags.iter().any(|et| et == t)) {
                    crate::models::VocabularyId::new(&format!("{sense}.{k}")).ok()
                } else {
                    None
                }
            })
            .collect()
    }

    pub fn candidate_set(&self, tags: &[&str]) -> std::collections::HashSet<String> {
        let mut set = std::collections::HashSet::new();
        for sense in ["visual", "auditory", "olfactory", "tactile", "gustatory"] {
            for id in self.candidates(sense, tags) {
                set.insert(id.as_str().to_string());
            }
        }
        set
    }
}
