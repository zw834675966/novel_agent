//! 结构化动作模板（N3: Action Templating / ID化动作库）
//!
//! 目标：消除自由文本 action 的 AI 套路感（P7 晋江红线）。
//! 动作 = 舞台提示级（主语 + 动作 + 对象 + 可选对话），禁心理/环境描写。

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// 动作类型枚举（所有小说常见动作）
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ActionKind {
    // 移动类
    MoveToward,  // 走向
    MoveAway,    // 退开
    Enter,       // 进入
    Exit,        // 离开
    StandUp,     // 起身
    SitDown,     // 坐下
    Turn,        // 转身
    StepForward, // 上前
    StepBack,    // 后退

    // 观察类
    LookAt,   // 看向
    GlanceAt, // 瞥一眼
    StareAt,  // 凝视
    PeerAt,   // 仔细看

    // 肢体动作
    Reach,     // 伸手
    Touch,     // 触碰
    Hold,      // 握住
    Drop,      // 放下
    Bow,       // 躬身/行礼
    Nod,       // 点头
    ShakeHead, // 摇头
    Frown,     // 皱眉
    Smile,     // 微笑
    Sigh,      // 叹气

    // 言语类
    Say,     // 道
    Ask,     // 问
    Answer,  // 答
    Whisper, // 低语
    Shout,   // 喊道
    Exclaim, // 惊呼

    // 互动类
    Grab, // 抓住
    Push, // 推
    Pull, // 拉
    Help, // 搀扶

    // 其他动作
    Pause,    // 停步
    Hesitate, // 犹豫
    Search,   // 搜寻
    Inspect,  // 查看
}

/// 结构化动作（舞台提示级）
///
/// 严格约束：
/// - 禁止心理描写（"心中暗想"）
/// - 禁止环境描写（"夜色苍茫中"）
/// - 禁止因果套话（"因此" "于是"）
///
/// 生成规则：动作 + 对象 + 对话（可选）
/// 例：她 看向 尸体 道："这是谁？"
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct StructuredAction {
    /// 动作类型
    pub kind: ActionKind,
    /// 动作对象（人/物/方向），6字以内
    pub target: Option<String>,
    /// 对话内容，放在动作后，无引号前缀，30字以内
    pub dialogue: Option<String>,
}

impl StructuredAction {
    /// 是否是言语类动作（动词已带冒号后缀）
    fn is_speech_kind(&self) -> bool {
        matches!(
            self.kind,
            ActionKind::Say
                | ActionKind::Ask
                | ActionKind::Answer
                | ActionKind::Whisper
                | ActionKind::Shout
                | ActionKind::Exclaim
        )
    }

    /// 将结构化动作渲染为正文文本（无 AI 套话）
    pub fn render(&self, subject: &str) -> String {
        let mut result = String::from(subject);
        result.push_str(self.action_verb());

        // Target (cleaned)
        if let Some(target) = &self.target {
            let target = Self::clean_short_text(target, 6);
            if !target.is_empty() {
                result.push_str(&target);
            }
        }

        // Dialogue (cleaned)
        if let Some(dialogue) = &self.dialogue {
            let dialogue = Self::clean_short_text(dialogue, 30);
            if !dialogue.is_empty() {
                if self.is_speech_kind() {
                    // 言语类动作，动词已带冒号，直接接内容
                    result.push_str(&dialogue);
                } else {
                    // 非言语类动作，加"道："前缀
                    let has_target = self.target.is_some()
                        && self.target.as_ref().is_some_and(|t| !t.is_empty());
                    if has_target {
                        result.push_str("道：");
                    } else {
                        result.push('，');
                        result.push_str("道：");
                    }
                    result.push_str(&dialogue);
                }
            }
        }

        result
    }

    /// 获取动作的中文动词
    fn action_verb(&self) -> &'static str {
        match self.kind {
            ActionKind::MoveToward => "走向",
            ActionKind::MoveAway => "退开",
            ActionKind::Enter => "进入",
            ActionKind::Exit => "离开",
            ActionKind::StandUp => "起身",
            ActionKind::SitDown => "坐下",
            ActionKind::Turn => "转身",
            ActionKind::StepForward => "上前",
            ActionKind::StepBack => "后退",

            ActionKind::LookAt => "看向",
            ActionKind::GlanceAt => "瞥了一眼",
            ActionKind::StareAt => "凝视",
            ActionKind::PeerAt => "细看",

            ActionKind::Reach => "伸手去",
            ActionKind::Touch => "触碰",
            ActionKind::Hold => "握住",
            ActionKind::Drop => "放下",
            ActionKind::Bow => "躬身",
            ActionKind::Nod => "点头",
            ActionKind::ShakeHead => "摇头",
            ActionKind::Frown => "皱眉",
            ActionKind::Smile => "微笑",
            ActionKind::Sigh => "叹了口气",

            ActionKind::Say => "道：",
            ActionKind::Ask => "问：",
            ActionKind::Answer => "答：",
            ActionKind::Whisper => "低声道：",
            ActionKind::Shout => "喊道：",
            ActionKind::Exclaim => "惊道：",

            ActionKind::Grab => "抓住",
            ActionKind::Push => "推",
            ActionKind::Pull => "拉",
            ActionKind::Help => "搀扶",

            ActionKind::Pause => "停步",
            ActionKind::Hesitate => "犹豫",
            ActionKind::Search => "搜寻",
            ActionKind::Inspect => "查看",
        }
    }

    /// 清理短文本：去套话 + 限长
    fn clean_short_text(text: &str, max_chars: usize) -> String {
        crate::text_guard::sanitize_free_text(text, max_chars)
    }
}

/// 情节原因短槽（N4: Memory/Plot 结构化）
///
/// 替代 PlotDevelopment.reason 自由字符串。
/// 禁开放抒情，只限短事实槽（P1: 防 AI 因果说明文）。
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum PlotReasonSlot {
    /// 观察到某物
    Observed(String),
    /// 听到某话
    Heard(String),
    /// 发现异常
    NoticedAnomaly(String),
    /// 行为反常
    BehaviorOdd(String),
    /// 时间点/环境变化
    ContextChanged(String),
    /// 人物状态变化
    CharacterState(String),
    /// 对话内容触发
    DialogueContent(String),
    /// 物证线索
    PhysicalEvidence(String),
    /// 其他短原因（≤20字）
    Other(String),
}

impl PlotReasonSlot {
    /// 渲染为正文可复用字符串（去套话版）
    pub fn render(&self) -> String {
        match self {
            PlotReasonSlot::Observed(x) => {
                let x = Self::clean(x, 15);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("见{}", x)
                }
            }
            PlotReasonSlot::Heard(x) => {
                let x = Self::clean(x, 15);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("闻{}", x)
                }
            }
            PlotReasonSlot::NoticedAnomaly(x) => {
                let x = Self::clean(x, 15);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("觉{}有异", x)
                }
            }
            PlotReasonSlot::BehaviorOdd(x) => {
                let x = Self::clean(x, 12);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("{}举止反常", x)
                }
            }
            PlotReasonSlot::ContextChanged(x) => {
                let x = Self::clean(x, 12);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("{}有变", x)
                }
            }
            PlotReasonSlot::CharacterState(x) => {
                let x = Self::clean(x, 12);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("{}神色不对", x)
                }
            }
            PlotReasonSlot::DialogueContent(x) => {
                let x = Self::clean(x, 15);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("话及{}", x)
                }
            }
            PlotReasonSlot::PhysicalEvidence(x) => {
                let x = Self::clean(x, 15);
                if x.is_empty() {
                    String::new()
                } else {
                    format!("物证：{}", x)
                }
            }
            PlotReasonSlot::Other(x) => Self::clean(x, 20),
        }
    }

    fn clean(text: &str, max_chars: usize) -> String {
        crate::text_guard::sanitize_free_text(text, max_chars)
    }
}

/// 从字符串构造 StructuredAction（测试便利：字符串作为 Say 动作的 dialogue）
impl From<&str> for StructuredAction {
    fn from(s: &str) -> Self {
        Self {
            kind: ActionKind::Say,
            target: None,
            dialogue: Some(s.to_string()),
        }
    }
}

impl From<String> for StructuredAction {
    fn from(s: String) -> Self {
        Self::from(s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn structured_action_render_simple() {
        let action = StructuredAction {
            kind: ActionKind::LookAt,
            target: Some("尸体".into()),
            dialogue: None,
        };
        assert_eq!(action.render("她"), "她看向尸体");
    }

    #[test]
    fn structured_action_render_with_dialogue() {
        let action = StructuredAction {
            kind: ActionKind::LookAt,
            target: Some("那人".into()),
            dialogue: Some("你是谁？".into()),
        };
        assert_eq!(action.render("她"), "她看向那人道：你是谁？");
    }

    #[test]
    fn structured_action_speech_only() {
        let action = StructuredAction {
            kind: ActionKind::Say,
            target: None,
            dialogue: Some("此事蹊跷。".into()),
        };
        assert_eq!(action.render("他"), "他道：此事蹊跷。");
    }

    #[test]
    fn structured_action_cleanup_fillers() {
        let action = StructuredAction {
            kind: ActionKind::LookAt,
            target: Some("因此尸体".into()), // 含套话，应清理
            dialogue: Some("于是你是谁？".into()),
        };
        let result = action.render("她");
        assert!(!result.contains("因此"));
        assert!(!result.contains("于是"));
    }

    #[test]
    fn plot_reason_slot_render() {
        let reason = PlotReasonSlot::Observed("血迹".into());
        assert_eq!(reason.render(), "见血迹");

        let reason = PlotReasonSlot::Heard("异响".into());
        assert_eq!(reason.render(), "闻异响");

        let reason = PlotReasonSlot::NoticedAnomaly("面色".into());
        assert_eq!(reason.render(), "觉面色有异");
    }

    #[test]
    fn plot_reason_slot_cleans_fillers() {
        let reason = PlotReasonSlot::Observed("因此有人".into());
        let result = reason.render();
        assert!(!result.contains("因此"));
        assert!(result.contains("见"));
    }

    #[test]
    fn plot_reason_slot_clamps_length() {
        let long = "人".repeat(30);
        let reason = PlotReasonSlot::Other(long);
        let result = reason.render();
        assert!(result.chars().count() <= 20);
    }

    #[test]
    fn all_action_kinds_have_verbs() {
        // Test that every ActionKind variant returns a non-empty verb
        let kinds = [
            ActionKind::MoveToward,
            ActionKind::MoveAway,
            ActionKind::Enter,
            ActionKind::Exit,
            ActionKind::StandUp,
            ActionKind::SitDown,
            ActionKind::Turn,
            ActionKind::StepForward,
            ActionKind::StepBack,
            ActionKind::LookAt,
            ActionKind::GlanceAt,
            ActionKind::StareAt,
            ActionKind::PeerAt,
            ActionKind::Reach,
            ActionKind::Touch,
            ActionKind::Hold,
            ActionKind::Drop,
            ActionKind::Bow,
            ActionKind::Nod,
            ActionKind::ShakeHead,
            ActionKind::Frown,
            ActionKind::Smile,
            ActionKind::Sigh,
            ActionKind::Say,
            ActionKind::Ask,
            ActionKind::Answer,
            ActionKind::Whisper,
            ActionKind::Shout,
            ActionKind::Exclaim,
            ActionKind::Grab,
            ActionKind::Push,
            ActionKind::Pull,
            ActionKind::Help,
            ActionKind::Pause,
            ActionKind::Hesitate,
            ActionKind::Search,
            ActionKind::Inspect,
        ];

        for kind in kinds {
            let action = StructuredAction {
                kind,
                target: None,
                dialogue: None,
            };
            let verb = action.action_verb();
            assert!(!verb.is_empty(), "ActionKind {:?} has empty verb", kind);
            let rendered = action.render("他");
            assert!(rendered.starts_with("他"));
            assert!(rendered.len() > 1);
        }
    }
}
