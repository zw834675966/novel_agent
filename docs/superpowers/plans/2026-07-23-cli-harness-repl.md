# CLI Harness REPL Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add an in-process story-operation CLI (`novels repl` + one-shot subcommands) that drives existing `StoryService` derive/narrate with full Harness gates and no quality degradation.

**Architecture:** Extract shared bootstrap (Db + vocab + generators) from `main`. New `src/cli/` owns clap args, REPL session, command dispatch, and observation formatting. Only `derive` / `narrate` call the LLM, exclusively via `StoryService`. Bare `novels` (no subcommand) keeps demo + API on `:3000`.

**Tech Stack:** Rust 2024, clap 4 (derive), existing tokio + sqlx + StoryService + Mock/Rig generators. No reedline in MVP.

**Spec:** `docs/superpowers/specs/2026-07-23-cli-harness-repl-design.md`

## Global Constraints

- CLI must not bypass `StoryService`, `validate_selection`, `text_guard`, or prose assembly.
- No free Chat, no `--prompt`, no `--raw-llm` / `--skip-validate` / `--free-prose`.
- English subcommands; Chinese `summary` / errors / `next`.
- Missing `DEEPSEEK_API_KEY` → Mock pair + explicit `status: warning` +「非生产质量」.
- Distilled env: same as `main` (`NOVELS_SKIP_DISTILLED`, `NOVELS_DISTILLED_DIR`).
- One-shot `show prose`: explicit message that cache is REPL-process-only; guide to `novels narrate`.
- Clap: optional subcommands, `subcommand_required_else_help(false)`, bare run → legacy main.
- SQLite multi-writer merge with concurrent API: out of scope; surface busy/lock as readable error.
- Tests: Mock generators only; no real DeepSeek.
- Do not revert or reformat unrelated dirty worktree files; stage only files touched by each task.

---

## File map

| Path | Responsibility |
|------|----------------|
| `Cargo.toml` | Add `clap` with `derive` feature |
| `src/bootstrap.rs` | Shared `AppRuntime` init (db path, vocab, generators, mock flag) |
| `src/cli/mod.rs` | Module root; `run_cli` entry |
| `src/cli/args.rs` | Clap `Cli` / `Commands` tree |
| `src/cli/session.rs` | `Session`, name/id resolve helpers |
| `src/cli/observation.rs` | `Observation`, `QualityKpis`, format human / JSON |
| `src/cli/commands.rs` | Execute one command against `StoryService` + session + prose cache |
| `src/cli/repl.rs` | Stdin loop, parse line → command, quit/help |
| `src/cli/prose_cache.rs` | Process-local last `AssembledProse` + scene id |
| `src/lib.rs` | `pub mod bootstrap; pub mod cli;` |
| `src/main.rs` | Clap dispatch: None → legacy; Some → cli |
| `tests/cli_test.rs` | Parse, session, pre-gate, closed-loop, one-shot show prose message |

---

### Task 1: Clap dependency + args types + parse tests

**Files:**
- Modify: `Cargo.toml`
- Create: `src/cli/mod.rs`, `src/cli/args.rs`
- Modify: `src/lib.rs`
- Test: unit tests inside `src/cli/args.rs` (`#[cfg(test)]`)

**Interfaces:**
- Produces:
  - `pub struct Cli { pub db: Option<PathBuf>, pub json: bool, pub command: Option<Commands> }`
  - `pub enum Commands { Repl, Character(CharacterCmd), Scene(SceneCmd), Derive { scene, character }, Narrate { scene }, Show(ShowCmd), Use(UseCmd), Context }`
  - Nested enums for character create/list/show, scene create/list/show, show derivation/prose, use scene/character/clear
- Consumes: nothing from later tasks

- [ ] **Step 1: Add clap to Cargo.toml**

```toml
clap = { version = "4", features = ["derive"] }
```

- [ ] **Step 2: Write failing parse tests in `src/cli/args.rs`**

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn bare_cli_has_no_subcommand() {
        let cli = Cli::try_parse_from(["novels"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn parse_character_create_with_tags() {
        let cli = Cli::try_parse_from([
            "novels",
            "character",
            "create",
            "宝玉",
            "--tags",
            "痴情,贵公子",
        ])
        .unwrap();
        match cli.command {
            Some(Commands::Character(CharacterCmd::Create { name, tags, .. })) => {
                assert_eq!(name, "宝玉");
                assert_eq!(tags.as_deref(), Some("痴情,贵公子"));
            }
            other => panic!("unexpected: {other:?}"),
        }
    }

    #[test]
    fn parse_derive_with_scene() {
        let cli = Cli::try_parse_from([
            "novels",
            "derive",
            "--scene",
            "11111111-1111-1111-1111-111111111111",
        ])
        .unwrap();
        assert!(matches!(cli.command, Some(Commands::Derive { .. })));
    }

    #[test]
    fn parse_repl() {
        let cli = Cli::try_parse_from(["novels", "repl"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Repl)));
    }
}
```

- [ ] **Step 3: Run tests — expect compile/link fail (module missing)**

Run: `cargo test --lib cli::args::tests -- --nocapture`  
Expected: FAIL (module `cli` not found or similar)

- [ ] **Step 4: Implement `src/cli/args.rs` + `src/cli/mod.rs` + export in lib**

```rust
// src/cli/mod.rs
pub mod args;
pub use args::{Cli, Commands, CharacterCmd, SceneCmd, ShowCmd, UseCmd};

// src/cli/args.rs — essential shape (complete fields in implementation)
use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Debug, Parser)]
#[command(name = "novels", subcommand_required = false)]
pub struct Cli {
    /// SQLite database path (default: novels.db)
    #[arg(long, global = true)]
    pub db: Option<PathBuf>,

    /// Machine-readable observation on stdout
    #[arg(long, global = true)]
    pub json: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Interactive story-operation REPL
    Repl,
    #[command(subcommand)]
    Character(CharacterCmd),
    #[command(subcommand)]
    Scene(SceneCmd),
    Derive {
        #[arg(long)]
        scene: Option<String>,
        #[arg(long)]
        character: Option<String>,
    },
    Narrate {
        #[arg(long)]
        scene: Option<String>,
    },
    #[command(subcommand)]
    Show(ShowCmd),
    #[command(subcommand)]
    Use(UseCmd),
    /// Print current session context (REPL; one-shot always empty)
    Context,
}

#[derive(Debug, Subcommand)]
pub enum CharacterCmd {
    Create {
        name: String,
        #[arg(long)]
        tags: Option<String>,
        #[arg(long)]
        skills: Option<String>,
    },
    List,
    Show { id_or_name: String },
}

#[derive(Debug, Subcommand)]
pub enum SceneCmd {
    Create {
        event: String,
        /// Comma-separated participant names or UUIDs
        #[arg(long = "with")]
        with: String,
    },
    List,
    Show { id_or_name: String },
}

#[derive(Debug, Subcommand)]
pub enum ShowCmd {
    Derivation {
        #[arg(long)]
        scene: Option<String>,
        #[arg(long)]
        character: Option<String>,
    },
    Prose {
        #[arg(long)]
        scene: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum UseCmd {
    Scene { id_or_name: String },
    Character { id_or_name: String },
    Clear,
}
```

In `src/lib.rs` add: `pub mod cli;`

Note: clap 4 uses `subcommand_required = false` in `#[command(...)]` attribute (maps to the design’s `subcommand_required_else_help(false)` intent: bare parse succeeds with `command: None`).

- [ ] **Step 5: Run parse tests — expect PASS**

Run: `cargo test --lib cli::args -- --nocapture`  
Expected: PASS

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock src/cli/mod.rs src/cli/args.rs src/lib.rs
git commit -m "feat(cli): add clap args tree for story harness"
```

---

### Task 2: Observation types + human/JSON formatting

**Files:**
- Create: `src/cli/observation.rs`
- Modify: `src/cli/mod.rs`
- Test: unit tests in `observation.rs`

**Interfaces:**
- Produces:
  - `pub enum Status { Success, Warning, Error }`
  - `pub struct QualityKpis { quote_density, stripped_refs, unverified_quotes, low_quote_density, action_only_beats, rejected_beats }`
  - `pub struct Observation { status, summary, artifacts: BTreeMap<String,String>, quality: Option<QualityKpis>, next: Vec<String> }`
  - `impl Observation { pub fn render(&self, json: bool) -> String }`
  - `pub fn quality_from_prose(p: &AssembledProse) -> QualityKpis`
- Consumes: `novels::prose::AssembledProse`

- [ ] **Step 1: Write failing tests**

```rust
#[test]
fn render_human_includes_status_and_summary() {
    let o = Observation {
        status: Status::Warning,
        summary: "DEEPSEEK_API_KEY 未设置，使用 Mock（非生产质量）".into(),
        artifacts: Default::default(),
        quality: None,
        next: vec![],
    };
    let s = o.render(false);
    assert!(s.contains("status: warning"));
    assert!(s.contains("非生产质量"));
}

#[test]
fn quality_from_prose_maps_kpis() {
    // Build a minimal AssembledProse with known fields (use struct literal
    // matching src/prose/assembly.rs fields + quality).
    // assert_eq!(k.stripped_refs, ...);
}
```

- [ ] **Step 2: Run — expect FAIL**

Run: `cargo test --lib cli::observation -- --nocapture`  
Expected: FAIL (module missing)

- [ ] **Step 3: Implement `observation.rs`**

Human format (exact keys for harness):

```text
status: success|warning|error
summary: ...
artifacts:
  scene_id: ...
quality:
  quote_density: 0.42
  stripped_refs: 0
  unverified_quotes: 0
  low_quote_density: false
  action_only_beats: 0
  rejected_beats: 0
next:
  - narrate
```

JSON: serde_json serialize the same logical fields (`status` as lowercase string).

Map `AssembledProse`:

```rust
QualityKpis {
  quote_density: prose.quote_density(), // or prose.quality.quote_density
  stripped_refs: prose.stripped_refs,
  unverified_quotes: prose.unverified_quotes,
  low_quote_density: prose.quality.low_quote_density,
  action_only_beats: prose.action_only_beats,
  rejected_beats: prose.rejected_beats,
}
```

- [ ] **Step 4: Tests PASS**

Run: `cargo test --lib cli::observation -- --nocapture`  
Expected: PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli/observation.rs src/cli/mod.rs
git commit -m "feat(cli): add harness observation formatting"
```

---

### Task 3: Session + entity resolve helpers

**Files:**
- Create: `src/cli/session.rs`
- Modify: `src/cli/mod.rs`
- Test: unit tests with `Db::open_in_memory()` (async tokio::test)

**Interfaces:**
- Produces:
  - `pub struct Session { pub current_scene: Option<SceneId>, pub current_character: Option<CharacterId> }`
  - `pub async fn resolve_character(db: &Db, id_or_name: &str) -> Result<CharacterId, CliError>`
  - `pub async fn resolve_scene(db: &Db, id_or_name: &str) -> Result<SceneId, CliError>`
  - `pub enum CliError { Message { summary: String, next: Vec<String> }, Story(StoryError) }`
- Consumes: `Db`, `CharacterId`, `SceneId`, list/get repos

Resolve rules (spec):
1. If `Uuid::parse_str` succeeds → treat as id; `get` or not found error.
2. Else list all characters/scenes; filter `name`/`objective_event` containing substring (case-sensitive Chinese OK).
3. 1 match → Ok; 0 → error「未找到」; ≥2 → error listing candidates (name + id), never pick first silently.

- [ ] **Step 1: Write failing async tests**

```rust
#[tokio::test]
async fn resolve_character_unique_name() {
    let db = Db::open_in_memory().await.unwrap();
    let id = CharacterId(uuid::Uuid::new_v4());
    db.characters().create(id, "宝玉", &[], &[]).await.unwrap();
    let got = resolve_character(&db, "宝玉").await.unwrap();
    assert_eq!(got, id);
}

#[tokio::test]
async fn resolve_character_ambiguous_errors() {
    let db = Db::open_in_memory().await.unwrap();
    let a = CharacterId(uuid::Uuid::new_v4());
    let b = CharacterId(uuid::Uuid::new_v4());
    db.characters().create(a, "宝玉甲", &[], &[]).await.unwrap();
    db.characters().create(b, "宝玉乙", &[], &[]).await.unwrap();
    let err = resolve_character(&db, "宝玉").await.unwrap_err();
    let msg = err.summary();
    assert!(msg.contains("多个") || msg.contains("歧义") || msg.contains("候选"));
}
```

- [ ] **Step 2: Run — FAIL**

Run: `cargo test --lib cli::session -- --nocapture`  
Expected: FAIL

- [ ] **Step 3: Implement session + resolve**

Use `db.characters().list()` and `db.scenes().list()`. For scenes, match against `objective_event` substring (scenes have no separate title field in domain model).

- [ ] **Step 4: Tests PASS**

- [ ] **Step 5: Commit**

```bash
git add src/cli/session.rs src/cli/mod.rs
git commit -m "feat(cli): add session and unique name resolution"
```

---

### Task 4: Shared bootstrap (`AppRuntime`)

**Files:**
- Create: `src/bootstrap.rs`
- Modify: `src/lib.rs` (`pub mod bootstrap;`)
- Test: unit/integration light test that mock path sets `using_mock: true` without API key (may run in-memory)

**Interfaces:**
- Produces:

```rust
pub struct AppRuntime {
    pub service: StoryService,
    pub using_mock: bool,
    pub vocab_report_line: String, // for stderr
}

pub struct BootstrapOptions {
    pub db_path: PathBuf, // default "novels.db"
}

pub async fn bootstrap(opts: BootstrapOptions) -> anyhow::Result<AppRuntime>
```

Logic **copied from current `main.rs`** (do not simplify):
- dotenv optional (caller may call dotenv once in main)
- `Db::open(opts.db_path)`
- distilled path resolution identical to main
- `load_runtime_vocab`
- `deepseek::Client::from_env()` → Rig both, else Mock both + `using_mock = true`
- `StoryService::new(...)`

For tests later: also provide:

```rust
pub fn bootstrap_for_test(service: StoryService) -> AppRuntime {
    AppRuntime { service, using_mock: true, vocab_report_line: "test".into() }
}
```

Or tests construct `StoryService` directly and skip bootstrap — preferred for `cli_test.rs`.

- [ ] **Step 1: Implement `bootstrap.rs` by extracting from main (leave main calling it later in Task 7)**

- [ ] **Step 2: `cargo check` PASS**

Run: `cargo check`  
Expected: PASS (main still self-contained until Task 7)

- [ ] **Step 3: Commit**

```bash
git add src/bootstrap.rs src/lib.rs
git commit -m "feat: extract shared AppRuntime bootstrap"
```

---

### Task 5: Prose cache + command executor (no LLM commands first)

**Files:**
- Create: `src/cli/prose_cache.rs`, `src/cli/commands.rs`
- Modify: `src/cli/mod.rs`
- Test: `tests/cli_test.rs` (start with character/scene only)

**Interfaces:**
- Produces:

```rust
pub struct ProseCache {
    pub scene_id: Option<SceneId>,
    pub prose: Option<AssembledProse>,
}

pub struct CommandContext<'a> {
    pub service: &'a StoryService,
    pub session: &'a mut Session,
    pub prose_cache: &'a mut ProseCache,
    pub using_mock: bool,
    pub json: bool,
    /// true when process is one-shot CLI (not inside repl loop)
    pub one_shot: bool,
}

pub async fn execute(cmd: Commands, ctx: &mut CommandContext<'_>) -> Observation
```

Helper: `fn split_csv(s: &str) -> Vec<String>` trim, drop empty.

**Character create:**

```rust
let id = CharacterId(uuid::Uuid::new_v4());
let tags = split_csv(tags.as_deref().unwrap_or(""));
let skills = split_csv(skills.as_deref().unwrap_or(""));
ctx.service.db().characters().create(id, &name, &tags, &skills).await?;
// Observation success + artifact character_id
```

**Scene create:** resolve each `--with` token via `resolve_character`; `CreateScene { objective_event, participant_ids, occurred_at: Utc::now() }`; `create_scene`; if not forced one-shot-only session policy: **always set `session.current_scene`** on success (harmless for one-shot).

**List/show:** use `list_characters` / `list_scenes` / get + resolve.

- [ ] **Step 1: Write integration tests for create/list**

```rust
// tests/cli_test.rs
#[tokio::test]
async fn character_and_scene_create_via_execute() {
    let svc = mock_service().await; // helper: in-memory + MockSense + MockProse
    let mut session = Session::default();
    let mut cache = ProseCache::default();
    let mut ctx = CommandContext {
        service: &svc,
        session: &mut session,
        prose_cache: &mut cache,
        using_mock: true,
        json: false,
        one_shot: true,
    };
    let o = execute(
        Commands::Character(CharacterCmd::Create {
            name: "宝玉".into(),
            tags: Some("痴情".into()),
            skills: None,
        }),
        &mut ctx,
    )
    .await;
    assert_eq!(o.status, Status::Success);
    let chars = svc.list_characters().await.unwrap();
    assert_eq!(chars.len(), 1);
}
```

- [ ] **Step 2: Run — FAIL**

Run: `cargo test --test cli_test -- --nocapture`  
Expected: FAIL

- [ ] **Step 3: Implement prose_cache + execute for Character/Scene/Use/Context only; other Commands → error「未实现」temporarily OR implement stubs that error with next**

Prefer implementing only non-LLM fully; `Derive`/`Narrate`/`Show` return clear error until Task 6 so tests stay focused — **but Task 6 must land same PR series soon**. In this task, match arms for Derive/Narrate/Show can `todo!` only if not referenced by tests; better return `Observation { status: Error, summary: "internal: command not wired".into(), .. }`.

- [ ] **Step 4: Tests PASS**

- [ ] **Step 5: Commit**

```bash
git add src/cli/prose_cache.rs src/cli/commands.rs src/cli/mod.rs tests/cli_test.rs
git commit -m "feat(cli): execute character and scene commands"
```

---

### Task 6: Derive, narrate, show (full harness path)

**Files:**
- Modify: `src/cli/commands.rs`
- Modify: `tests/cli_test.rs`

**Interfaces:**
- Extends `execute` for `Derive`, `Narrate`, `Show::Derivation`, `Show::Prose`
- Consumes: `StoryService::derive_character`, `derive_scene`, `narrate_scene`, `scene_derivations`, `get_scene`

**Pre-gates (no generator call):**

| Case | Observation |
|------|-------------|
| derive without scene (no flag, no session) | error; next: `use scene` / `derive --scene` |
| narrate without scene | same |
| narrate with scene but zero derivations | error; next: `derive` |
| show prose with empty cache | see one-shot vs REPL messaging below |

**Derive resolution:**
- `scene = flag.or(session.current_scene)` required
- if `character` flag or session character set → `derive_character`
- else → `derive_scene` (all participants)
- If `using_mock`, status may be `Warning` even on success, or Success with warning line in summary — pick **Warning** when mock for derive/narrate success to satisfy spec “recommended”.

**Narrate:**
- Load details via `scene_derivations` or reconstruct `Vec<CharacterDerivation>` from DB the same way service/tests do for narrate.
- Inspect `e2e` / service tests for how derivations are loaded after derive. Prefer:

```rust
let details = ctx.service.scene_derivations(scene_id).await?;
// Map SceneDerivationDetail → CharacterDerivation if needed
// OR collect Ok results from derive_scene return value when called in same flow
```

If `scene_derivations` does not yield full `CharacterDerivation`, use:

```rust
// After derive_scene, keep Ok(derivation) values for narrate_scene
// For show/narrate later: re-derive is WRONG. Must load from DB.
```

**Required research step for implementer:** Open `SceneDerivationDetail` and `CharacterDerivation` in `src/models/`. If narrate needs `CharacterDerivation`, build it from sensations/memories/plots already on the detail type, or add a small private helper in `commands.rs` that calls existing repos **read-only** (no new LLM). Do **not** change assembly rules.

Then:

```rust
let prose = ctx.service.narrate_scene(scene_id, &derivations).await?;
ctx.prose_cache.scene_id = Some(scene_id);
ctx.prose_cache.prose = Some(prose.clone());
// Observation: summary 正文已生成; include quality; print body in render path
```

**Body printing:** Either put prose text in `Observation.summary` (bad for long text) or add:

```rust
pub struct CommandOutput {
    pub observation: Observation,
    pub body: Option<String>, // narrate / show prose
}
```

Update `execute` to return `CommandOutput`. Formatters print observation then body.

**Show prose:**

```rust
if ctx.prose_cache.prose.is_none() {
    if ctx.one_shot {
        return Observation {
            status: Status::Error,
            summary: "正文仅在 REPL 进程内缓存；单次命令模式下请使用 novels narrate --scene <id> 直接输出正文。".into(),
            next: vec!["novels narrate --scene <id>".into()],
            ..
        };
    } else {
        return Observation {
            status: Status::Error,
            summary: "当前进程尚无 narrate 缓存。请先执行 narrate。".into(),
            next: vec!["narrate".into()],
            ..
        };
    }
}
```

**Show derivation:** `scene_derivations` + Chinese summary of memories/sensations counts (no vocab ID dump).

- [ ] **Step 1: Write tests**

```rust
#[tokio::test]
async fn derive_without_scene_is_pregate_error() { ... }

#[tokio::test]
async fn closed_loop_derive_narrate_has_quality() {
    // create character, scene, derive, narrate via execute
    // assert output.observation.quality.is_some()
    // assert body.is_some()
}

#[tokio::test]
async fn one_shot_show_prose_explains_repl_cache() {
    // one_shot: true, empty cache, Show::Prose
    // assert summary contains "REPL" and "narrate"
}
```

- [ ] **Step 2: Run — FAIL on new asserts**

- [ ] **Step 3: Implement derive/narrate/show fully**

- [ ] **Step 4: Tests PASS**

Run: `cargo test --test cli_test -- --nocapture`  
Expected: all PASS

- [ ] **Step 5: Commit**

```bash
git add src/cli/commands.rs tests/cli_test.rs
git commit -m "feat(cli): harness derive narrate and show commands"
```

---

### Task 7: REPL loop + main clap dispatch

**Files:**
- Create: `src/cli/repl.rs`
- Modify: `src/cli/mod.rs` — `pub async fn run_cli(cli: Cli) -> anyhow::Result<i32>`
- Modify: `src/main.rs` — parse Cli, branch
- Modify: `src/bootstrap.rs` if main now uses it for both paths

**Interfaces:**
- Produces: `run_cli` exit code 0/1; `run_repl(runtime, json)`; legacy `run_legacy(runtime)` remains in main or moves to `src/legacy_main.rs` — keep in `main.rs` as `async fn run_legacy(svc: StoryService)` to minimize churn.

**main.rs shape:**

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let cli = novels::cli::Cli::parse();
    let db_path = cli.db.clone().unwrap_or_else(|| PathBuf::from("novels.db"));

    match &cli.command {
        None => {
            let rt = novels::bootstrap::bootstrap(BootstrapOptions { db_path }).await?;
            eprintln!("{}", rt.vocab_report_line);
            run_legacy(rt.service).await
        }
        Some(_) => {
            let code = novels::cli::run_cli(cli).await?;
            std::process::exit(code);
        }
    }
}
```

**run_cli:**
1. bootstrap with `cli.db`
2. eprintln vocab line
3. if `using_mock`, print Observation warning to stdout (or stderr — prefer **stdout** for observation protocol; vocab stays stderr)
4. match command:
   - `Repl` → `run_repl`
   - other → `execute` once with `one_shot: true`, print, return exit code from status (Error → 1)

**REPL:**
- prompt: `> ` on stdout or stderr (use stderr for prompt so body pipelines stay clean: **prompt → stderr**)
- read line; empty continue; `quit`/`exit` break
- Parse with `Cli::try_parse_from` after injecting program name:

```rust
// User types: character create 宝玉 --tags 痴情
// Build: ["novels", ...split] — use shell-like split carefully.
// MVP: use a simple splitter that respects double quotes for Chinese events.
```

Implement `fn split_repl_line(line: &str) -> Vec<String>`:
- Split on whitespace; `"..."` keeps spaces.
- Then `Cli::try_parse_from(std::iter::once("novels".into()).chain(args))`
- Reject nested `repl` command with friendly error
- Map `Commands` only; if user types global flags, allow `--json` mid-session optional (YAGNI: ignore; use repl start json flag only)

Session + ProseCache live for the whole REPL process.

On each command: `one_shot: false`.

- [ ] **Step 1: Implement repl + run_cli + main dispatch**

- [ ] **Step 2: `cargo test --lib cli -- --nocapture` and `cargo test --test cli_test`**

Expected: PASS

- [ ] **Step 3: Manual smoke (optional if no API key)**

```powershell
$env:NOVELS_SKIP_DISTILLED="1"
cargo run -- character create 测试 --tags t
# expect warning mock + success
cargo run -- show prose
# expect REPL-cache message and non-zero exit
```

- [ ] **Step 4: Commit**

```bash
git add src/cli/repl.rs src/cli/mod.rs src/main.rs src/bootstrap.rs
git commit -m "feat(cli): wire REPL and clap bare-run dispatch"
```

---

### Task 8: Error mapping, SQLite busy message, AGENTS note

**Files:**
- Modify: `src/cli/commands.rs` / `session.rs` — map `StoryError` and sqlx busy to Chinese
- Modify: `Agents.md` or `Claude.md` — short CLI section (optional but recommended; path `Agents.md` per repo)

**StoryError → summary:** use `format!("{e}")` if Display is good; else match variants for NotFound / NotParticipant / Llm / InvalidNarrationContext.

**Busy:** if error string contains `database is locked` or `busy`, summary: `数据库忙碌（可能与 API 进程同时写入 novels.db）；请稍后重试，MVP 不提供多写合并。`

**AGENTS.md snippet:**

```markdown
### CLI Harness (MVP)
- `cargo run -- repl` interactive story ops
- `cargo run -- character|scene|derive|narrate ...` one-shot
- Bare `cargo run` still demo + API :3000
- Same StoryService quality path; no free chat
```

- [ ] **Step 1: Implement mapping + docs**

- [ ] **Step 2: `cargo test --test cli_test` + `cargo test --test e2e` (smoke no regress)**

Expected: PASS

- [ ] **Step 3: Commit**

```bash
git add src/cli/ Agents.md
git commit -m "docs: note CLI harness usage and busy DB errors"
```

---

### Task 9: Verification gate

**Files:** none (run only)

- [ ] **Step 1: Run focused CLI tests**

```powershell
cargo test --lib cli -- --nocapture
cargo test --test cli_test -- --nocapture
```

Expected: PASS

- [ ] **Step 2: Run e2e + scene smoke**

```powershell
cargo test --test e2e -- --nocapture
cargo test --test scene_test -- --nocapture
```

Expected: PASS (or document pre-existing baseline failures unrelated to CLI; do not “fix” by weakening tests)

- [ ] **Step 3: Confirm bare run still parses**

```powershell
cargo run -- --help
# shows subcommands
# Do not leave a long-running server in CI; bare run without args starts API — skip automated hang
```

- [ ] **Step 4: Final commit only if verification fixes were needed; else stop**

---

## Self-review (plan vs spec)

| Spec requirement | Task |
|------------------|------|
| Story-op REPL + one-shot | 1, 5–7 |
| Bare novels legacy | 7 |
| Min loop character/scene/derive/narrate/show | 5–6 |
| Explicit use session | 3, 5 |
| English cmds + Chinese results | 2, 5–6 |
| In-process StoryService | 5–6 |
| Quality non-degradation | 6 (only service path) |
| Harness observation + quality KPIs | 2, 6 |
| Mock warning | 4, 7 |
| One-shot show prose message | 6 |
| Clap optional subcommand | 1, 7 |
| SQLite multi-write out of scope + message | 8 |
| Tests Mock + pre-gate + closed loop | 3, 5, 6, 9 |
| No reedline / no free chat / no bypass flags | Global + args surface |

**Placeholder scan:** No TBD steps; derivation→narrate load path has an explicit implementer research step with allowed outcomes (read-only rebuild from DB / scene_derivations).

**Type consistency:** `Cli` / `Commands` / `Session` / `Observation` / `CommandContext` / `CommandOutput` / `AppRuntime` names are stable across tasks.

---

## Execution handoff

Plan complete and saved to `docs/superpowers/plans/2026-07-23-cli-harness-repl.md`.

**Two execution options:**

1. **Subagent-Driven (recommended)** — fresh subagent per task, review between tasks  
2. **Inline Execution** — this session with executing-plans and checkpoints  

Which approach?
