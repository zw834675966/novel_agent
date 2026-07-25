// 叙事生成模块
// ==============
// 职责:把 derive_scene 的结构化结果编排成小说正文。
//   - LLM 只写叙事骨架(action)
//   - 描写一律通过 VocabularyId 从素材库拉取原著原文
//   - 程序校验 ref 必须在该角色候选集内,造词剥离
//
// 与 llm 模块对称:ProseGenerator trait + rig/mock 双实现。
//
// 公共 API:LlmNarrative / NarrativeBeat / NarrateRequest /
//   VocabularyCandidate / CharacterProseCandidates / ProseGenerator /
//   MockProseGenerator / AssembledProse。
// crate 私有:AssembledProse::assemble、
//   candidate_refs_for。

mod assembly; // 按 ID 拉取片段 + 按类别拼装正文 + ref 校验
mod contract; // LlmNarrative / NarrativeBeat(JsonSchema)
mod generator; // ProseGenerator trait + NarrateRequest + 候选类型
mod mock; // 测试用 mock
mod rig_impl; // rig(DeepSeek)生产实现

pub use crate::models::VocabularyCandidate;
pub use assembly::{AssembledProse, MIN_QUOTE_DENSITY, ProseQualityReport};
pub use contract::{LlmNarrative, NarrativeBeat};
pub use generator::{CharacterProseCandidates, NarrateRequest, ProseGenerator};
pub use mock::MockProseGenerator;
pub use rig_impl::RigProseGenerator;

// crate 私有重导出:供 StoryService 跨模块调用,不对外暴露
#[allow(unused_imports)]
pub(crate) use assembly::candidate_refs_for;
