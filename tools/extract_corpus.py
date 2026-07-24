# -*- coding: utf-8 -*-
"""语料抽取:temp/ 两本小说 → corpus/ 逐章纯文本。

依赖成熟开源库,不自己造轮子:
  ebooklib        — EPUB 解析(spine 顺序、文档项)
  beautifulsoup4  — HTML → 纯文本
  cn2an           — 中文数字 → 阿拉伯数字("二十三" → 23)

安装: pip install ebooklib beautifulsoup4 cn2an
用法: python tools/extract_corpus.py [all|hlm|zhz]
"""
import re
import sys
from pathlib import Path

import cn2an
from bs4 import BeautifulSoup
from ebooklib import ITEM_DOCUMENT, epub

ROOT = Path(__file__).resolve().parent.parent
TEMP = ROOT / "temp"
CORPUS = ROOT / "corpus"


def chapter_num(s: str) -> int:
    # 校对版用"○"(圈形)表示〇,cn2an 只认"〇"
    return int(cn2an.cn2an(s.replace("○", "〇"), "smart"))


def extract_hlm():
    src = next(TEMP.glob("*红楼梦*.md"))
    text = src.read_text(encoding="utf-8")
    out = CORPUS / "hlm"
    out.mkdir(parents=True, exist_ok=True)

    count = 0
    for part in re.split(r"^# ", text, flags=re.M):
        # 支持合回标题"第十七回至十八回"(存为首回编号)
        m = re.match(r"第([零〇○一二三四五六七八九十百\d]+)回(?:至([零〇○一二三四五六七八九十百\d]+)回)?\s", part)
        if not m:
            continue  # 序言/凡例/注释等非正文章节
        n = chapter_num(m.group(1))
        header, _, body = part.partition("\n")
        # 校注本残留:
        #   1. 章末注释条目行,形如 "[](part0010.html#w86) 麻屣鹑衣——麻屣:麻鞋…",整行删除
        #   2. 正文内注释锚点链接 [](part0010.html#m54) 与 markdown 链接
        #   3. 注释编号、分隔线、章末注释区块
        header = re.sub(r"\[+[^(\]]*\]+\([^)]*\)", "", header)
        body = "\n".join(
            line for line in body.splitlines()
            if not re.match(r"\s*\[+[^(]*\]\([^)]*#w\d+\)", line)
        )
        body = re.sub(r"\[+[^(\]]*\]+\([^)]*\)", "", body)
        body = re.sub(r"\[\d+\]", "", body)
        body = re.sub(r"^xml version=[^\n]*$", "", body, flags=re.M)
        body = re.sub(r"^---+$", "", body, flags=re.M)
        body = re.split(r"\n#{1,3}\s*(?:注释|校记)", body)[0]
        body = re.sub(r"\n{3,}", "\n\n", body).strip()
        (out / f"c{n:03d}.txt").write_text(header.strip() + "\n\n" + body, encoding="utf-8")
        count += 1
    print(f"hlm: {count} chapters")


def extract_zhz():
    src = next(TEMP.glob("*ZhenHuan*.epub"))
    book = epub.read_epub(str(src))
    out = CORPUS / "zhz"
    out.mkdir(parents=True, exist_ok=True)

    chapters = {}
    current = None
    # spine 顺序遍历文档项
    docs = {item.get_name(): item for item in book.get_items_of_type(ITEM_DOCUMENT)}
    for idref, _ in book.spine:
        item = book.get_item_with_id(idref)
        if item is None or item.get_name() not in docs:
            continue
        soup = BeautifulSoup(item.get_content(), "html.parser")
        text = soup.get_text(separator="\n")
        text = "\n".join(line.strip() for line in text.splitlines() if line.strip())
        if not text:
            continue
        m = re.search(r"第(\d+)章", text)
        if m:
            current = int(m.group(1))
            chapters.setdefault(current, [])
        if current is not None:
            chapters[current].append(text)

    for n, parts in sorted(chapters.items()):
        body = re.sub(r"\n{3,}", "\n\n", "\n\n".join(parts)).strip()
        (out / f"c{n:03d}.txt").write_text(body, encoding="utf-8")
    print(f"zhz: {len(chapters)} chapters")


if __name__ == "__main__":
    which = sys.argv[1] if len(sys.argv) > 1 else "all"
    if which in ("all", "hlm"):
        extract_hlm()
    if which in ("all", "zhz"):
        extract_zhz()
