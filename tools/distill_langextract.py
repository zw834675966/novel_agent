# -*- coding: utf-8 -*-
"""langextract + Claude 蒸馏管线。

corpus/<book>/cNNN.txt → langextract(Claude via litellm)→ assets/distilled/<book>-cNNN.yaml

防 AI 化双保险:
  1. langextract source grounding:抽取自动对齐源文本字符区间,对不上的丢弃
  2. tools/validate_fragments.py 独立逐字复核(与 Rust 引擎同标准)

用法:
  python tools/distill_langextract.py hlm 1 5              # 默认 claude
  python tools/distill_langextract.py hlm 6 120 --model=step
  python tools/distill_langextract.py zhz 6 68 --model=claude
断点续跑:输出文件已存在的章节自动跳过。
"""
import os
import re
import sys
from collections import Counter
from pathlib import Path

import langextract as lx
from langextract_litellm.provider import LiteLLMLanguageModel

ROOT = Path(__file__).resolve().parent.parent

# 模型注册表:name -> (litellm model_id, api_key env, api_base env, litellm 读取的 key env)
# 注:langextract-litellm 的 __init__ 会吞掉 api_key 形参不传给 litellm.completion,
#     litellm 实际从各 provider 的标准环境变量读 key,故需按 provider 写入对应变量。
MODELS = {
    "claude": (
        "litellm/anthropic/claude-haiku-4-5-20251001",
        "ANTHROPIC_API_KEY",
        "ANTHROPIC_BASE_URL",
        "ANTHROPIC_API_KEY",
    ),
    "step": (
        "litellm/openai/step-3.7-flash",
        "STEPFUN_API_KEY",
        "STEPFUN_BASE_URL",
        "OPENAI_API_KEY",
    ),
}

CATEGORIES = [
    "visual", "auditory", "olfactory", "tactile",
    "gustatory", "emotion", "gesture", "atmosphere",
]
MIN_LEN, MAX_LEN = 6, 120

PROMPT = """从中文小说原文中抽取值得复用的细粒度描写片段,按出现顺序。

extraction_class 只允许这 8 个值,不得自创类别:
- visual: 视觉描写(容貌、服饰、光色、景物之所见)
- auditory: 听觉描写
- olfactory: 嗅觉描写
- tactile: 触觉描写(冷暖、质地、体感)
- gustatory: 味觉描写
- emotion: 情绪与心理描写(内心波动、隐忍、悲喜、彻悟)
- gesture: 动作与神态描写(举手投足、表情变化)
- atmosphere: 氛围与环境描写(光影、季节、庭院陈设、世态)

铁律:
1. extraction_text 必须逐字摘录原文连续片段,一个字不改,不增删标点,不拼接。
2. 每片段 6-120 字,选最有神韵的完整短句或从句。
3. 每段给定文本抽取 8-20 个片段;正文密集处宁多勿漏,但平铺直叙的功能性句子不要。
4. extraction_text 的值必须是单个字符串,不得是列表或对象。
5. attributes.tags 给 2-3 个:出场人物名、场合、情绪基调。"""

EXAMPLES = [
    lx.data.ExampleData(
        text=(
            "凤姐听了，眼圈儿红了半天，半日方说道：“天有不测风云，人有旦夕祸福。”"
            "说着，泪如雨下。窗外的雪不知何时又下了起来，压得竹枝簌簌作响，"
            "屋里炭盆的火光映着她半边脸，忽明忽暗。她伸手把帕子绞了又绞，"
            "指尖冰凉，袖口一缕冷香散进烟气里。"
        ),
        extractions=[
            lx.data.Extraction(
                extraction_class="emotion",
                extraction_text="眼圈儿红了半天，半日方说道",
                attributes={"tags": ["王熙凤", "隐忍", "悲伤"]},
            ),
            lx.data.Extraction(
                extraction_class="gesture",
                extraction_text="她伸手把帕子绞了又绞",
                attributes={"tags": ["王熙凤", "焦虑", "小动作"]},
            ),
            lx.data.Extraction(
                extraction_class="auditory",
                extraction_text="压得竹枝簌簌作响",
                attributes={"tags": ["雪夜", "竹", "静谧"]},
            ),
            lx.data.Extraction(
                extraction_class="atmosphere",
                extraction_text="屋里炭盆的火光映着她半边脸，忽明忽暗",
                attributes={"tags": ["室内", "火光", "阴晴不定"]},
            ),
            lx.data.Extraction(
                extraction_class="tactile",
                extraction_text="指尖冰凉",
                attributes={"tags": ["王熙凤", "寒冷"]},
            ),
            lx.data.Extraction(
                extraction_class="olfactory",
                extraction_text="袖口一缕冷香散进烟气里",
                attributes={"tags": ["冷香", "烟气", "室内"]},
            ),
        ],
    ),
]


# 标点宽度归一化表:LLM 常把全角标点写成半角,匹配时统一;落库文本仍从原文回取
_PUNCT_MAP = str.maketrans({
    ",": "，", ":": "：", ";": "；", "!": "！", "?": "？",
    "(": "（", ")": "）", '"': "”", "'": "’",
})


def norm(s: str) -> str:
    return re.sub(r"\s", "", s).translate(_PUNCT_MAP)


def build_norm_index(s: str) -> tuple[str, list[int]]:
    """归一化字符串 + 每个归一化字符在原文中的位置(用于把匹配跨度映射回原文)。"""
    chars: list[str] = []
    idx: list[int] = []
    for i, ch in enumerate(s):
        if ch.isspace():
            continue
        chars.append(ch.translate(_PUNCT_MAP))
        idx.append(i)
    return "".join(chars), idx


def locate_verbatim(frag: str, src: str, src_norm: str, src_idx: list[int]) -> str | None:
    """在原文中定位片段;命中则返回原文原字的连续子串,否则 None。"""
    n = norm(frag)
    if not n:
        return None
    p = src_norm.find(n)
    if p < 0:
        return None
    start = src_idx[p]
    end = src_idx[p + len(n) - 1] + 1
    return src[start:end]


def yaml_escape(s: str) -> str:
    return s.replace("\\", "\\\\").replace('"', '\\"')


def load_env_file():
    """读取项目 .env(STEPFUN_* 等凭据;已在 .gitignore)。"""
    env_path = ROOT / ".env"
    if not env_path.exists():
        return
    for line in env_path.read_text(encoding="utf-8").splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, _, v = line.partition("=")
            os.environ.setdefault(k.strip(), v.strip())


def make_model(name: str) -> LiteLLMLanguageModel:
    model_id, key_env, base_env, litellm_key_env = MODELS[name]
    os.environ[litellm_key_env] = os.environ[key_env]
    return LiteLLMLanguageModel(
        model_id=model_id,
        api_base=os.environ[base_env],
        max_tokens=8192,
        temperature=0.2,
    )


def distill_chapter(
    book: str, chap: int, out_dir: Path, model: LiteLLMLanguageModel
) -> Counter | None:
    src_path = ROOT / "corpus" / book / f"c{chap:03d}.txt"
    out_path = out_dir / f"{book}-c{chap:03d}.yaml"
    if out_path.exists():
        print(f"skip c{chap:03d} (exists)")
        return None
    if not src_path.exists():
        print(f"skip c{chap:03d} (no corpus)")
        return None
    text = src_path.read_text(encoding="utf-8")
    src_norm, src_idx = build_norm_index(text)

    # 单章重试:0 片段时重试,最多 3 次(StepFun reasoning 偶发返回空 content)
    result = None
    for attempt in range(3):
        result = lx.extract(
            text_or_documents=text,
            prompt_description=PROMPT,
            examples=EXAMPLES,
            model=model,
            max_char_buffer=1500,
            max_workers=2,  # 降并发:8 进程 × 高 worker 会触发中转抖动
            extraction_passes=2,
            fence_output=True,
            use_schema_constraints=False,
        )
        if result.extractions:
            break
        print(f"  c{chap:03d} attempt {attempt+1}: empty, retrying...")

    stats = Counter()
    seen: set[str] = set()
    kept: list[tuple[str, str, list[str]]] = []  # (category, text, tags)
    seq = 0
    for e in result.extractions:
        cat = e.extraction_class
        frag = e.extraction_text or ""
        if cat not in CATEGORIES:
            stats["bad_category"] += 1
            stats[f"badcat:{cat}"] += 1
            continue
        if not (MIN_LEN <= len(frag) <= MAX_LEN):
            stats["bad_length"] += 1
            continue
        # 双保险第一道:归一化(空白+标点宽度)定位,命中则回取原文原字落库
        verbatim = locate_verbatim(frag, text, src_norm, src_idx)
        if verbatim is None:
            stats["not_verbatim"] += 1
            continue
        n = norm(verbatim)
        if n in seen:
            stats["dup"] += 1
            continue
        seen.add(n)
        tags = []
        if e.attributes and isinstance(e.attributes.get("tags"), list):
            tags = [str(t) for t in e.attributes["tags"][:3]]
        kept.append((cat, verbatim, tags))
        stats["kept"] += 1

    # 0 片段不写文件:让断点续跑下次重蒸(避免空壳阻塞)
    if not kept:
        print(f"c{chap:03d}: 0 fragments after {3} attempts, not writing (will retry next run)")
        return stats

    # 按类别分组写 YAML(与 assets/vocab.yaml 同构)
    lines: list[str] = []
    for cat in CATEGORIES:
        group = [(f, t) for c, f, t in kept if c == cat]
        if not group:
            continue
        lines.append(f"{cat}:")
        for frag, tags in group:
            seq += 1
            lines.append(f"  {book}-c{chap:03d}-{seq:02d}:")
            lines.append(f'    text: "{yaml_escape(frag)}"')
            all_tags = [book] + tags
            tag_str = ", ".join(f'"{yaml_escape(t)}"' for t in all_tags)
            lines.append(f"    tags: [{tag_str}]")
    out_path.write_text("\n".join(lines) + "\n", encoding="utf-8")
    print(f"c{chap:03d}: kept={stats['kept']} "
          f"not_verbatim={stats['not_verbatim']} dup={stats['dup']} "
          f"bad_len={stats['bad_length']} -> {out_path.name}")
    return stats


def main():
    load_env_file()
    args = [a for a in sys.argv[1:] if not a.startswith("--")]
    opts = [a for a in sys.argv[1:] if a.startswith("--")]
    book, start, end = args[0], int(args[1]), int(args[2])
    model_name = "claude"
    for o in opts:
        if o.startswith("--model="):
            model_name = o.split("=", 1)[1]
    if model_name not in MODELS:
        sys.exit(f"unknown model {model_name!r}, choose from {list(MODELS)}")
    out_dir = ROOT / "assets" / "distilled"
    out_dir.mkdir(parents=True, exist_ok=True)
    model = make_model(model_name)
    print(f"model={model_name} book={book} range={start}-{end}")
    total = Counter()
    for chap in range(start, end + 1):
        stats = distill_chapter(book, chap, out_dir, model)
        if stats:
            total.update(stats)
    print(f"\n=== done === {dict(total)}")


if __name__ == "__main__":
    main()
