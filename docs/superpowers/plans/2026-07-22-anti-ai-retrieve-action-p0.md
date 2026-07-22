# Anti-AI Retrieve + Action Clamp P0 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Close critique gaps D2/D3 (partial) and research P0: **scene/character-aware candidate ranking** (replace pure dictionary top-k) and **programmatic action clamp** (cut AI causal glue + track quote density).

**Architecture:** Keep ID-only selection and verbatim assemble. Add zero-dependency **lexical scoring** over candidate `text`/`tags` using character name + scene event + selected tags as query. After LLM prose returns, **sanitize action** (length + causal filler strip) and compute **quote density** on assembled output. No embeddings, no vector DB.

**Tech Stack:** Rust 2024 · `src/vocab/loader.rs` · `src/scene/service.rs` · `src/prose/assembly.rs` · existing tests

**Sources:** `novels-conclusion-critique` D2/D3/D4/D7 · `2026-07-22-original-material-fill-research.md` · shipped runtime-closure

## Global Constraints

- Do not change DeepSeek provider / model constant.
- Do not break `validate_selection` or derivation transactions.
- Caps remain DEFAULT_PER_SENSE_CAP=24, DEFAULT_TOTAL_CAP=96.
- Ranking must be **deterministic** (score desc, then id asc).
- Do not rewrite distilled YAML or corpus.
- Do not stage unrelated dirty WIP (Cargo.toml / models).

---

### Task 1: Ranked candidate selection

**Files:**
- Modify: `src/vocab/loader.rs`
- Modify: `src/vocab/mod.rs` (exports if needed)
- Modify: `src/scene/service.rs`
- Test: `tests/vocab_test.rs`

**Interfaces:**
- Produces: `Vocab::candidates_ranked_limited(&self, selected: &[String], query_terms: &[String], per_sense: usize, total_max: usize) -> Vec<VocabularyCandidate>`
- Produces: `fn score_candidate(c: &VocabularyCandidate, query_terms: &[String], selected: &[String]) -> i64` (crate or private)

- [x] **Step 1–4:** Implement TDD ranked candidates (see code in repo)
- [x] **Step 5:** Wire `service.rs` with character name + scene objective_event + selected_tags as query_terms

### Task 2: Action sanitize + quote density

**Files:**
- Modify: `src/prose/assembly.rs`
- Modify: `src/prose/mod.rs` if re-exports
- Test: `tests/prose_test.rs`

**Interfaces:**
- Produces: `AssembledProse { ..., quote_chars, total_chars }` + `quote_density() -> f64`
- Produces: `sanitize_action(action: &str) -> String` with MAX_ACTION_CHARS=80 and causal filler strip

- [x] Steps: TDD sanitize + density; assemble always sanitizes action

### Task 3: Docs + knowledge base

- [x] AGENTS.md note on ranked candidates + action clamp
- [x] Knowledge base critique P0 status
- [x] Mark this plan checklist complete after VERIFY

## Verification

```
cargo test --test vocab_test
cargo test --test prose_test
cargo test --test scene_test
cargo test --test e2e
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
```

## Deviations

- Inline execution (not subagent) for speed; scope is 2 modules + tests.
- Lexical scoring only (no BM25 crate / embed) per YAGNI.
