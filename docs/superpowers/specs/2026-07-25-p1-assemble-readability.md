---
run_id: 20260725-101650-616
seq: spec
role: pm (locked design) -> eng (file)
title: P1 Assemble readability
created: 2026-07-25
status: implemented
source: .agent-org/runs/20260725-101650-616/01-pm.out.md
---

# P1 Assemble readability

## Problem

`assemble_beat_descriptions` joined injected 原著 fragments with `""`, gluing
quotes into an unreadable brick (e.g. `夜凉如水血迹脚步声…`). The desc/action
boundary had no separator either. KPI `quote_chars` counted the joined string,
which would have inflated density once separators were introduced.

## Solution (locked J1–J6)

1. **J1** Quotes within a beat join with fullwidth comma `，` (U+FF0C).
2. **J2** Between non-empty desc and non-empty action: insert `。` (U+3002)
   unless desc already ends with `。！？…` or ASCII `.!?`.
3. **J3** Empty desc -> action only (unchanged). Empty action -> desc only,
   no forced trailing `。`.
4. **J4** `quote_chars` sums injected quote text char counts only
   (`quotes.iter().map(count).sum()`), excluding `，`/`。` separators.
5. **J5** `unverified_quotes` still exact `text.contains(quote)` per injected
   fragment; the `quotes` vec holds raw fragments so separators never mutate
   quote strings.
6. **J6** No soft-retry / hard-fail on KPI this hop (P0 telemetry stands).

## Non-goals

BM25 / Chinese tokenizer; rewriting distilled excerpt text; hard-fail on
`low_quote_density`; clap / sqlx migrate / error enum changes.

## Test plan

- Updated golden strings: `orders_all_eight_categories`,
  `strips_unknown_and_malformed_refs`, `rejects_non_participant_pov`.
- New: `multi_quote_join_uses_fullwidth_comma` (A1),
  `desc_action_separator_inserts_period` (A2),
  `desc_ending_in_terminal_punct_skips_extra_period` (J2 edge),
  `quote_chars_exclude_separators` (A5).
- Existing `verify_provenance_zero_for_clean_assemble` covers A3
  (single-quote + action, `unverified_quotes == 0`).

## Acceptance (from 01-pm.out.md)

| ID | Criterion | Verify |
|----|-----------|--------|
| A1 | Multi-quote beat joins with `，` | `multi_quote_join_uses_fullwidth_comma` + `orders_all_eight_categories` |
| A2 | Non-empty desc + action -> `。` between (desc lacks terminal punct) | `desc_action_separator_inserts_period` |
| A3 | Single-quote + action readable; `unverified_quotes=0` | `verify_provenance_zero_for_clean_assemble` |
| A4 | Strip/reject/action_only semantics preserved | `cargo test --lib assembly::` |
| A5 | `quote_chars` excludes separators | `quote_chars_exclude_separators` |
| A6 | `cargo test --lib prose::` and `--test prose_test` green | exit 0 |
