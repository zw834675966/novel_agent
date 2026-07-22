# -*- coding: utf-8 -*-
"""强制校验器:检查 assets/distilled/*.yaml 每一条 text 是否为 corpus 原文逐字子串。

用法:
  python tools/validate_fragments.py           # 校验全部,只报告
  python tools/validate_fragments.py --prune   # 校验并删除非逐字条目(重写文件)
  python tools/validate_fragments.py --drop-dialogue  # 额外将对话句启发式计入 DROP
  python tools/validate_fragments.py hlm-c001  # 只校验指定文件(stem 前缀匹配)

与 distill.rs 的内联校验同一标准:归一化(去所有空白)后必须是源章节连续子串。
这是"从源头杜绝 AI 化"的最后防线:不管片段来自 DeepSeek 还是 Claude,
非原文一律不进素材库。
"""
import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DISTILLED = ROOT / "assets" / "distilled"

CATEGORIES = {
    "visual", "auditory", "olfactory", "tactile",
    "gustatory", "emotion", "gesture", "atmosphere",
}
MIN_LEN, MAX_LEN = 6, 120

DIALOGUE_RE = re.compile(r"(道|说|问道|笑道|便道)[：“\"]")


def looks_like_dialogue(text: str) -> bool:
    if text.startswith(("笑道", "便道", "说道", "问道")):
        return True
    return bool(DIALOGUE_RE.search(text))


def norm(s: str) -> str:
    return re.sub(r"\s", "", s)


def unescape(s: str) -> str:
    return s.replace('\\"', '"').replace("\\\\", "\\")


def validate_file(path: Path, prune: bool, drop_dialogue: bool = False) -> tuple[int, int]:
    book, chap = path.stem.split("-c")
    src_path = ROOT / "corpus" / book / f"c{chap}.txt"
    src = norm(src_path.read_text(encoding="utf-8"))

    lines = path.read_text(encoding="utf-8").splitlines()
    ok, bad = 0, 0
    out_lines = []
    seen_norm = set()
    i = 0
    current_cat = None
    while i < len(lines):
        line = lines[i]
        m_cat = re.match(r"^(\w+):$", line)
        m_key = re.match(r"^  ([\w.-]+):$", line)
        if m_cat:
            current_cat = m_cat.group(1)
            out_lines.append(line)
            i += 1
            continue
        if m_key and i + 2 < len(lines):
            m_text = re.match(r'^    text: "(.*)"$', lines[i + 1])
            m_tags = re.match(r"^    tags: \[.*\]$", lines[i + 2])
            if m_text and m_tags:
                text = unescape(m_text.group(1))
                n = norm(text)
                drop_reason = None
                if current_cat not in CATEGORIES:
                    drop_reason = f"bad category {current_cat}"
                elif not (MIN_LEN <= len(text) <= MAX_LEN):
                    drop_reason = f"bad length {len(text)}"
                elif n not in src:
                    drop_reason = "NOT VERBATIM"
                elif n in seen_norm:
                    drop_reason = "duplicate"
                elif drop_dialogue and looks_like_dialogue(text):
                    drop_reason = "dialogue"
                if drop_reason:
                    bad += 1
                    print(f"  DROP [{path.name}] {m_key.group(1)}: {drop_reason}: {text[:40]}")
                    if not prune:
                        out_lines.extend(lines[i:i + 3])
                else:
                    ok += 1
                    seen_norm.add(n)
                    out_lines.extend(lines[i:i + 3])
                i += 3
                continue
        out_lines.append(line)
        i += 1

    if prune and bad:
        # 清掉可能变空的类别标题(标题行后面紧跟另一个标题或 EOF)
        cleaned = []
        for j, line in enumerate(out_lines):
            if re.match(r"^\w+:$", line):
                nxt = out_lines[j + 1] if j + 1 < len(out_lines) else None
                if nxt is None or re.match(r"^\w+:$", nxt):
                    continue
            cleaned.append(line)
        path.write_text("\n".join(cleaned) + "\n", encoding="utf-8")
    return ok, bad


def main():
    prune = "--prune" in sys.argv
    drop_dialogue = "--drop-dialogue" in sys.argv
    stems = [a for a in sys.argv[1:] if not a.startswith("--")]
    files = sorted(DISTILLED.glob("*.yaml"))
    if stems:
        files = [f for f in files if any(f.stem.startswith(s) for s in stems)]
    total_ok, total_bad = 0, 0
    for f in files:
        ok, bad = validate_file(f, prune, drop_dialogue=drop_dialogue)
        total_ok += ok
        total_bad += bad
    print(f"\n=== {'pruned' if prune else 'checked'} {len(files)} files: "
          f"{total_ok} verbatim, {total_bad} dropped ===")
    sys.exit(1 if (total_bad and not prune) else 0)


if __name__ == "__main__":
    main()
