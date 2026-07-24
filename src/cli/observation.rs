// CLI Harness 观测类型
// ====================
// 每个 harness 命令返回一个 `Observation`：状态、摘要、产物、质量 KPI、下一步建议。
// 支持人类可读文本（REPL）与机器可读 JSON（`--json`）两种渲染。

use std::collections::BTreeMap;
use std::fmt;

use serde::Serialize;

use crate::prose::AssembledProse;

/// 命令观测状态。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    Success,
    Warning,
    Error,
}

impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Status::Success => f.write_str("success"),
            Status::Warning => f.write_str("warning"),
            Status::Error => f.write_str("error"),
        }
    }
}

/// 从 [`AssembledProse`] 提取的质量 KPI（VeriCite 式后验可观测）。
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct QualityKpis {
    pub quote_density: f64,
    pub stripped_refs: usize,
    pub unverified_quotes: usize,
    pub low_quote_density: bool,
    pub action_only_beats: usize,
    pub rejected_beats: usize,
}

/// 结构化命令结果：状态 + 摘要 + 产物 + 质量 + 下一步建议。
#[derive(Debug, Clone, Serialize)]
pub struct Observation {
    pub status: Status,
    pub summary: String,
    pub artifacts: BTreeMap<String, String>,
    pub quality: Option<QualityKpis>,
    pub next: Vec<String>,
}

impl Observation {
    /// 渲染观测：`json = false` 为人类可读文本，`json = true` 为缩进 JSON。
    pub fn render(&self, json: bool) -> String {
        if json {
            serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into())
        } else {
            self.render_human()
        }
    }

    /// 人类可读文本（REPL 默认输出）。
    fn render_human(&self) -> String {
        let mut out = String::new();
        out.push_str(&format!("status: {}\n", self.status));
        out.push_str(&format!("summary: {}\n", self.summary));
        out.push_str("artifacts:\n");
        for (k, v) in &self.artifacts {
            out.push_str(&format!("  {}: {}\n", k, v));
        }
        if let Some(q) = &self.quality {
            out.push_str("quality:\n");
            out.push_str(&format!("  quote_density: {}\n", q.quote_density));
            out.push_str(&format!("  stripped_refs: {}\n", q.stripped_refs));
            out.push_str(&format!("  unverified_quotes: {}\n", q.unverified_quotes));
            out.push_str(&format!("  low_quote_density: {}\n", q.low_quote_density));
            out.push_str(&format!("  action_only_beats: {}\n", q.action_only_beats));
            out.push_str(&format!("  rejected_beats: {}\n", q.rejected_beats));
        }
        out.push_str("next:\n");
        for n in &self.next {
            out.push_str(&format!("  - {}\n", n));
        }
        out
    }
}

/// 把 [`AssembledProse`] 映射为 harness 质量 KPI。
///
/// `quote_density` 取自 `AssembledProse::quote_density()`（规范值），
/// 而非 `quality.quote_density`——两者可能不一致。
pub fn quality_from_prose(p: &AssembledProse) -> QualityKpis {
    QualityKpis {
        quote_density: p.quote_density(),
        stripped_refs: p.stripped_refs,
        unverified_quotes: p.unverified_quotes,
        low_quote_density: p.quality.low_quote_density,
        action_only_beats: p.action_only_beats,
        rejected_beats: p.rejected_beats,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prose::{AssembledProse, ProseQualityReport};

    #[test]
    fn render_human_includes_status_and_summary() {
        let o = Observation {
            status: Status::Warning,
            summary: "DEEPSEEK_API_KEY 未设置，使用 Mock（非生产质量）".into(),
            artifacts: Default::default(),
            quality: None,
            next: vec![],
        };
        let s = o.render(false);
        assert!(s.contains("status: warning"));
        assert!(s.contains("非生产质量"));
    }

    #[test]
    fn render_human_status_each_variant() {
        for (status, expect) in [
            (Status::Success, "status: success"),
            (Status::Warning, "status: warning"),
            (Status::Error, "status: error"),
        ] {
            let o = Observation {
                status,
                summary: "x".into(),
                artifacts: Default::default(),
                quality: None,
                next: vec![],
            };
            assert!(o.render(false).contains(expect), "missing {expect}");
        }
    }

    #[test]
    fn render_human_quality_block() {
        let o = Observation {
            status: Status::Success,
            summary: "narrated".into(),
            artifacts: Default::default(),
            quality: Some(QualityKpis {
                quote_density: 0.42,
                stripped_refs: 2,
                unverified_quotes: 1,
                low_quote_density: false,
                action_only_beats: 3,
                rejected_beats: 4,
            }),
            next: vec![],
        };
        let s = o.render(false);
        assert!(s.contains("quality:"));
        assert!(s.contains("quote_density: 0.42"));
        assert!(s.contains("stripped_refs: 2"));
        assert!(s.contains("unverified_quotes: 1"));
        assert!(s.contains("low_quote_density: false"));
        assert!(s.contains("action_only_beats: 3"));
        assert!(s.contains("rejected_beats: 4"));
    }

    #[test]
    fn render_human_artifacts_and_next() {
        let mut artifacts = BTreeMap::new();
        artifacts.insert(
            "scene_id".into(),
            "11111111-1111-1111-1111-111111111111".into(),
        );
        let o = Observation {
            status: Status::Success,
            summary: "derived".into(),
            artifacts,
            quality: None,
            next: vec!["narrate".into(), "show derivation".into()],
        };
        let s = o.render(false);
        assert!(s.contains("artifacts:"));
        assert!(s.contains("scene_id: 11111111-1111-1111-1111-111111111111"));
        assert!(s.contains("next:"));
        assert!(s.contains("  - narrate"));
        assert!(s.contains("  - show derivation"));
    }

    #[test]
    fn render_json_status_is_lowercase_string() {
        let o = Observation {
            status: Status::Warning,
            summary: "mock".into(),
            artifacts: Default::default(),
            quality: None,
            next: vec![],
        };
        let s = o.render(true);
        // status must serialize as a lowercase JSON string, not the debug enum name.
        assert!(s.contains("\"status\": \"warning\""), "got: {s}");
        assert!(!s.contains("Warning"), "enum debug leaked into JSON: {s}");
    }

    #[test]
    fn render_json_includes_all_fields() {
        let mut artifacts = BTreeMap::new();
        artifacts.insert("scene_id".into(), "abc".into());
        let o = Observation {
            status: Status::Success,
            summary: "done".into(),
            artifacts,
            quality: Some(QualityKpis {
                quote_density: 0.5,
                stripped_refs: 1,
                unverified_quotes: 0,
                low_quote_density: false,
                action_only_beats: 0,
                rejected_beats: 0,
            }),
            next: vec!["narrate".into()],
        };
        let s = o.render(true);
        assert!(s.contains("\"summary\": \"done\""));
        assert!(s.contains("\"scene_id\": \"abc\""));
        assert!(s.contains("\"quote_density\": 0.5"));
        assert!(s.contains("\"next\": ["));
        assert!(s.contains("\"narrate\""));
    }

    #[test]
    fn quality_from_prose_maps_kpis() {
        // quote_chars/total_chars drive the canonical quote_density() (40/100 = 0.4).
        // quality.quote_density is deliberately 0.99 to prove we use the method.
        let prose = AssembledProse {
            text: "心中未免悔恨。她坐下".into(),
            stripped_refs: 3,
            rejected_beats: 1,
            action_only_beats: 2,
            quote_chars: 40,
            total_chars: 100,
            unverified_quotes: 4,
            quality: ProseQualityReport {
                quote_density: 0.99,
                action_only_rate: 0.5,
                stripped_ref_rate: 0.3,
                unverified_quotes: 4,
                low_quote_density: true,
                sensory_diversity_score: 0.0,
                missing_senses: vec![],
                degraded_sensory_density: false,
            },
        };
        let k = quality_from_prose(&prose);
        // Canonical method value, NOT quality.quote_density (0.99).
        assert!(
            (k.quote_density - 0.4).abs() < 1e-9,
            "got {}",
            k.quote_density
        );
        assert_eq!(k.stripped_refs, 3);
        assert_eq!(k.unverified_quotes, 4);
        assert!(k.low_quote_density);
        assert_eq!(k.action_only_beats, 2);
        assert_eq!(k.rejected_beats, 1);
    }
}
