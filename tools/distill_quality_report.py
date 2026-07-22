# -*- coding: utf-8 -*-
"""蒸馏库 KPI 报告。只读统计，不改写任何 YAML。

用法:
  python tools/distill_quality_report.py
  python tools/distill_quality_report.py --dir assets/distilled --json out.json
  python tools/distill_quality_report.py --dir assets/distilled --verbatim-sample 50

KPI:
  entries, cat_pct, dialogue_proxy_rate, short_le10_rate,
  unique_content_tags, verbatim_sample_pass_rate
"""
from __future__ import annotations

import argparse
import collections
import json
import random
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    print("error: PyYAML required (pip install pyyaml)", file=sys.stderr)
    sys.exit(2)

# Repo root = parent of tools/
REPO_ROOT = Path(__file__).resolve().parent.parent

# Book-source tags are not "content" tags for uniqueness KPIs.
BOOK_TAGS = frozenset({"hlm", "zhz"})

# Dialogue proxy: same heuristic as gap analysis / validate_fragments plan.
DIALOGUE_INLINE_RE = re.compile(r"(道|说|问道|笑道|便道)[：“\"]")
DIALOGUE_PREFIXES = ("笑道", "便道", "说道", "问道")

# Filename stem: book-cNNN  e.g. hlm-c001, zhz-c050
STEM_RE = re.compile(r"^(.+)-c(\d+)$")


def norm(s: str) -> str:
    """Whitespace-normalized form used for verbatim substring checks."""
    return re.sub(r"\s", "", s)


def unescape(s: str) -> str:
    return s.replace('\\"', '"').replace("\\\\", "\\")


def looks_like_dialogue(text: str) -> bool:
    if text.startswith(DIALOGUE_PREFIXES):
        return True
    return bool(DIALOGUE_INLINE_RE.search(text))


def parse_stem(stem: str) -> tuple[str, str] | None:
    """Return (book, chap_digits) or None if stem is not book-cNNN."""
    m = STEM_RE.match(stem)
    if not m:
        return None
    return m.group(1), m.group(2)


def load_entries(yaml_path: Path) -> list[dict]:
    """Parse one distilled YAML into flat entry dicts.

    Each item: {file, stem, cat, key, text, tags}
    """
    raw = yaml_path.read_text(encoding="utf-8")
    data = yaml.safe_load(raw) or {}
    out: list[dict] = []
    if not isinstance(data, dict):
        return out
    for cat, ents in data.items():
        if not isinstance(ents, dict):
            continue
        for key, e in ents.items():
            if not isinstance(e, dict):
                continue
            text = e.get("text", "") or ""
            if not isinstance(text, str):
                text = str(text)
            tags = e.get("tags") or []
            if not isinstance(tags, list):
                tags = []
            tags = [str(t) for t in tags]
            out.append(
                {
                    "file": yaml_path.name,
                    "stem": yaml_path.stem,
                    "cat": str(cat),
                    "key": str(key),
                    "text": text,
                    "tags": tags,
                }
            )
    return out


def corpus_path_for(stem: str, corpus_root: Path) -> Path | None:
    parsed = parse_stem(stem)
    if not parsed:
        return None
    book, chap = parsed
    # Keep chapter zero-padding as in stem (c001, c050, …).
    return corpus_root / book / f"c{chap}.txt"


def sample_verbatim(
    entries: list[dict],
    sample_n: int,
    corpus_root: Path,
    rng: random.Random,
) -> dict:
    """Random sample entries and check whitespace-normalized substring in corpus.

    Missing corpus files are skipped gracefully (counted separately, not fail).
    """
    if sample_n <= 0 or not entries:
        return {
            "requested": sample_n,
            "sampled": 0,
            "checked": 0,
            "pass": 0,
            "fail": 0,
            "skipped_missing_corpus": 0,
            "skipped_bad_stem": 0,
            "pass_rate": None,
            "fail_examples": [],
        }

    n = min(sample_n, len(entries))
    sample = rng.sample(entries, n)

    # Cache normalized corpus text by path.
    corpus_cache: dict[Path, str | None] = {}
    checked = 0
    passed = 0
    failed = 0
    skip_missing = 0
    skip_stem = 0
    fail_examples: list[dict] = []

    for ent in sample:
        cpath = corpus_path_for(ent["stem"], corpus_root)
        if cpath is None:
            skip_stem += 1
            continue
        if cpath not in corpus_cache:
            if not cpath.is_file():
                corpus_cache[cpath] = None
            else:
                try:
                    corpus_cache[cpath] = norm(cpath.read_text(encoding="utf-8"))
                except OSError:
                    corpus_cache[cpath] = None
        src = corpus_cache[cpath]
        if src is None:
            skip_missing += 1
            continue

        text = unescape(ent["text"])
        ntext = norm(text)
        checked += 1
        if ntext and ntext in src:
            passed += 1
        else:
            failed += 1
            if len(fail_examples) < 10:
                fail_examples.append(
                    {
                        "file": ent["file"],
                        "key": ent["key"],
                        "text_preview": text[:40],
                        "corpus": str(cpath.relative_to(corpus_root.parent))
                        if corpus_root.parent in cpath.parents
                        else str(cpath),
                    }
                )

    pass_rate = round(100.0 * passed / checked, 2) if checked else None
    return {
        "requested": sample_n,
        "sampled": n,
        "checked": checked,
        "pass": passed,
        "fail": failed,
        "skipped_missing_corpus": skip_missing,
        "skipped_bad_stem": skip_stem,
        "pass_rate": pass_rate,
        "fail_examples": fail_examples,
    }


def build_report(
    root: Path,
    corpus_root: Path,
    verbatim_sample: int,
    seed: int,
) -> dict:
    files = sorted(root.glob("*.yaml"))
    entries: list[dict] = []
    for fp in files:
        entries.extend(load_entries(fp))

    total = len(entries)
    cat_counts: collections.Counter[str] = collections.Counter()
    content_tag_counter: collections.Counter[str] = collections.Counter()
    dialogue_n = 0
    short_n = 0
    no_content_tags_n = 0

    for ent in entries:
        cat_counts[ent["cat"]] += 1
        text = ent["text"]
        if len(text) <= 10:
            short_n += 1
        if looks_like_dialogue(text):
            dialogue_n += 1
        content_tags = [t for t in ent["tags"] if t not in BOOK_TAGS]
        if not content_tags:
            no_content_tags_n += 1
        for tg in content_tags:
            content_tag_counter[tg] += 1

    if total:
        cat_pct = {
            c: round(100.0 * cat_counts[c] / total, 1)
            for c in sorted(cat_counts)
        }
        dialogue_proxy_rate = round(100.0 * dialogue_n / total, 2)
        short_le10_rate = round(100.0 * short_n / total, 2)
        no_content_tags_rate = round(100.0 * no_content_tags_n / total, 2)
    else:
        cat_pct = {}
        dialogue_proxy_rate = 0.0
        short_le10_rate = 0.0
        no_content_tags_rate = 0.0

    rng = random.Random(seed)
    verbatim = sample_verbatim(entries, verbatim_sample, corpus_root, rng)

    report = {
        "dir": str(root),
        "files": len(files),
        "entries": total,
        "cat_counts": dict(sorted(cat_counts.items())),
        "cat_pct": cat_pct,
        "dialogue_proxy_rate": dialogue_proxy_rate,
        "dialogue_count": dialogue_n,
        "short_le10_rate": short_le10_rate,
        "short_le10_count": short_n,
        "unique_content_tags": len(content_tag_counter),
        "no_content_tags_rate": no_content_tags_rate,
        "top_content_tags": content_tag_counter.most_common(20),
        "verbatim_sample": verbatim,
        "verbatim_sample_pass_rate": verbatim["pass_rate"],
        "seed": seed,
    }
    return report


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Distilled vocab quality KPI report (read-only; never rewrites YAML)."
    )
    ap.add_argument(
        "--dir",
        default="assets/distilled",
        help="Directory of distilled *.yaml (default: assets/distilled)",
    )
    ap.add_argument(
        "--json",
        dest="json_out",
        default=None,
        help="Optional path to write JSON report",
    )
    ap.add_argument(
        "--verbatim-sample",
        type=int,
        default=50,
        help="Number of random entries to check against corpus (default: 50)",
    )
    ap.add_argument(
        "--corpus",
        default=None,
        help="Corpus root (default: <repo>/corpus)",
    )
    ap.add_argument(
        "--seed",
        type=int,
        default=7,
        help="RNG seed for verbatim sample (default: 7)",
    )
    args = ap.parse_args(argv)

    root = Path(args.dir)
    if not root.is_absolute():
        # Prefer CWD-relative; fall back to repo-relative for convenience.
        if not root.exists():
            cand = REPO_ROOT / root
            if cand.exists():
                root = cand
    root = root.resolve()

    if not root.is_dir():
        print(f"error: directory not found: {root}", file=sys.stderr)
        return 1

    if args.corpus:
        corpus_root = Path(args.corpus)
        if not corpus_root.is_absolute():
            corpus_root = (Path.cwd() / corpus_root).resolve()
    else:
        corpus_root = (REPO_ROOT / "corpus").resolve()

    report = build_report(
        root=root,
        corpus_root=corpus_root,
        verbatim_sample=args.verbatim_sample,
        seed=args.seed,
    )

    text = json.dumps(report, ensure_ascii=False, indent=2)
    print(text)

    if args.json_out:
        out = Path(args.json_out)
        out.parent.mkdir(parents=True, exist_ok=True)
        out.write_text(text + "\n", encoding="utf-8")
        print(f"\n# wrote {out}", file=sys.stderr)

    return 0


if __name__ == "__main__":
    sys.exit(main())
