//! 提示词工程与模版封装模块
//! ================================
//! 本模块实现将提示词工程与 LLM 推理逻辑解耦。
//! 提供固定的系统角色提示词与确定性构建模板。

pub mod templates;

pub use templates::{
    DerivationPromptInput, NarrationPromptInput, build_derivation_prompt, build_narration_prompt,
    build_system_prompt,
};
