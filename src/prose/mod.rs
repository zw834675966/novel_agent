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
//   ProseCandidate / CharacterProseCandidates / ProseGenerator /
//   MockProseGenerator / AssembledProse。
// crate 私有:assemble(以 AssembledProse::assemble 关联函数暴露)、
//   candidate_refs_for、build_candidate_refs、participant_set。

mod assembly; // 按 ID 拉取片段 + 按类别拼装正文 + ref 校验
mod contract; // LlmNarrative / NarrativeBeat(JsonSchema)
mod generator; // ProseGenerator trait + NarrateRequest + 候选类型
mod mock; // 测试用 mock
mod rig_impl; // rig(DeepSeek)生产实现

pub use assembly::AssembledProse;
pub use contract::{LlmNarrative, NarrativeBeat};
pub use generator::{CharacterProseCandidates, NarrateRequest, ProseCandidate, ProseGenerator};
pub use mock::MockProseGenerator;
#[allow(unused_imports)]
pub(crate) use rig_impl::RigProseGenerator;

// crate 私有重导出:供 StoryService / rig_impl 跨模块调用,不对外暴露
#[allow(unused_imports)]
pub(crate) use assembly::candidate_refs_for;
#[allow(unused_imports)]
pub(crate) use rig_impl::{build_candidate_refs, participant_set};
