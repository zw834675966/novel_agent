use crate::db::Db;
use crate::db::{CandidateResolution, PendingCandidate};
use crate::llm::{
    ContextTagRequest, DerivationRequest, LlmCharacterDerivation, LlmRelationshipCandidate,
    SenseGenerator,
};
use crate::models::{
    CharacterDerivation, CharacterId, CreateScene, RelationshipCandidate, RelationshipCandidateId,
    SceneId, StoryError,
};
use crate::prose::{
    AssembledProse, CharacterProseCandidates, NarrateRequest, PlanRequest, ProseCandidate,
    ProseGenerator, ScenePlanner,
};
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
    scene_planner: Arc<dyn ScenePlanner>,     // 场景编导 LLM 引擎（大纲+镜头表）
}

impl StoryService {
    pub fn new(
        db: Db,
        vocab: Vocab,
        sense_generator: Arc<dyn SenseGenerator>,
        prose_generator: Arc<dyn ProseGenerator>,
        scene_planner: Arc<dyn ScenePlanner>,
    ) -> Self {
        Self {
            db,
            vocab,
            sense_generator,
            prose_generator,
            scene_planner,
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

        // 1.5. N5 长线物理状态逻辑断言拦截：若角色处于 Dead (死亡) 或 Absent (不在场) 状态，拒绝推导
        if let Some(state) = self.db.states().get_state(character_id).await? {
            match state {
                crate::models::CharacterState::Dead => {
                    return Err(StoryError::InvalidParticipant(format!(
                        "角色 {} 已处于死亡 (Dead) 状态，不能参与场景推导",
                        character.name
                    )));
                }
                crate::models::CharacterState::Absent => {
                    return Err(StoryError::InvalidParticipant(format!(
                        "角色 {} 已处于不在场 (Absent) 状态，不能参与场景推导",
                        character.name
                    )));
                }
                _ => {}
            }
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

        // 加载场景全部参与者 Character（供关系候选参照）
        let mut scene_participants: Vec<crate::models::Character> =
            Vec::with_capacity(scene.participant_ids.len());
        for pid in &scene.participant_ids {
            let c = self
                .db
                .characters()
                .get(*pid)
                .await?
                .ok_or(StoryError::CharacterNotFound(*pid))?;
            scene_participants.push(c);
        }

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
        let candidates = self.vocab.candidates_ranked_limited_with_quotas(
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
            character: character.clone(),
            scene: scene.clone(),
            recent_memories: memories,
            last_sensation,
            candidates,
            prior_plot_developments,
            scene_participants: scene_participants.clone(),
        };

        let raw: LlmCharacterDerivation = self.sense_generator.derive(&req).await?;

        // 4. 校验 LLM 输出的词汇是否在候选集中
        //    如果全部为空（LLM 选的词全非法），自动重试一次
        let (mut raw, sensations) = self.validate_with_retry(&req, raw, &candidate_set).await?;

        // 4b. 自由文本护栏：记忆与情节 reason 剥因果套话并限长（批判 P1）
        sanitize_derivation_free_text(&mut raw);

        // 5. 原子替换当前场景已有结果，避免重复推导留下过期状态。
        let now = Utc::now();
        let new_memory = self
            .db
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

        // 6. 校验并持久化关系候选（只保留目标为场景参与者的有效候选）
        let participant_ids: HashSet<CharacterId> =
            scene_participants.iter().map(|c| c.id).collect();
        let mut relationship_candidates: Vec<RelationshipCandidate> = Vec::new();
        for llm_cand in &raw.relationship_candidates {
            if let Some(valid) = validate_and_build_pending_candidate(
                character_id,
                llm_cand,
                scene_id,
                &participant_ids,
                Some(new_memory.id),
            ) {
                let stored = self.db.relationships().insert_pending(valid).await?;
                relationship_candidates.push(stored);
            }
        }

        Ok(CharacterDerivation {
            character_id,
            scene_id,
            sensations,
            new_memory,
            plot_development: raw.plot_development,
            relationship_candidates,
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
    /// 5. 排序 characters / derivations / 候选组 / 候选 / tags
    /// 6. 调用 service-owned ScenePlanner 生成编导大纲+镜头表并验证(camera_beats 非空、pov_name 匹配参与者)
    /// 7. 构造 NarrateRequest 调用 service-owned ProseGenerator 产出 LlmNarrative
    /// 8. 调用内部 assembly 拼装正文 + 校验 ref
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
                let sa = crate::vocab::SENSES
                    .iter()
                    .position(|s| *s == a.sense)
                    .unwrap_or(usize::MAX);
                let sb = crate::vocab::SENSES
                    .iter()
                    .position(|s| *s == b.sense)
                    .unwrap_or(usize::MAX);
                sa.cmp(&sb).then_with(|| a.id.cmp(&b.id))
            });
            for cand in &mut group.candidates {
                cand.tags.sort();
            }
        }

        // 6. 调用 ScenePlanner 生成编导大纲+镜头表
        //    复用已排序的 characters（按 CharacterId 升序）保证 prompt 稳定。
        let plan = self
            .scene_planner
            .plan_scene(&PlanRequest {
                scene: scene.clone(),
                characters: characters.clone(),
            })
            .await?;

        // 6.1. 验证计划：camera_beats 非空；每个 pov_name 匹配某个角色名
        if plan.scene_card.camera_beats.is_empty() {
            return Err(StoryError::Llm(
                "scene planner returned no camera beats".into(),
            ));
        }
        let valid_names: HashSet<&str> = characters.iter().map(|c| c.name.as_str()).collect();
        for beat in &plan.scene_card.camera_beats {
            if !valid_names.contains(beat.pov_name.as_str()) {
                return Err(StoryError::Llm(format!(
                    "camera beat pov_name '{}' not found among scene participants",
                    beat.pov_name
                )));
            }
        }

        let req = NarrateRequest {
            scene: scene.clone(),
            characters,
            derivations: sorted_derivations,
            candidates: sorted_candidates,
            plan,
        };
        let narrative = self.prose_generator.narrate(&req).await?;

        // 7. 拼装正文 + ref 校验
        let participants: HashSet<String> = scene
            .participant_ids
            .iter()
            .map(|id| id.0.to_string())
            .collect();
        // 7b. 降级感官兜底池：必须取自「全量 ranked 候选集」(五感主类)，
        //     绝不能取自 LLM 已选的 derivation 感官 ID。当 degraded_sensory_density
        //     为真（五感零选中）时，已选 ID 池里根本没有五感 ID，`assemble` 的注入
        //     路径会沦为死代码。query_terms 复用 derive_character 对 objective_event
        //     的分词；tags 留空即可（quota 检索仍会给弱感官保底）。生产环境兜底池必须
        //     来自 ranked 候选而非 selected IDs——见 tests/prose_test.rs 注释。
        let mut fallback_query_terms: Vec<String> = Vec::new();
        for part in scene
            .objective_event
            .split(|c: char| c.is_whitespace() || "，。！？、；：,.!?;:\"'《》【】".contains(c))
        {
            let t = part.trim();
            if t.chars().count() >= 2 {
                fallback_query_terms.push(t.to_string());
            }
        }
        let fallback_pool: HashSet<String> = self
            .vocab
            .candidates_ranked_limited_with_quotas(
                &[],
                &fallback_query_terms,
                crate::vocab::DEFAULT_PER_SENSE_CAP,
                crate::vocab::DEFAULT_TOTAL_CAP,
            )
            .into_iter()
            .filter(|c| {
                matches!(
                    c.sense.as_str(),
                    "visual" | "auditory" | "olfactory" | "tactile" | "gustatory"
                )
            })
            .map(|c| c.id)
            .collect();
        AssembledProse::assemble(
            &narrative,
            &self.vocab,
            &req.derivations,
            &participants,
            &fallback_pool,
        )
    }

    /// 解析关系候选（接受或拒绝），委托给 RelationshipRepo
    pub async fn resolve_relationship_candidate(
        &self,
        candidate_id: RelationshipCandidateId,
        resolution: CandidateResolution,
    ) -> Result<(), StoryError> {
        self.db
            .relationships()
            .resolve_candidate(candidate_id, resolution)
            .await
    }

    // ---- 工作台读取 API ----

    /// 列出全部角色（按 name, id 排序）
    pub async fn list_characters(&self) -> Result<Vec<crate::models::Character>, StoryError> {
        self.db.characters().list().await
    }

    /// 列出全部场景（按 occurred_at, id 排序）
    pub async fn list_scenes(&self) -> Result<Vec<crate::models::Scene>, StoryError> {
        self.db.scenes().list().await
    }

    /// 获取单个场景（不存在返回 None）
    pub async fn get_scene(
        &self,
        scene_id: SceneId,
    ) -> Result<Option<crate::models::Scene>, StoryError> {
        self.db.scenes().get(scene_id).await
    }

    /// 获取场景中所有参与者的推导详情（记忆/感官/剧情/候选）
    pub async fn scene_derivations(
        &self,
        scene_id: SceneId,
    ) -> Result<Vec<crate::models::SceneDerivationDetail>, StoryError> {
        let scene = self
            .db
            .scenes()
            .get(scene_id)
            .await?
            .ok_or(StoryError::SceneNotFound(scene_id))?;

        let memories = self.db.memories().list_for_scene(scene_id).await?;
        let sensations = self.db.sensations().list_for_scene(scene_id).await?;
        let plots = self.db.plots().list_for_scene(scene_id).await?;
        let candidates = self
            .db
            .relationships()
            .list_candidates_for_scene(scene_id)
            .await?;

        let mut details = Vec::new();
        for cid in &scene.participant_ids {
            let character = self
                .db
                .characters()
                .get(*cid)
                .await?
                .ok_or(StoryError::CharacterNotFound(*cid))?;

            let memory = memories
                .iter()
                .find(|m| m.character_id == *cid)
                .cloned()
                .ok_or(StoryError::CharacterNotFound(*cid))?;

            let sensation = sensations
                .iter()
                .find(|(c, _)| c == cid)
                .map(|(_, s)| s.clone())
                .unwrap_or_default();

            let char_plots: Vec<_> = plots
                .iter()
                .filter(|p| p.character_id == *cid)
                .cloned()
                .collect();
            let char_candidates: Vec<_> = candidates
                .iter()
                .filter(|c| c.from_character_id == *cid)
                .cloned()
                .collect();

            details.push(crate::models::SceneDerivationDetail {
                character,
                memory,
                sensation,
                plot_developments: char_plots,
                relationship_candidates: char_candidates,
            });
        }
        Ok(details)
    }

    /// 获取通过指定场景的关系图谱快照
    pub async fn story_graph_through(
        &self,
        scene_id: SceneId,
    ) -> Result<crate::models::GraphSnapshot, StoryError> {
        self.db
            .relationships()
            .graph_snapshot_through(scene_id)
            .await
    }

    /// 获取关系事实的修订历史
    pub async fn relationship_history(
        &self,
        fact_id: crate::models::RelationshipFactId,
    ) -> Result<Vec<crate::models::RelationshipRevision>, StoryError> {
        self.db.relationships().history(fact_id).await
    }
}

/// 校验 LLM 关系候选并构建待持久化的 PendingCandidate
///
/// 规则：
///   - 目标角色 != 源角色
///   - 目标角色必须是场景参与者
///   - summary 非空且不超过 200 Unicode 字符
///   - 分数有效（0-100）
///   - confidence 有限且在 [0.0, 1.0] 范围内
///
/// 返回 None 表示候选无效，应被丢弃（不阻塞有效推导）。
fn validate_and_build_pending_candidate(
    source: CharacterId,
    llm_cand: &LlmRelationshipCandidate,
    scene_id: SceneId,
    participant_ids: &HashSet<CharacterId>,
    evidence_memory_id: Option<crate::models::MemoryId>,
) -> Option<PendingCandidate> {
    if llm_cand.target_character_id == source {
        return None;
    }
    if !participant_ids.contains(&llm_cand.target_character_id) {
        return None;
    }
    let summary_trimmed = llm_cand.summary.trim();
    if summary_trimmed.is_empty() || summary_trimmed.chars().count() > 200 {
        return None;
    }
    // 分数校验
    if llm_cand.tension_score.map(|v| v > 100).unwrap_or(false)
        || llm_cand.trust_score.map(|v| v > 100).unwrap_or(false)
        || llm_cand.affection_score.map(|v| v > 100).unwrap_or(false)
        || llm_cand.power_score.map(|v| v > 100).unwrap_or(false)
    {
        return None;
    }
    // confidence 校验
    if !llm_cand.confidence.is_finite() || llm_cand.confidence < 0.0 || llm_cand.confidence > 1.0 {
        return None;
    }

    Some(PendingCandidate {
        scene_id,
        from: source,
        to: llm_cand.target_character_id,
        relationship_type: llm_cand.relationship_type,
        summary: summary_trimmed.to_string(),
        tension_score: llm_cand.tension_score,
        trust_score: llm_cand.trust_score,
        affection_score: llm_cand.affection_score,
        power_score: llm_cand.power_score,
        evidence_memory_id,
        confidence: llm_cand.confidence,
    })
}

/// 推导后自由文本护栏：记忆槽就地清理，保留枚举判别式。
/// PlotReasonSlot 在 `.render()` 时清理；不再 `render→From` 坍缩为 Other。
fn sanitize_derivation_free_text(raw: &mut LlmCharacterDerivation) {
    raw.new_memory.content.sanitize_in_place();
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
                    Some(ProseCandidate {
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
        Certainty, CharacterMemoryDraft, MemoryContentSlot, MemorySource, PlotDevelopment,
        PlotDevelopmentKind, SensorySelection,
    };

    #[test]
    fn sanitizes_memory_preserving_variant() {
        let mut raw = LlmCharacterDerivation {
            sensory_analysis: String::new(),
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: MemoryContentSlot::Dialogue("因此他说了不禁".into()),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![PlotDevelopment {
                kind: PlotDevelopmentKind::NewClue,
                reason: "于是发现线索".into(),
            }],
            relationship_candidates: vec![],
        };
        sanitize_derivation_free_text(&mut raw);
        assert!(matches!(
            &raw.new_memory.content,
            MemoryContentSlot::Dialogue(_)
        ));
        assert!(!raw.new_memory.content.render().contains("因此"));
        assert!(!raw.new_memory.content.render().contains("不禁"));
        // plot reason 仍在 render 时剥套话
        assert!(!raw.plot_development[0].reason.render().contains("于是"));
    }

    #[test]
    fn sanitize_does_not_collapse_observation_to_other() {
        let mut raw = LlmCharacterDerivation {
            sensory_analysis: String::new(),
            sensations: SensorySelection::default(),
            new_memory: CharacterMemoryDraft {
                content: MemoryContentSlot::Observation("见血印于地".into()),
                source: MemorySource::Witnessed,
                certainty: Certainty::Certain,
            },
            plot_development: vec![],
            relationship_candidates: vec![],
        };
        sanitize_derivation_free_text(&mut raw);
        assert_eq!(
            raw.new_memory.content,
            MemoryContentSlot::Observation("见血印于地".into())
        );
    }
}
