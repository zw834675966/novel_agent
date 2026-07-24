# -*- coding: utf-8 -*-
"""独立复核:抽样验证 assets/distilled/*.yaml 的 text 均为 corpus 原文逐字子串。"""
import random
import re
from pathlib import Path

random.seed(7)


def norm(s):
    return re.sub(r"\s", "", s)


bad = []
total = 0
for f in sorted(Path("assets/distilled").glob("*.yaml")):
    book, chap = f.stem.split("-c")
    src = norm(Path(f"corpus/{book}/c{chap}.txt").read_text(encoding="utf-8"))
    texts = re.findall(r'text: "(.*)"', f.read_text(encoding="utf-8"))
    for t in random.sample(texts, min(10, len(texts))):
        total += 1
        t2 = t.replace('\\"', '"').replace("\\\\", "\\")
        if norm(t2) not in src:
            bad.append((f.name, t[:30]))

print(f"independent verbatim check: {total - len(bad)}/{total} pass")
for b in bad[:5]:
    print("FAIL:", b)
