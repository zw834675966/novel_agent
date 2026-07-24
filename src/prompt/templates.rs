// 提示词模板构建与规范字典排序实现
// ===================================

use crate::llm::VocabularyCandidate;
use std::collections::BTreeMap;

/// 角色推导提示词输入参数
#[derive(Debug, Clone)]
pub struct DerivationPromptInput<'a> {
    pub character_name: &'a str,
    pub character_tags: &'a [String],
    pub character_skills: &'a [String],
    pub scene_objective_event: &'a str,
    pub memories: &'a [String],
    pub candidates_by_sense: &'a BTreeMap<String, Vec<VocabularyCandidate>>,
}

/// 叙事正文提示词输入参数
#[derive(Debug, Clone)]
pub struct NarrationPromptInput<'a> {
    pub scene_objective_event: &'a str,
    pub character_actions: &'a [(String, String)], // (角色名, 动作文本)
    pub plot_reasons: &'a [(String, String)],      // (角色名, 原因渲染)
    pub selected_quotes: &'a [String],             // 注入的感官/描写原著短句
}

/// 获取固定、不可变的系统角色与硬性约束 Prompt (DeepSeek System Role)
pub fn build_system_prompt() -> String {
    r#"你是专注于小说角色感知与叙事编排的专业 AI 引擎。

【硬性行为准则】
1. 严禁输出任何开放抒情或因果说明文套话（禁止使用“由于”、“因此”、“从而”、“因为”等过渡词）。
2. 所有选取的感官词汇必须严格来自提供的【蒸馏候选词库】，严禁发明不在词库中的 ID。
3. 剧情原因必须严格填入指定格式的短事实槽位（PlotReasonSlot），字符限长 20 字以内。
4. 动作必须使用舞台剧提示格式（StructuredAction）。
5. 必须且只能输出严格符合 JSON Schema 的 JSON 对象，禁止包含任何 Markdown 标记之外的解释性文字。"#
        .to_string()
}

/// 自动构建结构化、规范排序的蒸馏素材推导 User Prompt (DeepSeek User Role)
pub fn build_derivation_prompt(input: &DerivationPromptInput<'_>) -> String {
    let mut prompt = String::new();

    // 1. 角色与场景基础上下文
    prompt.push_str(&format!("【目标角色】: {}\n", input.character_name));
    if !input.character_tags.is_empty() {
        prompt.push_str(&format!(
            "【角色性格/身份标签】: {}\n",
            input.character_tags.join(", ")
        ));
    }
    if !input.character_skills.is_empty() {
        prompt.push_str(&format!(
            "【角色技能】: {}\n",
            input.character_skills.join(", ")
        ));
    }
    prompt.push_str(&format!(
        "【当前场景客观事件】: {}\n\n",
        input.scene_objective_event
    ));

    // 2. 前情历史记忆 ($T < T_{current}$)
    if !input.memories.is_empty() {
        prompt.push_str("【角色近期历史记忆】:\n");
        for (idx, mem) in input.memories.iter().enumerate() {
            prompt.push_str(&format!("{}. {}\n", idx + 1, mem));
        }
        prompt.push('\n');
    }

    // 3. 规范字典序排序与上限控制的蒸馏素材投喂 (BTreeMap 确保确定性)
    prompt.push_str("【蒸馏感官与情感候选词库 (Candidate Vocabulary)】:\n");
    for (sense, candidates) in input.candidates_by_sense {
        prompt.push_str(&format!("[感官类别: {}]\n", sense));
        for cand in candidates {
            prompt.push_str(&format!(
                "  - ID: \"{}\" | 文本: \"{}\" | 标签: [{}]\n",
                cand.id,
                cand.text,
                cand.tags.join(", ")
            ));
        }
    }

    // 4. JSON 输出契约要求
    prompt.push_str(
        r#"
【输出 JSON 契约要求】:
请严格返回如下 JSON 结构，不允许添加任何多余文字：
{
  "sensory_analysis": "先分析此场景的感官焦点（中文，50字以内），再选择 ID",
  "sensations": {
    "visual_ids": ["..."],
    "auditory_ids": ["..."],
    "olfactory_ids": ["..."],
    "tactile_ids": ["..."],
    "gustatory_ids": ["..."],
    "emotion_ids": ["..."],
    "gesture_ids": ["..."],
    "atmosphere_ids": ["..."]
  },
  "new_memory": {
    "content": "简洁事实记忆，30字以内",
    "source": "witnessed",
    "certainty": "certain"
  },
  "plot_development": [
    {
      "kind": "conflict_escalated",
      "reason": {
        "kind": "observed",
        "detail": "短事实原因，15字以内"
      }
    }
  ],
  "relationship_candidates": []
}
"#,
    );

    prompt
}

/// 自动构建叙事正文拼装 User Prompt
pub fn build_narration_prompt(input: &NarrationPromptInput<'_>) -> String {
    let mut prompt = String::new();

    prompt.push_str(&format!(
        "【场景事件】: {}\n\n",
        input.scene_objective_event
    ));

    if !input.character_actions.is_empty() {
        prompt.push_str("【角色动作列表】:\n");
        for (char_name, action) in input.character_actions {
            prompt.push_str(&format!("- {}: {}\n", char_name, action));
        }
        prompt.push('\n');
    }

    if !input.plot_reasons.is_empty() {
        prompt.push_str("【剧情转折原因槽】:\n");
        for (char_name, reason) in input.plot_reasons {
            prompt.push_str(&format!("- {}: {}\n", char_name, reason));
        }
        prompt.push('\n');
    }

    if !input.selected_quotes.is_empty() {
        prompt.push_str("【必须植入的原著描写词句库】:\n");
        for quote in input.selected_quotes {
            prompt.push_str(&format!("- \"{}\"\n", quote));
        }
        prompt.push('\n');
    }

    prompt.push_str("请结合以上硬性词句与动作提示，组装为高密度、无 AI 套话的小说正文段落。\n");
    prompt
}
