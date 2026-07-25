---
run_id: 20260725-095002-712
seq: spec
role: pm (spec body) -> eng (file)
title: P0 Hollow-fix
created: 2026-07-25
status: implemented
source: .agent-org/runs/20260725-095002-712/01-pm.out.md
---

# P0 Hollow-fix

## Problem

Hollow claims: docs stale; narrate ignores derive semantics; main mock empty sensations; KPI named like quality gate but observational.

## Solution

1. Doc alignment only where code already correct.
2. Extend `build_prompt` to serialize characters' personality/skills and derivations' memory/plot (truncate free text e.g. 120/60 chars reuse text_guard caps).
3. Main mock: fill sensations with known ids from base vocab; ensure narrate still runs.
4. Comments on `ProseQualityReport` / MIN_QUOTE: "telemetry, not gate".

## Non-goals

BM25 rewrite; literary join; schema migrate; distill key format; Cargo.toml lance surgery.

## Test plan

- New: `prompt_includes_personality_memory_and_plot` in `src/prose/rig_impl.rs` tests
- Existing assemble low_density still Ok
- prose_test / scene_test as needed for mock non-empty

## Acceptance (from 01-pm.out.md)

| ID | Criterion | Verify |
|----|-----------|--------|
| A1 | AGENTS.md: no "rig-lancedb is declared" as direct dep; note optional/transitive/historical only if needed; distill sense-nested -> `sense.key`; 8 dims; quality metrics observational | grep AGENTS + read sections |
| A2 | `RigProseGenerator` `build_prompt` contains personality, skills, memory content, plot reason when present on request | unit test on prompt string |
| A3 | `main` mock sense path uses >=1 valid vocab id present in `assets/vocab.yaml` | code review + `cargo test` if testable helper, else e2e mock fixture |
| A4 | Assemble still does **not** error solely because `low_quote_density` | existing low_quote_density unit test still Ok |
| A5 | Spec file `docs/superpowers/specs/2026-07-25-p0-hollow-fix.md` created from this body | path exists |
| A6 | Targeted tests: `cargo test --test prose_test` and prompt unit in prose; `cargo test --lib` assembly tests if any; no full lance drama | exit 0 on targeted |
