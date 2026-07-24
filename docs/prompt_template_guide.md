# DeepSeek V4 Flash 提示词工程模板与 Rust 封装使用指南

> **设计目标**：将提示词工程与 LLM 推理代码彻底解耦。使用者只需填写结构化 Rust 输入参数，即可自动生成最佳提示词，并在 `temperature=0.0` 贪婪解码与规范字典排序下实现**语义与结果确定性（无输出飘移）**。
> **参考标准**：Google Research (STED & In-Context Learning) 与 DeepSeek V4 Flash 官方提示词工程指南。

---

## 一、 提示词架构拆分 (Three-Tier Prompt Architecture)

提示词划分为三个独立模块：

```
+-------------------------------------------------------------------+
| 1. System Prompt (系统角色与硬性法则)                             |
|    - 强行禁止 "由于/因此" 等 AI 说明文套话                         |
|    - 强制限制只能使用 [候选词库] 中的 ID                             |
|    - 强制使用 PlotReasonSlot 相邻 JSON 与 StructuredAction 格式        |
+-------------------------------------------------------------------+
| 2. Context Schema (上下文与蒸馏素材投喂)                           |
|    - 场景客观事件 (Objective Event)                              |
|    - 角色属性与前情记忆 ($T < T_{current}$)                        |
|    - 规范排序与上限封顶的【蒸馏感官/情感候选词库】 (Caps: 80 tags/96 candidates) |
+-------------------------------------------------------------------+
| 3. Output JSON Schema (输出强类型 JSON 契约)                       |
|    - 严格 JSON 语法与规范键名                                     |
+-------------------------------------------------------------------+
```

---

## 二、 Rust 函数 API 接口说明

`novels::prompt` 模块导出以下核心函数：

### 1. `build_system_prompt() -> String`
返回固定、不可变的系统角色与硬性规约 Prompt，用于 `deepseek-v4-flash` 的 `system` 角色设定。

### 2. `build_derivation_prompt(input: &DerivationPromptInput) -> String`
根据输入的角色、场景、历史记忆与过滤后的词库候选集，自动生成格式最优的 `user` 推导提示词。

#### 输入参数结构体 `DerivationPromptInput`：
```rust
pub struct DerivationPromptInput<'a> {
    pub character_name: &'a str,
    pub character_tags: &'a [String],
    pub character_skills: &'a [String],
    pub scene_objective_event: &'a str,
    pub memories: &'a [String],
    pub candidates_by_sense: &'a std::collections::BTreeMap<String, Vec<VocabularyCandidate>>,
}
```

### 3. `build_narration_prompt(input: &NarrationPromptInput) -> String`
将推导出的结构化感官、剧情原因槽（`PlotReasonSlot`）与动作段落结合，自动拼装生成叙事正文提示词。

---

## 三、 语义与输出一致性保障 (Deterministic Output Rules)

为了确保在相同 Prompt 下 DeepSeek V4 Flash 产生**一致且语义无差异**的输出，使用者须遵循以下 4 条硬性配置：

1. **API 解码参数**：
   * `temperature`: `0.0`（启用 Greed Decoding 贪婪采样，消除随机熵）
   * `top_p`: `1.0`
   * `seed`: `42`（显式固定随机数种子）
2. **规范字典序 (Canonical Sorting)**：
   * Rust 提示词构建器会在生成 Prompt 前，使用 `BTreeMap` 对词库候选集按 ID 进行字母升序排序，防止 HashMap 遍历乱序导致 LLM 注意力漂移。
3. **80 标签硬上限 (Candidate Cap)**：
   * 单次投喂的 tag 总数严格限制在 80 个以内，候选词最多 96 个，防止长上下文末端注意力衰减 (Attention Decay)。
4. **单次幂等校验重试 (Guardrail Retry)**：
   * LLM 输出经 `validate_selection` 校验，若存在非法 ID，用同 Seed 自动重试一次，仍失败则归一化报错，不写入破坏性数据。

---

## 四、 Rust 使用示例

```rust
use novels::prompt::{build_derivation_prompt, build_system_prompt, DerivationPromptInput};
use std::collections::BTreeMap;

// 1. 获取 System Prompt
let system_prompt = build_system_prompt();

// 2. 组装输入参数
let mut candidates = BTreeMap::new();
// ... 填充 candidates ...

let input = DerivationPromptInput {
    character_name: "姜宁",
    character_tags: &["坚韧".to_string(), "投资人".to_string()],
    character_skills: &["并购".to_string()],
    scene_objective_event: "慈善拍卖会上天价举牌，与陆沉重逢",
    memories: &["十年前姜家受恶意竞争破产".to_string()],
    candidates_by_sense: &candidates,
};

// 3. 自动生成最佳 Prompt
let user_prompt = build_derivation_prompt(&input);

// 4. 提交给 DeepSeek (temperature = 0.0)
println!("System Prompt:\n{}", system_prompt);
println!("User Prompt:\n{}", user_prompt);
```
