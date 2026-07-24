// 语料蒸馏器
// ============
// 从 corpus/<book>/cNNN.txt 逐章抽取细粒度描写片段,产出 assets/distilled/<book>.yaml。
//
// 防 AI 化核心:LLM 只做"定位与分类",不做改写。每个返回片段都要通过
// 逐字子串校验(去空白后必须是源章节的连续子串),不是原文的一律丢弃。
//
// 用法:
//   cargo run --bin distill -- <book> <start_chap> <end_chap>
//   cargo run --bin distill -- hlm 1 5
//   cargo run --bin distill -- zhz 1 5
//
// 断点续跑:输出目录已存在 <book>-cNNN.yaml 的章节自动跳过。

use rig::client::{CompletionClient, ProviderClient};
use rig::extractor::Extractor;
use rig::providers::deepseek;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fmt::Write as _;
use std::path::PathBuf;

/// 每次调用给 LLM 的正文块大小(字符数)。太大召回率低,太小上下文断裂。
const CHUNK_CHARS: usize = 2500;
/// 单片段最小/最大长度(字符):过短没有"灵魂",过长不成"词条"。
const MIN_EXCERPT: usize = 6;
const MAX_EXCERPT: usize = 120;

/// 蒸馏输出的单个片段
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
struct Fragment {
    /// 类别:visual/auditory/olfactory/tactile/gustatory/emotion/gesture/atmosphere 之一
    category: String,
    /// 逐字摘录的原文片段(不得改写、增删一个字)
    excerpt: String,
    /// 标签:人物名、情绪、场合等,2-4 个
    tags: Vec<String>,
}

/// LLM 单次调用的返回批
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct FragmentBatch {
    fragments: Vec<Fragment>,
}

const CATEGORIES: [&str; 8] = [
    "visual",
    "auditory",
    "olfactory",
    "tactile",
    "gustatory",
    "emotion",
    "gesture",
    "atmosphere",
];

fn build_prompt(chunk: &str) -> String {
    let mut s = String::new();
    s.push_str(
        "你是文学描写片段标注器。从下面的小说原文中找出值得复用的细粒度描写片段。\n\
         类别定义:\n\
         - visual/auditory/olfactory/tactile/gustatory: 五感描写\n\
         - emotion: 情绪与心理描写(如内心波动、隐忍、悲喜)\n\
         - gesture: 动作与神态描写(如举手投足、表情变化)\n\
         - atmosphere: 氛围与环境描写(如光影、季节、庭院陈设)\n\
         \n\
         铁律:excerpt 必须逐字摘录原文连续片段,一个字都不许改、不许增删标点。\n\
         每个片段 6-120 字,选最有神韵的完整短句或从句。每块原文抽 8-20 个片段。\n\
         tags 给 2-4 个:出场人物名、情绪基调、场合。\n\n=== 原文 ===\n",
    );
    s.push_str(chunk);
    s.push_str("\n\n请调用 submit 提交结构化结果。");
    s
}

/// 归一化:去所有空白,用于子串比对(源文本排版差异不应影响校验)
fn normalize(s: &str) -> String {
    s.chars().filter(|c| !c.is_whitespace()).collect()
}

/// YAML 双引号字符串转义
fn yaml_escape(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

struct Stats {
    kept: usize,
    dropped_not_verbatim: usize,
    dropped_bad_category: usize,
    dropped_length: usize,
    dropped_dup: usize,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let args: Vec<String> = std::env::args().collect();
    let (book, start, end) = match args.as_slice() {
        [_, b, s, e] => (b.clone(), s.parse::<u32>()?, e.parse::<u32>()?),
        _ => anyhow::bail!("usage: distill <book> <start_chap> <end_chap>"),
    };

    let client = deepseek::Client::from_env()
        .map_err(|e| anyhow::anyhow!("DEEPSEEK_API_KEY required: {e:?}"))?;
    let extractor: Extractor<deepseek::CompletionModel, FragmentBatch> = client
        .extractor::<FragmentBatch>(deepseek::DEEPSEEK_V4_FLASH)
        .retries(1)
        .build();

    let out_dir = PathBuf::from("assets/distilled");
    std::fs::create_dir_all(&out_dir)?;

    let mut total = Stats {
        kept: 0,
        dropped_not_verbatim: 0,
        dropped_bad_category: 0,
        dropped_length: 0,
        dropped_dup: 0,
    };

    for chap in start..=end {
        let src = PathBuf::from(format!("corpus/{book}/c{chap:03}.txt"));
        let out = out_dir.join(format!("{book}-c{chap:03}.yaml"));
        if out.exists() {
            eprintln!("skip c{chap:03} (exists)");
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&src) else {
            eprintln!("skip c{chap:03} (no corpus file)");
            continue;
        };
        let source_norm = normalize(&text);

        // 分块:按字符数切,不切断段落(在块边界向后找最近换行)
        let chars: Vec<char> = text.chars().collect();
        let mut chunks = Vec::new();
        let mut i = 0;
        while i < chars.len() {
            let mut j = (i + CHUNK_CHARS).min(chars.len());
            while j < chars.len() && chars[j] != '\n' {
                j += 1;
            }
            chunks.push(chars[i..j].iter().collect::<String>());
            i = j;
        }

        let mut seen: HashSet<String> = HashSet::new();
        // category -> Vec<(id_key, fragment)>
        let mut kept: Vec<(String, Fragment)> = Vec::new();
        let mut seq = 0usize;

        for (ci, chunk) in chunks.iter().enumerate() {
            let batch = match extractor.extract(&build_prompt(chunk)).await {
                Ok(b) => b,
                Err(e) => {
                    eprintln!("  c{chap:03} chunk {ci}: LLM error, skipping: {e:?}");
                    continue;
                }
            };
            for f in batch.fragments {
                if !CATEGORIES.contains(&f.category.as_str()) {
                    total.dropped_bad_category += 1;
                    continue;
                }
                let n = f.excerpt.chars().count();
                if !(MIN_EXCERPT..=MAX_EXCERPT).contains(&n) {
                    total.dropped_length += 1;
                    continue;
                }
                let norm = normalize(&f.excerpt);
                // 逐字校验:归一化后必须是源章节连续子串
                if !source_norm.contains(&norm) {
                    total.dropped_not_verbatim += 1;
                    continue;
                }
                if !seen.insert(norm) {
                    total.dropped_dup += 1;
                    continue;
                }
                seq += 1;
                let key = format!("{book}-c{chap:03}-{seq:02}");
                kept.push((key, f));
                total.kept += 1;
            }
            eprintln!(
                "  c{chap:03} chunk {}/{}: kept so far {}",
                ci + 1,
                chunks.len(),
                seq
            );
        }

        // 写 YAML:按类别分组,结构与 assets/vocab.yaml 一致
        let mut yaml = String::new();
        for cat in CATEGORIES {
            let group: Vec<_> = kept.iter().filter(|(_, f)| f.category == cat).collect();
            if group.is_empty() {
                continue;
            }
            writeln!(yaml, "{cat}:")?;
            for (key, f) in group {
                writeln!(yaml, "  {key}:")?;
                writeln!(yaml, "    text: \"{}\"", yaml_escape(&f.excerpt))?;
                let tags: Vec<String> = std::iter::once(book.clone())
                    .chain(f.tags.iter().take(4).map(|t| yaml_escape(t)))
                    .map(|t| format!("\"{t}\""))
                    .collect();
                writeln!(yaml, "    tags: [{}]", tags.join(", "))?;
            }
        }
        std::fs::write(&out, yaml)?;
        eprintln!("c{chap:03}: {} fragments -> {}", seq, out.display());
    }

    eprintln!(
        "\n=== done ===\nkept={} not_verbatim={} bad_category={} bad_length={} dup={}",
        total.kept,
        total.dropped_not_verbatim,
        total.dropped_bad_category,
        total.dropped_length,
        total.dropped_dup
    );
    Ok(())
}
