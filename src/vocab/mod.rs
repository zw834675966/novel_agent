// 感官词库模块
// ==============
// 职责：
//   1. 从 YAML 文件加载感官词汇（Vocab）
//   2. 根据标签生成语义候选集（candidates_for_tags）
//   3. 校验 LLM 输出是否在候选集中（validate_selection）
//
// 为什么需要词库？
//   直接让 LLM 自由生成感官描述会导致：
//     - 风格不一致（不同场景用词风格跳变）
//     - 自由度太大难以控制叙事氛围
//   通过预定义的词汇表 + 强制校验，确保 LLM 输出的感官词在可控范围内。

mod loader; // YAML 加载 + VocabularyId 候选集生成
mod validate; // LLM 输出校验（过滤非法词汇，重试检测）

pub use loader::{Vocab, VocabEntry, VocabFile};
pub use validate::{ValidationResult, validate_selection as validate};
