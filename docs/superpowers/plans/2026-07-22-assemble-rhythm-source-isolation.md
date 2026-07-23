# Assemble Rhythm + Book-Source Isolation (Platform reverse N1/N2)

> **Status:** Implemented 2026-07-22

**Goal:** Reduce reader/platform AI tells: bare quote list texture (P2) and cross-book voice mix (P3).

## Done
- [x] `join_quotes_with_rhythm` / `join_desc_action` in `src/prose/assembly.rs`
- [x] Dominant `hlm`/`zhz` filter within beat; base lemmas kept
- [x] Expand `text_guard` explanatory fillers
- [x] Unit tests + Agents.md + reverse research status update

## Verify
```
cargo test --lib --test prose_test --test e2e --test scene_test --test vocab_test
```
