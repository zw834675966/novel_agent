// src/cli/mod.rs
pub mod args;
pub mod commands;
pub mod observation;
pub mod prose_cache;
pub mod session;
pub use args::{CharacterCmd, Cli, Commands, SceneCmd, ShowCmd, UseCmd};
pub use commands::{CommandContext, execute};
pub use observation::{Observation, QualityKpis, Status};
pub use prose_cache::ProseCache;
pub use session::{CliError, Session, resolve_character, resolve_scene};
