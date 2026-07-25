# Stack A Feature-Gate Smoke (Novels Repo)

- Run ID: 20260725-104936-059
- Created: 2026-07-25
- Owner: director (accountable) / eng (responsible)
- Change class: normal
- Security flags: none

## Goal

Prove the Stack A feature-gate lifecycle (pm -> eng -> reviewer -> qa) works end-to-end on the novels repository with **zero product-code changes**. This is a doc-only smoke artifact that confirms IntentGate, evidence gates, and phase transitions are wired correctly on a real business repo, and that any Director can spawn the pipeline and verify it in minutes.

## Scope

Exactly two in-scope paths:

1. `docs/superpowers/specs/2026-07-25-stack-a-feature-gate-smoke.md` (this file).
2. `AGENTS.md` - one new pointer line under `## Commands（命令）`:
   `- Stack A process specs: docs/superpowers/specs/`

No other files are modified by this smoke.

## Non-goals

- Any change under `src/`, `tests/`, `Cargo.toml`, `Cargo.lock`, or `assets/`.
- Fixing or refactoring existing WIP product work (BM25 ranking, assembly readability, `main.rs`).
- Adding new runtime behavior, CLI flags, database schema, or vocabulary entries.
- Plan-review hop, explore wave, or E2E prose generation.
- Committing secrets or modifying `.env`, `novels.db`, `corpus/`, or `assets/distilled/`.
- Mega rewrites of `AGENTS.md`.

## Acceptance

1. This spec file exists and contains sections: Goal, Scope, Non-goals, Acceptance, Security, Handoff.
2. `AGENTS.md` `## Commands` section contains exactly one new pointer line: `- Stack A process specs: docs/superpowers/specs/`.
3. No files outside the two in-scope paths are modified by this run.
4. `cargo fmt --all -- --check` exits 0 (formatting gate; doc-only change).
5. `cargo check --all-targets` exits 0 (compile gate; confirms no accidental product-code inclusion).

## Security

Security flags: **none**. No secrets, credentials, or sensitive paths are touched. `.env`, `novels.db`, `corpus/`, and `assets/distilled/` are out of scope and frozen.

## Handoff

- **eng**: Write only the two in-scope paths above. Use surgical `search_replace` for the AGENTS.md line (no whole-file rewrite). Do not touch `src/**`, `tests/**`, `Cargo.toml`, or any other AGENTS.md section. Run `cargo fmt --all -- --check` and `cargo check --all-targets`; report exit codes in `02-eng.out.md`.
- **reviewer**: Verify scope is exactly two doc paths; confirm no product-code drift; confirm AGENTS.md change is one line only.
- **qa**: Re-run the five Acceptance commands and confirm exit codes; verify the spec contains all six required sections.
- **Constraints**: If `fmt`/`check` fail due to unrelated pre-existing dirty WIP (e.g. `src/main.rs`, `src/prose/assembly.rs`), document the pre-existing failure in the OUT and do not attempt to fix product code; the doc deliverables remain complete.
