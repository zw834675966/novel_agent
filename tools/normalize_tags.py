# -*- coding: utf-8 -*-
"""Normalize tags in assets/distilled/*.yaml against controlled facets.

Reads assets/tag_facets.yaml and rewrites entry tags so that:
  1. Book tags (hlm / zhz) are always kept (and ensured from filename stem).
  2. drop_tokens are stripped (exact match).
  3. Moods are kept only when in facets.moods (after optional mood_aliases).
  4. Settings are kept only when in facets.settings (after optional setting_aliases).
  5. Character-like / free descriptive tags are kept when not drop_tokens and
     not an unlisted mood/setting alias target-class only.

Default is dry-run (report only). Pass --write to rewrite files.
Does not modify any file without --write.

用法:
  python tools/normalize_tags.py
  python tools/normalize_tags.py --file assets/distilled/hlm-c001.yaml
  python tools/normalize_tags.py --write
  python tools/normalize_tags.py --file assets/distilled/hlm-c001.yaml --write

全库 rewrite 需用户确认后再加 --write；本工具默认只 dry-run。
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

try:
    import yaml
except ImportError:  # pragma: no cover
    print("error: PyYAML required (pip install pyyaml)", file=sys.stderr)
    sys.exit(2)

REPO_ROOT = Path(__file__).resolve().parent.parent
DEFAULT_FACETS = REPO_ROOT / "assets" / "tag_facets.yaml"
DEFAULT_DIR = REPO_ROOT / "assets" / "distilled"

STEM_RE = re.compile(r"^(.+)-c(\d+)$")
# Match a full tags: [...] line with leading indentation preserved.
TAGS_LINE_RE = re.compile(r"^(\s*tags:\s*)(\[.*\])\s*$")


def load_facets(path: Path) -> dict:
    data = yaml.safe_load(path.read_text(encoding="utf-8")) or {}
    books = frozenset(str(x) for x in (data.get("books") or []))
    moods = frozenset(str(x) for x in (data.get("moods") or []))
    settings = frozenset(str(x) for x in (data.get("settings") or []))
    drop = frozenset(str(x) for x in (data.get("drop_tokens") or []))
    mood_aliases = {
        str(k): str(v) for k, v in (data.get("mood_aliases") or {}).items()
    }
    setting_aliases = {
        str(k): str(v) for k, v in (data.get("setting_aliases") or {}).items()
    }
    if not books:
        print(f"error: {path} has empty books", file=sys.stderr)
        sys.exit(2)
    return {
        "books": books,
        "moods": moods,
        "settings": settings,
        "drop_tokens": drop,
        "mood_aliases": mood_aliases,
        "setting_aliases": setting_aliases,
    }


def book_from_stem(stem: str, books: frozenset[str]) -> str | None:
    m = STEM_RE.match(stem)
    if not m:
        return None
    book = m.group(1)
    return book if book in books else None


def normalize_tags(
    tags: list[str],
    facets: dict,
    book: str | None,
) -> list[str]:
    """Return normalized tag list (order: book first, then stable unique others)."""
    books = facets["books"]
    moods = facets["moods"]
    settings = facets["settings"]
    drop = facets["drop_tokens"]
    mood_aliases = facets["mood_aliases"]
    setting_aliases = facets["setting_aliases"]

    # Known mood/setting vocab after alias expansion (for classification).
    # Aliases map synonym → canonical; synonyms are also "mood/setting-class".
    mood_class = set(moods) | set(mood_aliases.keys()) | set(mood_aliases.values())
    setting_class = (
        set(settings) | set(setting_aliases.keys()) | set(setting_aliases.values())
    )

    kept: list[str] = []
    seen: set[str] = set()

    def add(t: str) -> None:
        if t not in seen:
            seen.add(t)
            kept.append(t)

    # 1) Always keep book tag (prefer filename stem, else any book tag in list).
    if book:
        add(book)
    else:
        for t in tags:
            if t in books:
                add(t)
                break

    for raw in tags:
        t = str(raw).strip()
        if not t:
            continue
        if t in books:
            # Already ensured; avoid duplicates / wrong-book noise if book fixed.
            if book and t != book:
                continue
            add(t)
            continue
        if t in drop:
            continue

        # Optional alias remap for moods / settings.
        if t in mood_aliases:
            t = mood_aliases[t]
        elif t in setting_aliases:
            t = setting_aliases[t]

        # 3–4) Mood / setting only if in controlled facets.
        if t in mood_class:
            if t in moods:
                add(t)
            # Unlisted mood-class (alias that mapped outside, or bare synonym left
            # without landing in moods) → strip.
            continue
        if t in setting_class:
            if t in settings:
                add(t)
            continue

        # 5) Character-like / free descriptive: keep.
        add(t)

    return kept


def format_tags_line(indent_and_key: str, tags: list[str]) -> str:
    """Rebuild `    tags: ["a", "b"]` with double-quoted items."""
    inner = ", ".join(f'"{t}"' for t in tags)
    return f"{indent_and_key}[{inner}]"


def parse_tags_literal(literal: str) -> list[str]:
    """Parse a YAML flow-sequence literal like `["hlm", "封氏"]`."""
    value = yaml.safe_load(literal)
    if value is None:
        return []
    if not isinstance(value, list):
        raise ValueError(f"tags value is not a list: {literal!r}")
    return [str(x) for x in value]


def process_file(
    path: Path,
    facets: dict,
    write: bool,
) -> tuple[int, int, list[tuple[str, list[str], list[str]]]]:
    """Process one YAML file. Returns (entries, changed, samples)."""
    text = path.read_text(encoding="utf-8")
    lines = text.splitlines(keepends=True)
    book = book_from_stem(path.stem, facets["books"])

    entries = 0
    changed = 0
    samples: list[tuple[str, list[str], list[str]]] = []
    out: list[str] = []
    dirty = False

    for line in lines:
        # Strip newline for match; restore later.
        if line.endswith("\r\n"):
            body, nl = line[:-2], "\r\n"
        elif line.endswith("\n"):
            body, nl = line[:-1], "\n"
        else:
            body, nl = line, ""

        m = TAGS_LINE_RE.match(body)
        if not m:
            out.append(line)
            continue

        entries += 1
        prefix, literal = m.group(1), m.group(2)
        try:
            old_tags = parse_tags_literal(literal)
        except Exception as e:  # noqa: BLE001 — report and keep line
            print(f"  WARN [{path.name}] unparseable tags: {body.strip()} ({e})")
            out.append(line)
            continue

        new_tags = normalize_tags(old_tags, facets, book)
        if new_tags != old_tags:
            changed += 1
            dirty = True
            if len(samples) < 12:
                samples.append((body.strip(), old_tags, new_tags))
            new_body = format_tags_line(prefix, new_tags)
            out.append(new_body + nl)
        else:
            out.append(line)

    if write and dirty:
        path.write_text("".join(out), encoding="utf-8", newline="")

    return entries, changed, samples


def iter_targets(file_arg: str | None, dist_dir: Path) -> list[Path]:
    if file_arg:
        p = Path(file_arg)
        if not p.is_absolute():
            p = (REPO_ROOT / p).resolve() if not p.exists() else p.resolve()
        if not p.exists():
            # try relative to repo
            alt = (REPO_ROOT / file_arg).resolve()
            if alt.exists():
                p = alt
            else:
                print(f"error: file not found: {file_arg}", file=sys.stderr)
                sys.exit(2)
        return [p]
    if not dist_dir.is_dir():
        print(f"error: distilled dir not found: {dist_dir}", file=sys.stderr)
        sys.exit(2)
    return sorted(dist_dir.glob("*.yaml"))


def main(argv: list[str] | None = None) -> int:
    ap = argparse.ArgumentParser(
        description="Normalize distilled YAML tags against assets/tag_facets.yaml"
    )
    ap.add_argument(
        "--facets",
        type=Path,
        default=DEFAULT_FACETS,
        help=f"path to facets YAML (default: {DEFAULT_FACETS})",
    )
    ap.add_argument(
        "--dir",
        type=Path,
        default=DEFAULT_DIR,
        help=f"distilled directory (default: {DEFAULT_DIR})",
    )
    ap.add_argument(
        "--file",
        type=str,
        default=None,
        help="single file (path or relative to repo root); skip --dir",
    )
    ap.add_argument(
        "--write",
        action="store_true",
        help="rewrite files in place (default: dry-run only)",
    )
    args = ap.parse_args(argv)

    facets_path = args.facets
    if not facets_path.is_absolute():
        facets_path = (REPO_ROOT / facets_path).resolve()
    if not facets_path.exists():
        print(f"error: facets not found: {facets_path}", file=sys.stderr)
        return 2

    facets = load_facets(facets_path)
    dist_dir = args.dir if args.dir.is_absolute() else (REPO_ROOT / args.dir)
    targets = iter_targets(args.file, dist_dir)

    if not targets:
        print("no YAML files to process")
        return 0

    mode = "WRITE" if args.write else "DRY-RUN"
    print(f"normalize_tags [{mode}] facets={facets_path}")
    print(
        f"  books={sorted(facets['books'])} "
        f"moods={len(facets['moods'])} settings={len(facets['settings'])} "
        f"drop={sorted(facets['drop_tokens'])}"
    )
    print(f"  files={len(targets)}")

    total_entries = 0
    total_changed = 0
    files_changed = 0

    for path in targets:
        entries, changed, samples = process_file(path, facets, write=args.write)
        total_entries += entries
        total_changed += changed
        if changed:
            files_changed += 1
            rel = path
            try:
                rel = path.relative_to(REPO_ROOT)
            except ValueError:
                pass
            action = "rewrote" if args.write else "would change"
            print(f"  {action} {rel}: {changed}/{entries} tag lines")
            for raw, old, new in samples:
                print(f"    - {old} -> {new}")

    print(
        f"summary: entries={total_entries} changed_lines={total_changed} "
        f"files_with_changes={files_changed}/{len(targets)} mode={mode}"
    )
    if not args.write:
        print(
            "note: dry-run only; pass --write after review to rewrite. "
            "Full-library rewrite is not forced."
        )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
