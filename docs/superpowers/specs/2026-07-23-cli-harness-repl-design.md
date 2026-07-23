# CLI Harness REPL Design

## Status

Approved design. Implementation has not started.

## Goal

Add a **command-line Harness layer** so authors can drive the existing novel engine from a terminal: create characters and scenes, run derivation and narration, and inspect results — without degrading output quality relative to the current `StoryService` + vocabulary + prose assembly pipeline.

This is a **story-operation REPL and one-shot CLI**, not free-form LLM chat.

## Product Decisions (locked)

| Decision | Choice |
|----------|--------|
| Interaction model | Story-operation commands (not free Chat) |
| Entry / coexistence | Subcommands: `repl` + one-shot; bare `novels` keeps current demo + API behavior |
| MVP scope | Minimal closed loop: character, scene, derive, narrate, show |
| Session context | Explicit `use scene` / `use character` |
| Language | English subcommands + Chinese human-readable results |
| Call path | In-process `StoryService` (not HTTP client) |
| Quality | No degradation; same harness gates as production path |
| LLM constraint model | Harness Engineering: restricted action space + pre/post gates + structured observation |

## Quality Non-Degradation (hard constraint)

The CLI must **not**:

- Bypass `StoryService` to call the LLM for free-form description
- Skip `validate_selection`, `text_guard`, or prose assembly / quote re-scan
- Default to Mock without a clear warning, or disable distilled merge by default
- Widen candidate sets, loosen quote checks, or expose `--skip-validate` / `--raw-llm` / `--free-prose`
- Inject REPL chat history or user free-text into derive/narrate prompts

**Only legal path:**

```text
User input (clap / REPL)
  → cli parse + session
  → shared bootstrap (Db + load_runtime_vocab + Sense/Prose generators)
  → StoryService::create_* / derive_* / narrate_scene
  → existing structured results + AssembledProse KPIs
  → terminal formatting (optional --json, same data)
```

Success criteria:

1. Same DB, same API key, same scene: CLI derive+narrate uses the same service contracts and guardrails as the non-CLI path (IDs/timestamps may differ; skipping validation or empty assembly must not).
2. Missing `DEEPSEEK_API_KEY` → Mock + explicit `status=warning` (never silent “production quality”).
3. Distilled merge and `NOVELS_SKIP_DISTILLED` / `NOVELS_DISTILLED_DIR` behave as in `main`.
4. Terminal output exposes quote-density and related KPIs; low density remains flag-only (no algorithm change).

## Harness Engineering (constrain the LLM)

CLI is a **Harness shell**. The quality engine remains StoryService + structured extractors + vocab/assembly gates. Harness may only **mirror or tighten** existing gates — never relax them for convenience.

### Dimensions

| Dimension | Meaning here | MVP |
|-----------|--------------|-----|
| Action space | Only story ops | Fixed subcommands; no `chat`, no raw prompt inject |
| Schema-first | Structured contracts only | `SenseGenerator` / `ProseGenerator` + JsonSchema extractors |
| Pre-gates | World-state before LLM | Existence, participation, session `use`, Dead/Absent, etc. via service |
| Post-gates | Output must pass rails | validate + retry, text_guard, strip illegal refs, quote re-scan |
| Observation | Recoverable, auditable output | `status` / `summary` / `artifacts` / `quality` / `next` |
| Recovery | Clear stop + next step | Reuse service one all-empty retry; CLI suggests next command |
| Context budget | No chat pollution | No REPL history into derive/narrate; service owns temporal context |

### Allowed actions (MVP)

```text
character create|list|show
scene create|list|show
use scene | use character | use clear | context
derive | narrate
show derivation | show prose
help | quit
```

Only `derive` and `narrate` invoke the LLM. They are medium macros: full internal orchestration, no user-bypass of sub-steps.

### Forbidden (quality / harness)

- Free multi-turn chat, custom system prompts, user-tunable temperature/seed
- “Skip vocab, generate description freely”
- CLI direct writes of sensations/memories outside service transactions
- Feeding full REPL history into the LLM
- Hiding KPIs or reporting success on Mock as production quality
- `--raw-llm`, `--skip-validate`, `--free-prose`

### Gate pipeline

```text
[Pre]  parse → Session → StoryService world checks
         fail → status=error, no LLM call
[LLM]  structured Extractor only (existing model/seed/schema policy unchanged)
[Post] validate / text_guard / assembly / quote verify
[Obs]  terminal observation block (quality required when assembly ran)
```

### Observation protocol

Every command (especially `derive` / `narrate`) emits:

```text
status: success | warning | error
summary: one-line Chinese summary
artifacts:
  scene_id: ...
  character_id: ...   # when applicable
quality:              # required after narrate / when assembly metrics exist
  quote_density: 0.xx
  stripped_refs: N
  unverified_quotes: N
  low_quote_density: true|false
  action_only_beats: N
next:
  - suggested next command(s)
```

`low_quote_density` and related flags are **report-only**; CLI must not change thresholds or rewrite prose to “fix” density.

### Harness non-goals

- New ReAct planning loop inside CLI
- Auto “quality too low → rewrite prompt → regenerate prose” repair loop (breaks determinism and anti-AI-taste invariants)
- User-configurable relaxation of gates

## Architecture

### Approach

**In-process CLI** (clap + simple REPL stdin loop) calling `StoryService`. Optional reedline polish is post-MVP.

Rejected for MVP:

- Full reedline-first UX (delay)
- CLI as HTTP client to Axum (requires server; weak session; longer failure path)

### Module layout

```text
src/
  main.rs              # clap dispatch: bare run | repl | one-shot
  cli/
    mod.rs
    args.rs            # clap definitions
    repl.rs            # interactive loop + Session
    commands.rs        # invoke StoryService
    format.rs          # Chinese human-readable + KPI / --json
  scene/service.rs     # unchanged quality path
  llm/ prose/ vocab/   # unchanged
```

| Layer | Owns | Must not |
|-------|------|----------|
| `src/cli/` | Parse, REPL, session, formatting | Rewrite LLM contracts or assembly |
| Shared bootstrap | Db, vocab, generators (same as main) | CLI-only “lite” vocab |
| `StoryService` | Sole business orchestration | CLI bypass writes |
| `api/` | Unchanged | CLI must not depend on HTTP |

### Coexistence with bare `cargo run`

| Mode | Behavior |
|------|----------|
| `novels` (no subcommand) | **Unchanged**: demo flow + API on `127.0.0.1:3000` |
| `novels repl` | CLI Harness only; **do not** start HTTP by default |
| `novels <cmd>` | One command, exit; no HTTP |

Adding CLI must not change bare-run derive/narrate semantics or guardrails. Renaming default entry to `serve` is out of scope for this MVP.

## Commands and session

### Session (REPL only)

```text
Session {
  current_scene: Option<SceneId>,
  current_character: Option<CharacterId>,
}
```

| Command | Effect |
|---------|--------|
| `use scene <id\|name-fragment>` | Set `current_scene` |
| `use character <id\|name-fragment>` | Set `current_character` |
| `context` | Print current scene/character (name + id) |
| `use clear` | Clear session |

Resolution: prefer full UUID; else unique name substring match in DB. **0 or ≥2 matches → error + list candidates; never silent first-match.**

One-shot processes start with an empty session; `derive` / `narrate` require `--scene` (and `--character` when needed).

### Commands that do not call LLM

| Command | Args | Pre-gate | Behavior |
|---------|------|----------|----------|
| `character create <name>` | `--tags`, `--skills` optional | name non-empty | Create character |
| `character list` | — | — | List |
| `character show <id\|name>` | — | unique resolve | Detail |
| `scene create <event>` | `--with a,b` (≥1) | event non-empty; all participants resolve | `create_scene`; REPL **auto `use scene`** new scene |
| `scene list` / `scene show` | — | show: unique resolve | List / detail |
| `show derivation` | optional `--scene` `--character` | resolvable scene | Read DB; no LLM |
| `show prose` | optional `--scene` | — | Process-local cache of last `narrate` in this process; if none, error with `next: narrate` |
| `help` / `quit` | — | — | REPL |

**Prose persistence:** MVP does **not** add a prose table. `show prose` uses in-process cache only. One-shot `narrate` prints immediately.

### Commands that call LLM (full harness path)

| Command | Target resolution | Pre-gate (no LLM if fail) | Service call | Observation |
|---------|-------------------|---------------------------|--------------|-------------|
| `derive` | scene: `--scene` or session; character: `--character` or session or **all participants** | scene exists; if character set: exists, participant, not Dead/Absent | one → `derive_character`; else → `derive_scene` | per-character summary; partial failures preserved |
| `narrate` | scene: `--scene` or session | scene exists; **≥1 derivation** for scene | load derivations → `narrate_scene` | full text + quality KPIs; fill ProseCache |

No `--prompt` or free-text “author note” parameters on `derive` / `narrate`.

### Example REPL flow

```text
$ novels repl
vocab loaded: ...
status: warning    # only if no API key
summary: DEEPSEEK_API_KEY 未设置，使用 Mock（非生产质量）

> character create 宝玉 --tags 痴情,贵公子
> character create 黛玉 --tags 敏感,诗才
> scene create "大观园初见雨" --with 宝玉,黛玉
> derive
> narrate
```

### One-shot examples

```text
novels character create 宝玉 --tags 痴情
novels scene create "古宅发现尸体" --with 宝玉
novels derive --scene <uuid>
novels narrate --scene <uuid>
```

### Copy rules

- Command names: English
- `summary`, errors, `next`: Chinese
- Each error: one-line cause + one copy-pasteable next command
- Do not dump raw vocabulary ID catalogs in normal author-facing output (`--verbose` optional, not required for MVP)

## Data flow, config, errors

### Shared bootstrap

```text
dotenv
Db::open (default novels.db; optional --db <path>)
load_runtime_vocab (same env as main)
SenseGenerator + ProseGenerator (Key → Rig; else Mock + warning)
StoryService::new(...)
```

### Config flags

| Source | Behavior |
|--------|----------|
| `DEEPSEEK_API_KEY` | Same as main |
| `NOVELS_SKIP_DISTILLED` / `NOVELS_DISTILLED_DIR` | Same as main |
| `--db <path>` | Optional DB path |
| `--json` | Machine-readable observation (same fields); does not change generation |
| Forbidden flags | `--raw-llm`, `--skip-validate`, `--free-prose` |

### I/O split

- Vocab load lines → `stderr`
- Observation + prose body → `stdout`
- `--json`: stdout is JSON only

### Error contract

| Layer | Behavior |
|-------|----------|
| Parse errors | Chinese hint + help; one-shot exit ≠ 0 |
| Pre-gate | No LLM; `status=error` + `next` |
| `StoryError` | Mapped to readable Chinese; keep type distinction |
| Partial `derive_scene` failure | Report failed character; keep others (service semantics) |
| Mock | Success still may carry `warning` (non-production) |
| REPL | Command failure does not exit REPL |
| One-shot | Failure → exit code 1 |

### Concurrency

- REPL runs commands **serially**
- Shared `novels.db` with a concurrent API process is allowed but undocumented multi-writer merge is out of scope; note race risk in docs only

## Testing

### Principles

- No real DeepSeek in tests (Mock generators)
- Lock harness path: CLI must not skip validate/assembly
- Prefer narrow unit tests + one in-memory closed-loop integration test

### MVP test matrix

| Layer | Content | Pass criteria |
|-------|---------|---------------|
| Parse | clap args for create/derive/narrate | Valid → struct; missing → error |
| Session | use / ambiguity / clear | No silent multi-match pick |
| Pre-gate | derive/narrate without scene; narrate without derivation | **Zero** generator calls + `next` |
| Closed loop | create → scene → derive → narrate (Mock, in-memory DB) | Persist OK; observation includes quality keys after narrate |
| Coexistence | Bare main still builds | No regression to service assembly |
| Exit codes | One-shot failure | Non-zero exit |

Suggested home: `tests/cli_test.rs` and/or `src/cli` unit tests with `Db::open_in_memory()`.

### Not tested in MVP

- Full TTY / reedline recordings
- Dual-writer SQLite races with live Axum
- Real-API quote_density numerical stability

## Non-goals (explicit)

| Excluded | Why |
|----------|-----|
| Free chat / custom prompts | Breaks action space and anti-AI-taste |
| Looser validation / skip assembly | Quality regression |
| Chinese command aliases, fancy completion | Not MVP |
| Relationship resolve / story-graph CLI | Later; Web already covers |
| Mandatory prose table | No schema expansion for MVP |
| CLI over HTTP | Extra failure mode; no quality benefit |
| Default entry becomes REPL-only | Preserve bare `novels` |
| User-tunable sampling | Determinism tied to existing policy |

## Acceptance (MVP done)

1. `novels repl` completes: create characters → create scene → `derive` → `narrate`, Chinese output + quality KPIs.
2. One-shot `character|scene|derive|narrate` paths work.
3. No API key → warning + Mock; not presented as production quality.
4. Vocab / distilled / env parity with `main`.
5. No bypass flags; LLM only via `StoryService`.
6. New CLI tests green; existing scene/e2e (and related) suites do not regress because of CLI wiring.
7. Implementation follows a separate plan from `writing-plans` after this spec is reviewed.

## Implementation note

Do not implement until:

1. This spec is reviewed by the user with no blocking changes, and
2. An implementation plan is written under `docs/superpowers/plans/`.

Dependencies to add (expected): `clap` (derive features). Avoid reedline in MVP unless plan explicitly promotes it.

## Research notes (brief)

Industry patterns for Rust interactive CLIs include clap subcommands plus optional reedline / clap-repl for line editing. For this product, a simple stdin REPL over the existing service boundary is sufficient for MVP; richer line editing is a later UX upgrade, not a quality prerequisite.
