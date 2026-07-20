use crate::db::Db;
use crate::llm::{DerivationRequest, LlmCharacterDerivation, SenseGenerator, VocabularyCandidate};
use crate::models::{CharacterDerivation, CharacterId, CreateScene, SceneId, StoryError};
use crate::vocab::{SENSES, Vocab};
use chrono::Utc;
use futures::stream::{self, StreamExt};
use std::sync::Arc;

/// 历史记忆上限：传给 LLM 的最近记忆条数
const MEMORY_LIMIT: i64 = 50;
/// 并发推导上限：derive_scene 同时推导的角色数
const CONCURRENCY: usize = 4;

/// 故事服务
/// ==========
/// 项目对外的主要业务接口。提供：
///   - 场景创建
///   - 单角色推导
///   - 全场景推导（并发处理所有参与角色）
///
/// Clone 是廉价的（内部场都是 Arc/Clone 类型）。
#[derive(Clone)]
pub struct StoryService {
    db: Db,                             // 数据库句柄
    vocab: Vocab,                       // 感官词库（用于候选集校验）
    generator: Arc<dyn SenseGenerator>, // LLM 推导引擎（生产用 rig / 测试用 mock）
}

impl StoryService {
    pub fn new(db: Db, vocab: Vocab, generator: Arc<dyn SenseGenerator>) -> Self {
        Self {
            db,
            vocab,
            generator,
        }
    }

    /// 获取数据库引用（供外部直接操作 Repo 用）
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// 创建一个新场景
    ///
    /// # 参数
    /// - `input` — 场景创建请求（客观事件 + 参与者 + 时间）
    ///
    /// # 返回
    /// - `Ok(SceneId)` — 新生成的场景 UUID
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

    /// 对单个角色进行感官/记忆/剧情推导
    ///
    /// # 流程
    /// 1. 校验场景和角色存在性、角色是否参与场景
    /// 2. 加载角色最近的记忆（上限 MEMORY_LIMIT=50）
    /// 3. 加载该角色上一次的感官选择（用于连续性）
    /// 4. 从词库生成候选集
    /// 5. 构造 DerivationRequest 调用 LLM
    /// 6. 校验 LLM 输出（过滤非法词汇，失败时重试一次）
    /// 7. 原子写入感官 + 新记忆
    ///
    /// # 错误
    /// - `SceneNotFound` — 场景不存在
    /// - `CharacterNotFound` — 角色不存在
    /// - `NotSceneParticipant` — 角色未参与该场景
    /// - `InvalidVocabularySelection` — LLM 两次返回都选了非法词汇
    /// - `Llm(...)` — LLM 调用失败
    pub async fn derive_character(
        &self,
        scene_id: SceneId,
        character_id: CharacterId,
    ) -> Result<CharacterDerivation, StoryError> {
        // 1. 加载场景和角色，校验存在性和参与关系
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

        // 2. 加载上下文：记忆 + 上次感官 + 候选词汇
        let memories = self.db.memories().list(character_id, MEMORY_LIMIT).await?;
        let last_sensation = self
            .db
            .sensations()
            .latest(character_id)
            .await?
            .map(|(s, _)| s);

        let tags: Vec<&str> = vec![];
        let candidate_set = self.vocab.candidate_set(&tags);
        let candidates = SENSES
            .iter()
            .flat_map(|sense| {
                self.vocab
                    .entries(sense)
                    .into_iter()
                    .flat_map(move |entries| {
                        entries.iter().map(move |(key, entry)| VocabularyCandidate {
                            id: format!("{sense}.{key}"),
                            sense: (*sense).to_string(),
                            text: entry.text.clone(),
                            tags: entry.tags.clone(),
                        })
                    })
            })
            .collect();

        // 3. 构造请求并调用 LLM
        let req = DerivationRequest {
            character,
            scene,
            recent_memories: memories,
            last_sensation,
            candidates,
            prior_plot_developments: vec![],
        };

        let raw: LlmCharacterDerivation = self.generator.derive(&req).await?;

        // 4. 校验 LLM 输出的词汇是否在候选集中
        //    如果全部为空（LLM 选的词全非法），自动重试一次
        let (raw, sensations) = self.validate_with_retry(&req, raw, &candidate_set).await?;

        // 5. 原子持久化
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

    /// 校验 LLM 感官输出，全部非法时自动重试
    ///
    /// # 重试策略
    /// - 第一次调用 v1：校验 → 存在合法词汇 → 直接返回清理后结果
    /// - 第一次调用 v1：校验 → 全部非法 → 重新调用 LLM
    /// - 第二次调用 v2：校验 → 还是全部非法 → 返回错误
    ///
    /// 这种设计平衡了 LLM "偶尔跑偏"的概率和重试成本。
    async fn validate_with_retry(
        &self,
        req: &DerivationRequest,
        raw: LlmCharacterDerivation,
        candidate_set: &std::collections::HashSet<String>,
    ) -> Result<(LlmCharacterDerivation, crate::models::SensorySelection), StoryError> {
        let v1 = crate::vocab::validate(&raw.sensations, candidate_set);
        if !v1.all_empty {
            return Ok((raw, v1.cleaned));
        }
        let raw2 = self.generator.derive(req).await?;
        let v2 = crate::vocab::validate(&raw2.sensations, candidate_set);
        if v2.all_empty {
            return Err(StoryError::InvalidVocabularySelection(
                "all senses empty after retry".into(),
            ));
        }
        Ok((raw2, v2.cleaned))
    }

    /// 批量推导：场景内所有角色并行处理
    ///
    /// # 行为
    /// - 并发数上限 CONCURRENCY=4
    /// - 使用 futures::stream::buffer_unordered 实现
    /// - 返回 Vec<Result<...>>：每个角色独立成功/失败
    ///   不会因为一个角色失败而阻塞其他角色
    ///
    /// # 错误处理
    /// - 场景不存在 → 返回单元素 Vec [Err]
    /// - 部分角色失败 → 返回混合结果（Ok + Err 混合）
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
                let svc = self.clone();
                async move { svc.derive_character(scene_id, cid).await }
            })
            .buffer_unordered(CONCURRENCY)
            .collect()
            .await
    }
}
