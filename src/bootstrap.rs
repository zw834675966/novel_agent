//! Shared application bootstrap.
//!
//! Extracts the Db + vocab + generator initialization that previously lived
//! inline in `src/main.rs` so that both the CLI harness and the legacy binary
//! can share a single initialization path.
//!
//! The caller is responsible for invoking `dotenv::dotenv()` once before
//! calling [`bootstrap`]; this module only reads environment variables for
//! distilled-vocab resolution and DeepSeek client construction.

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Context;
use rig::client::ProviderClient;
use rig::providers::deepseek;

use crate::db::Db;
use crate::llm::{
    LlmCharacterDerivation, LlmContextTagSelection, MockSenseGenerator, RigSenseGenerator,
    SenseGenerator,
};
use crate::models::{Certainty, CharacterMemoryDraft, MemorySource, SensorySelection};
use crate::prose::{MockProseGenerator, ProseGenerator, RigProseGenerator};
use crate::scene::StoryService;
use crate::vocab::load_runtime_vocab;

/// Fully initialized application runtime.
///
/// Holds the assembled [`StoryService`] plus the bookkeeping a caller may want
/// to report: whether mock generators were used and the single-line vocab load
/// summary that `main` historically printed to stderr.
pub struct AppRuntime {
    /// The assembled story service, ready for derivation / narration.
    pub service: StoryService,
    /// `true` when the DeepSeek client could not be built from the environment
    /// and the mock sense + prose generators were used instead.
    pub using_mock: bool,
    /// The single-line vocab load summary (`"vocab loaded: base=... ..."`)
    /// that `main` historically printed to stderr. [`bootstrap`] returns it
    /// instead of printing so the caller controls logging.
    pub vocab_report_line: String,
}

/// Options for [`bootstrap`].
pub struct BootstrapOptions {
    /// Path to the SQLite database file (created on first open).
    pub db_path: PathBuf,
}

/// Initialize the full application stack.
///
/// Mirrors the bootstrap sequence previously inlined in `src/main.rs`:
///
/// 1. Open the SQLite database at `opts.db_path`.
/// 2. Resolve the optional distilled-vocab directory from
///    `NOVELS_SKIP_DISTILLED` / `NOVELS_DISTILLED_DIR`.
/// 3. Load the base vocab (`assets/vocab.yaml`) plus any distilled merge.
/// 4. Build the DeepSeek-backed sense + prose generators, or fall back to the
///    mock pair when the client cannot be constructed from the environment.
/// 5. Assemble the [`StoryService`].
///
/// The caller should call `dotenv::dotenv()` beforehand if `.env` loading is
/// desired; this function does not call dotenv itself.
pub async fn bootstrap(opts: BootstrapOptions) -> anyhow::Result<AppRuntime> {
    // 1. Database. `Db::open` takes a `&str` path, so a non-UTF-8 db_path is
    //    surfaced as an error rather than silently lossy-converted.
    let db_path_str = opts
        .db_path
        .to_str()
        .with_context(|| format!("db_path is not valid UTF-8: {}", opts.db_path.display()))?;
    let db = Db::open(db_path_str).await?;

    // 2. Distilled-vocab path resolution.
    //    - NOVELS_SKIP_DISTILLED=1|true : base only
    //    - NOVELS_DISTILLED_DIR=<path>  : override (must be an existing dir)
    //    - otherwise                    : default to assets/distilled if it is a dir
    let skip_distilled = std::env::var("NOVELS_SKIP_DISTILLED")
        .map(|v| v == "1" || v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);
    let distilled = if skip_distilled {
        None
    } else {
        std::env::var("NOVELS_DISTILLED_DIR")
            .ok()
            .filter(|s| !s.is_empty())
            .map(PathBuf::from)
            .filter(|p| p.is_dir())
            .or_else(|| {
                let p = PathBuf::from("assets/distilled");
                p.is_dir().then_some(p)
            })
    };

    // 3. Vocabulary: base (assets/vocab.yaml) + optional distilled merge.
    let (vocab, report) = load_runtime_vocab(
        std::path::Path::new("assets/vocab.yaml"),
        distilled.as_deref(),
    )?;
    let vocab_report_line = format!(
        "vocab loaded: base={} distilled_files={} total={}",
        report.base_entries, report.distilled_files, report.total_entries
    );

    // 4. Generators. A single from_env() call builds both sense + prose
    //    adapters; on failure both degrade to mocks so we never ship a
    //    half-configured (one real + one mock) state.
    let (sense_generator, prose_generator, using_mock): (
        Arc<dyn SenseGenerator>,
        Arc<dyn ProseGenerator>,
        bool,
    ) = match deepseek::Client::from_env() {
        Ok(client) => (
            Arc::new(RigSenseGenerator::new(client.clone(), vocab.clone())),
            Arc::new(RigProseGenerator::new(client)),
            false,
        ),
        Err(_) => {
            eprintln!("DEEPSEEK_API_KEY not set, using mock generators");
            (
                Arc::new(MockSenseGenerator::new(
                    LlmContextTagSelection::default(),
                    LlmCharacterDerivation {
                        sensations: SensorySelection::default(),
                        new_memory: CharacterMemoryDraft {
                            content: "mock".into(),
                            source: MemorySource::Witnessed,
                            certainty: Certainty::Certain,
                        },
                        plot_development: vec![],
                        relationship_candidates: vec![],
                    },
                )),
                Arc::new(MockProseGenerator::fallback()),
                true,
            )
        }
    };

    // 5. StoryService.
    let service = StoryService::new(db, vocab, sense_generator, prose_generator);

    Ok(AppRuntime {
        service,
        using_mock,
        vocab_report_line,
    })
}

/// Build an [`AppRuntime`] wrapping an already-constructed [`StoryService`].
///
/// Intended for tests that construct the service directly (e.g. with in-memory
/// SQLite and mock generators) and want to skip the full [`bootstrap`] path.
/// Marks `using_mock: true` since such test services are mock-backed.
pub fn bootstrap_for_test(service: StoryService) -> AppRuntime {
    AppRuntime {
        service,
        using_mock: true,
        vocab_report_line: "test".into(),
    }
}
