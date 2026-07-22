//! Programmatic free-text guards (anti AI-taste / AI-causal glue).
//! Shared by prose action, memory content, and plot reasons.

/// Max characters kept in a single prose `action` after sanitize.
pub const MAX_ACTION_CHARS: usize = 80;
/// Max characters for persisted character memory content.
pub const MAX_MEMORY_CHARS: usize = 120;
/// Max characters for each plot development reason.
pub const MAX_PLOT_REASON_CHARS: usize = 60;

/// Common AI-causal / glue fillers stripped from free LLM text.
const CAUSAL_FILLERS: &[&str] = &[
    "因此",
    "于是",
    "所以",
    "不禁",
    "不由得",
    "心中暗想",
    "暗想",
    "似乎感到",
    "仿佛感到",
    "突然意识到",
    "忍不住",
    "不由自主",
    "冥冥之中",
    "命运般",
    // Platform reverse: AI 说明文 / 套话 glue (P1)
    "不禁想到",
    "心中涌起",
    "值得注意的是",
    "总而言之",
    "综上所述",
    "必然",
    "表明",
    "意味着",
];

/// Strip causal fillers and hard-clamp length (UTF-8 char count).
pub fn sanitize_free_text(text: &str, max_chars: usize) -> String {
    let mut s = text.to_string();
    for filler in CAUSAL_FILLERS {
        s = s.replace(filler, "");
    }
    // Collapse whitespace runs to a single space (preserve Latin words; Chinese usually has none).
    let s = s.split_whitespace().collect::<Vec<_>>().join(" ");
    let chars: Vec<char> = s.chars().collect();
    if chars.len() > max_chars {
        chars.into_iter().take(max_chars).collect()
    } else {
        chars.into_iter().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strips_fillers_and_clamps() {
        let out = sanitize_free_text("因此他于是走了不禁", 4);
        assert!(!out.contains("因此"));
        assert!(!out.contains("于是"));
        assert!(!out.contains("不禁"));
        assert!(out.chars().count() <= 4);
    }

    #[test]
    fn memory_cap_constant_holds() {
        let long = "他".repeat(200);
        let out = sanitize_free_text(&long, MAX_MEMORY_CHARS);
        assert_eq!(out.chars().count(), MAX_MEMORY_CHARS);
    }

    #[test]
    fn strips_platform_ai_explanatory_fillers() {
        let out = sanitize_free_text("这必然意味着危险表明有人", 80);
        assert!(!out.contains("必然"));
        assert!(!out.contains("意味着"));
        assert!(!out.contains("表明"));
    }
}
