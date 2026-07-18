use crate::db::Db;
use crate::llm::{DerivationRequest, LlmCharacterDerivation, SenseGenerator};
use crate::models::{CharacterDerivation, CharacterId, CreateScene, SceneId, StoryError};
use crate::vocab::Vocab;
use chrono::Utc;
use futures::stream::{self, StreamExt};
use std::sync::Arc;

const MEMORY_LIMIT: i64 = 50;
const CONCURRENCY: usize = 4;

pub struct StoryService {
    db: Db,
    vocab: Vocab,
    generator: Arc<dyn SenseGenerator>,
}

impl StoryService {
    pub fn new(db: Db, vocab: Vocab, generator: Arc<dyn SenseGenerator>) -> Self {
        Self {
            db,
            vocab,
            generator,
        }
    }

    pub fn db(&self) -> &Db {
        &self.db
    }

    pub async fn create_scene(&self, input: CreateScene) -> Result<SceneId, StoryError> {
        let id = SceneId(uuid::Uuid::new_v4());
        self.db
            .scenes()
            .create(
                id,
                &input.objective_event,
                &input.participant_ids,
                input.occurred_at,
            )
            .await?;
        Ok(id)
    }

    pub async fn derive_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<CharacterDerivation, StoryError> {
        let scene = self
            .db
            .scenes()
            .get(scene_id)
            .await?
            .ok_or(StoryError::SceneNotFound(scene_id))?;
        let character = self
            .db
            .characters()
            .get(character_id)
            .await?
            .ok_or(StoryError::CharacterNotFound(character_id))?;
        if !self
            .db
            .scenes()
            .is_participant(scene_id, character_id)
            .await?
        {
            return Err(StoryError::NotSceneParticipant(character_id, scene_id));
        }

        let memories = self.db.memories().list(character_id, MEMORY_LIMIT).await?;
        let last_sensation = self
            .db
            .sensations()
            .latest(character_id)
            .await?
            .map(|(s, _)| s);

        let tags: Vec<&str> = vec![];
        let candidate_set = self.vocab.candidate_set(&tags);
        let candidate_ids = candidate_set.clone();

        let req = DerivationRequest {
            character,
            scene,
            recent_memories: memories,
            last_sensation,
            candidate_ids,
            candidate_tags: vec![],
        };

        let raw: LlmCharacterDerivation = self.generator.derive(&req).await?;
        let sensations = self.validate_with_retry(&req, &raw, &candidate_set).await?;

        let now = Utc::now();
        self.db
            .derivations()
            .insert_derivation(character_id, scene_id, &sensations, &raw.new_memory, now)
            .await?;

        Ok(CharacterDerivation {
            character_id,
            scene_id,
            sensations,
            new_memory: raw.new_memory,
            plot_development: raw.plot_development,
        })
    }

    async fn validate_with_retry(
        &self,
        req: &DerivationRequest,
        raw: &LlmCharacterDerivation,
        candidate_set: &std::collections::HashSet<String>,
    ) -> Result<crate::models::SensorySelection, StoryError> {
        let v1 = crate::vocab::validate(&raw.sensations, candidate_set);
        if !v1.all_empty {
            return Ok(v1.cleaned);
        }
        let raw2 = self.generator.derive(req).await?;
        let v2 = crate::vocab::validate(&raw2.sensations, candidate_set);
        if v2.all_empty {
            return Err(StoryError::InvalidVocabularySelection(
                "all senses empty after retry".into(),
            ));
        }
        Ok(v2.cleaned)
    }

    pub async fn derive_scene(
        &self,
        scene_id: SceneId,
    ) -> Vec<Result<CharacterDerivation, StoryError>> {
        let scene = match self.db.scenes().get(scene_id).await {
            Ok(Some(s)) => s,
            Ok(None) => return vec![Err(StoryError::SceneNotFound(scene_id))],
            Err(e) => return vec![Err(e)],
        };
        let participants = scene.participant_ids.clone();
        stream::iter(participants)
            .map(|cid| {
                let svc = self.clone_refs();
                async move { svc.derive_character(scene_id, cid).await }
            })
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await
    }

    fn clone_refs(&self) -> StoryServiceRef {
        StoryServiceRef {
            db: self.db.clone(),
            vocab: self.vocab.clone(),
            generator: self.generator.clone(),
        }
    }
}

struct StoryServiceRef {
    db: Db,
    vocab: Vocab,
    generator: Arc<dyn SenseGenerator>,
}

impl StoryServiceRef {
    async fn derive_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<CharacterDerivation, StoryError> {
        let svc = StoryService {
            db: self.db.clone(),
            vocab: self.vocab.clone(),
            generator: self.generator.clone(),
        };
        svc.derive_character(scene_id, character_id).await
    }
}
