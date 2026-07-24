# BM25 Retrieve + Post-Assemble Provenance Plan

> **Status:** Implemented 2026-07-22

**Goal:** Close research §5a open items: **BM25 candidate ranking** (replace boolean lexical-only) and **post-assemble quote provenance scan + quality report**.

**Architecture:** Zero new deps. BM25 over candidate pool (text+tags docs, whole-term substring TF). Assemble records injected quotes and re-checks `text.contains`; expose `ProseQualityReport` (density / rates / low_quote_density flag only).

## Tasks

- [x] Task 1: `Bm25Scores` + `candidates_ranked_limited` uses BM25 + voice boost; vocab tests
- [x] Task 2: `unverified_quotes` + `ProseQualityReport` on `AssembledProse`; main log; prose tests
- [x] Task 3: Sync research §5a, vault summary, AGENTS.md

## Verification

```
cargo test --test vocab_test
cargo test --test prose_test
cargo test --test e2e --test scene_test
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```
