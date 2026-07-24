use crate::db::Db;
use crate::llm::{ContextTagRequest, DerivationRequest, LlmCharacterDerivation, SenseGenerator};
use crate::models::{CharacterDerivation, CharacterId, CreateScene, SceneId, StoryError};
use crate::prose::{
    AssembledProse, CharacterProseCandidates, NarrateRequest, ProseGenerator, VocabularyCandidate,
};
use crate::text_guard::{MAX_MEMORY_CHARS, MAX_PLOT_REASON_CHARS, sanitize_free_text};
use crate::vocab::Vocab;
use chrono::Utc;
use futures::stream::{self, StreamExt};
use std::collections::HashSet;
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
///   - 场景叙事编排（结构化推导 -> 小说正文）
///
/// Clone 是廉价的（内部场都是 Arc/Clone 类型）。
#[derive(Clone)]
pub struct StoryService {
    db: Db,                                   // 数据库句柄
    vocab: Vocab,                             // 感官词库（候选集校验 + 叙事拼装）
    sense_generator: Arc<dyn SenseGenerator>, // 感官/记忆 LLM 推导引擎
    prose_generator: Arc<dyn ProseGenerator>, // 叙事编排 LLM 引擎
}

impl StoryService {
    pub fn new(
        db: Db,
        vocab: Vocab,
        sense_generator: Arc<dyn SenseGenerator>,
        prose_generator: Arc<dyn ProseGenerator>,
    ) -> Self {
        Self {
            db,
            vocab,
            sense_generator,
            prose_generator,
        }
    }

    /// 获取数据库引用（供外部直接操作 Repo 用）
    pub fn db(&self) -> &Db {
        &self.db
    }

    /// 创建一个新场景
    ///
    /// # 参数
    /// - `input` - 场景创建请求（客观事件 + 参与者 + 时间）
    ///
    /// # 返回
    /// - `Ok(SceneId)` - 新生成的场景 UUID
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
    /// - `SceneNotFound` - 场景不存在
    /// - `CharacterNotFound` - 角色不存在
    /// - `NotSceneParticipant` - 角色未参与该场景
    /// - `InvalidVocabularySelection` - LLM 两次返回都选了非法词汇
    /// - `Llm(...)` - LLM 调用失败
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

        // 2. 只读取当前场景之前发生的上下文，避免未来事件泄漏进推导。
        let memories = self
            .db
            .memories()
            .list_before_scene(character_id, scene.occurred_at, MEMORY_LIMIT)
            .await?;
        let last_sensation = self
            .db
            .sensations()
            .latest_before_scene(character_id, scene.occurred_at)
            .await?
            .map(|(s, _)| s);
        let prior_plot_developments = self
            .db
            .plots()
            .list_before_scene(character_id, scene.occurred_at)
            .await?;
        // Build query early so tag shortlist is relevance-ranked (not pure alpha truncate).
        let mut query_terms: Vec<String> = Vec::new();
        if !character.name.trim().is_empty() {
            query_terms.push(character.name.clone());
        }
        for part in scene
            .objective_event
            .split(|c: char| c.is_whitespace() || "，。！？、；：,.!?;:\"'《》【】".contains(c))
        {
            let t = part.trim();
            if t.chars().count() >= 2 {
                query_terms.push(t.to_string());
            }
        }
        let raw_tags = self
            .sense_generator
            .select_context_tags(&ContextTagRequest {
                character: character.clone(),
                scene: scene.clone(),
                prior_plot_developments: prior_plot_developments.clone(),
                available_tags: self
                    .vocab
                    .known_tags_ranked_limited(&query_terms, crate::vocab::DEFAULT_TAG_CAP),
            })
            .await?;
        let selected_tags = self.vocab.filter_known_tags(&raw_tags.tags);
        // Lexical ranking (RELiC-style select): name + scene tokens + selected tags.
        query_terms.extend(selected_tags.iter().cloned());
        let candidates = self.vocab.candidates_ranked_limited(
            &selected_tags,
            &query_terms,
            crate::vocab::DEFAULT_PER_SENSE_CAP,
            crate::vocab::DEFAULT_TOTAL_CAP,
        );
        let candidate_set = candidates
            .iter()
            .map(|candidate| candidate.id.clone())
            .collect();

        // 3. 构造请求并调用 LLM
        let req = DerivationRequest {
            character,
            scene,
            recent_memories: memories,
            last_sensation,
            candidates,
            prior_plot_developments,
        };

        let raw: LlmCharacterDerivation = self.sense_generator.derive(&req).await?;

        // 4. 校验 LLM 输出的词汇是否在候选集中
        //    如果全部为空（LLM 选的词全非法），自动重试一次
        let (mut raw, sensations) = self.validate_with_retry(&req, raw, &candidate_set).await?;

        // 4b. 自由文本护栏：记忆与情节 reason 剥因果套话并限长（批判 P1）
        sanitize_derivation_free_text(&mut raw);

        // 5. 原子替换当前场景已有结果，避免重复推导留下过期状态。
        let now = Utc::now();
        self.db
            .derivations()
            .replace_derivation(
                character_id,
                scene_id,
                &sensations,
                &raw.new_memory,
                &raw.plot_development,
                &selected_tags,
                now,
            )
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
    /// - 第一次调用 v1：校验 -> 存在合法词汇 -> 直接返回清理后结果
    /// - 第一次调用 v1：校验 -> 全部非法 -> 重新调用 LLM
    /// - 第二次调用 v2：校验 -> 还是全部非法 -> 返回错误
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
        let raw2 = self.sense_generator.derive(req).await?;
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
    /// - 场景不存在 -> 返回单元素 Vec [Err]
    /// - 部分角色失败 -> 返回混合结果（Ok + Err 混合）
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

    /// 把场景的结构化推导结果编排成小说正文
    ///
    /// # 流程
    /// 1. 取场景客观事件与参与者
    /// 2. 校验所有 derivation:scene_id 匹配、角色为参与者、无重复角色
    /// 3. 加载全部参与者 Character
    /// 4. 从 service-owned Vocab 构造每角色的候选片段元数据(只含当前可解析的 ID)
    /// 5. 排序 characters / derivations / 候选组 / 候选 / tags 后构造 NarrateRequest
    /// 6. 调用 service-owned ProseGenerator 产出 LlmNarrative
    /// 7. 调用内部 assembly 拼装正文 + 校验 ref
    ///
    /// # 防AI化
    /// LLM 只写 action(叙事骨架),描写一律通过 ref 拉取原文;
    /// 非 pov 参与者的 beat 被拒,非法 ref 被剥离。
    ///
    /// # 错误
    /// - `SceneNotFound` - 场景不存在
    /// - `InvalidNarrationContext` - derivation scene_id 不匹配 / 非参与者角色 / 重复角色
    /// - `CharacterNotFound` - 参与者角色不存在
    /// - `Llm(...)` - LLM 调用失败或返回空 narrative
    pub async fn narrate_scene(
        &self,
        scene_id: SceneId,
        derivations: &[CharacterDerivation],
    ) -> Result<AssembledProse, StoryError> {
        // 1. 加载场景
        let scene = self
            .db
            .scenes()
            .get(scene_id)
            .await?
            .ok_or(StoryError::SceneNotFound(scene_id))?;

        // 2. 校验 derivation 上下文
        //    - 每个 derivation 的 scene_id 必须等于请求的 scene_id
        //    - 每个 derivation 的角色必须是场景参与者
        //    - 不允许重复角色的 derivation
        let participant_set: HashSet<CharacterId> = scene.participant_ids.iter().copied().collect();
        let mut seen: HashSet<CharacterId> = HashSet::new();
        for d in derivations {
            if d.scene_id != scene_id {
                return Err(StoryError::InvalidNarrationContext(format!(
                    "derivation for character {:?} belongs to scene {:?}, not {:?}",
                    d.character_id, d.scene_id, scene_id
                )));
            }
            if !participant_set.contains(&d.character_id) {
                return Err(StoryError::InvalidNarrationContext(format!(
                    "character {:?} is not a participant of scene {:?}",
                    d.character_id, scene_id
                )));
            }
            if !seen.insert(d.character_id) {
                return Err(StoryError::InvalidNarrationContext(format!(
                    "duplicate derivation for character {:?}",
                    d.character_id
                )));
            }
        }

        // 3. 加载全部参与者 Character(必须全部存在,否则 CharacterNotFound)
        let mut characters: Vec<crate::models::Character> =
            Vec::with_capacity(scene.participant_ids.len());
        for cid in &scene.participant_ids {
            let c = self
                .db
                .characters()
                .get(*cid)
                .await?
                .ok_or(StoryError::CharacterNotFound(*cid))?;
            characters.push(c);
        }

        // 4. 从 service-owned Vocab 构造候选元数据
        //    只保留(a)出现在 derivation 候选集中 (b)当前 Vocab 能解析 的 ID。
        let candidates = build_candidate_refs(derivations, &self.vocab);

        // 5. 排序保证 prompt 稳定:
        //    - characters 按 CharacterId(Uuid) 升序
        //    - derivations 按 character_id 升序
        //    - candidate groups 按 character_id 升序
        //    - 组内候选按 SENSES 顺序 + id 升序
        //    - tags 字典序
        characters.sort_by_key(|c| c.id.0);
        let mut sorted_derivations = derivations.to_vec();
        sorted_derivations.sort_by_key(|d| d.character_id.0);
        let mut sorted_candidates = candidates;
        sorted_candidates.sort_by_key(|g| g.character_id.0);
        for group in &mut sorted_candidates {
            group.candidates.sort_by(|a, b| {
                let sa = crate::vocab::sense_order(&a.sense);
                let sb = crate::vocab::sense_order(&b.sense);
                sa.cmp(&sb).then_with(|| a.id.cmp(&b.id))
            });
            for cand in &mut group.candidates {
                cand.tags.sort();
            }
        }

        // 6. 构造语义请求并调用 LLM
        let req = NarrateRequest {
            scene: scene.clone(),
            characters,
            derivations: sorted_derivations,
            candidates: sorted_candidates,
        };
        let narrative = self.prose_generator.narrate(&req).await?;

        // 7. 拼装正文 + ref 校验
        let participants: HashSet<String> = scene
            .participant_ids
            .iter()
            .map(|id| id.0.to_string())
            .collect();
        AssembledProse::assemble(&narrative, &self.vocab, &req.derivations, &participants)
    }
}

/// Clamp memory content and plot reasons after LLM derive (critique P1 multi-layer leak).
fn sanitize_derivation_free_text(raw: &mut LlmCharacterDerivation) {
    raw.new_memory.content = sanitize_free_text(&raw.new_memory.content, MAX_MEMORY_CHARS);
    for plot in &mut raw.plot_development {
        plot.reason = sanitize_free_text(&plot.reason, MAX_PLOT_REASON_CHARS);
    }
}

/// 从 derivations 构造每角色的语义候选引用(供 NarrateRequest 使用)
///
/// 需要词库以解析每个 VocabularyId 的 text/tags/sense。
/// 只保留当前 Vocab 能解析的 ID。
fn build_candidate_refs(
    derivations: &[CharacterDerivation],
    vocab: &Vocab,
) -> Vec<CharacterProseCandidates> {
    derivations
        .iter()
        .map(|d| {
            let ids = crate::prose::candidate_refs_for(d);
            let candidates = ids
                .iter()
                .filter_map(|raw| {
                    let vid = crate::models::VocabularyId::new(raw).ok()?;
                    let entry = vocab.entries(vid.sense())?.get(vid.key())?;
                    Some(VocabularyCandidate {
                        id: raw.to_string(),
                        sense: vid.sense().to_string(),
                        text: entry.text.clone(),
                        tags: entry.tags.clone(),
                    })
                })
                .collect::<Vec<_>>();
            CharacterProseCandidates {
                character_id: d.character_id,
                candidates,
            }
        })
        .collect()
}

#[cfg(test)]
mod free_text_guard_tests {
    use super::sanitize_derivation_free_text;
    use crate::llm::LlmCharacterDerivation;
    use crate::models::{
        Certainty, CharacterMemoryDraft, MemorySource, PlotDevelopment, PlotDevelopmentKind,
        SensorySelection,
    };

    #[test]
    fn sanitizes_memory_and_plot_reason() {
        let mut raw = LlmCharacterDerivation {
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: "因此他想起了不禁".into(),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![PlotDevelopment {
                kind: PlotDevelopmentKind::NewClue,
                reason: "于是发现线索".into(),
            }],
        };
        sanitize_derivation_free_text(&mut raw);
        assert!(!raw.new_memory.content.contains("因此"));
        assert!(!raw.new_memory.content.contains("不禁"));
        assert!(!raw.plot_development[0].reason.contains("于是"));
    }
}
