# AI Maintenance Playbook Design

## Purpose

Expand the repository-root `AGENTS.md` into the single operational entrypoint for AI agents maintaining `novels`. The playbook must let an agent identify a task's ownership boundary, make a minimal safe change, and run the correct verification without reconstructing project behavior from source files.

## Scope

Append an `AI Maintenance Playbook` section to the existing `AGENTS.md`. Preserve existing repository, runtime, architecture, data-flow, and environment guidance as the authoritative project description.

The added playbook will cover:

- A start-here procedure that names authoritative files, required preflight checks, and dirty-worktree handling.
- Task playbooks for domain models, database repositories and schema, scene orchestration, LLM integration, vocabulary, and corpus distillation.
- Runtime operations for the main binary, the `distill` binary, mock versus DeepSeek execution, database location, and asset loading.
- Quality gates mapping change categories to formatting, compile, test, and lint commands.
- Data and safety rules for credentials, source corpus, generated distilled vocabulary, and user-created uncommitted material.
- A change checklist that prevents cross-layer contract, persistence, validation, and test omissions.
- Troubleshooting for known Windows dependency prerequisites, missing API keys, and malformed vocabulary or corpus inputs.

## Non-Goals

- No runtime, schema, API, dependency, test, or tooling change.
- No replacement of existing architecture guidance.
- No generic Rust tutorial or human onboarding guide.
- No mutation, cleanup, or normalization of existing uncommitted assets.

## Document Structure

The new section is task-oriented and follows existing `AGENTS.md` terminology:

1. `Start Here`
2. `Task Playbooks`
3. `Runtime Operations`
4. `Quality Gates`
5. `Data and Safety Rules`
6. `Change Checklist`
7. `Troubleshooting`

Each task playbook states:

- Primary files and dependent files.
- Invariants that must remain true.
- Required tests or checks.
- Boundaries that must not be crossed without an explicit request.

## Information Sources

The manual will derive its claims from the checked-in source, tests, `Cargo.toml`, existing `AGENTS.md`, and current binary/tool entrypoints. It will distinguish committed project behavior from user worktree artifacts.

## Acceptance Criteria

An AI agent reading only `AGENTS.md` can:

- Determine whether a task belongs to `models`, `db`, `scene`, `llm`, `vocab`, or corpus-distillation tooling.
- Identify mock and production runtime paths and required environment variables.
- Select appropriate verification commands and recognize the known Lance/protoc baseline issue.
- Avoid committing credentials or overwriting source corpus, generated assets, and unrelated user work.
- Recognize when a proposed change crosses a public contract, persistence boundary, or LLM validation guardrail.

## Verification

Review the rendered Markdown for duplicate or contradictory instructions, then run `cargo fmt --all -- --check` to ensure repository formatting remains clean. No behavioral test is required because the implementation changes documentation only.
