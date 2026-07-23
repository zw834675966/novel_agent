// src/cli/mod.rs
pub mod args;
pub mod observation;
pub mod session;
pub use args::{CharacterCmd, Cli, Commands, SceneCmd, ShowCmd, UseCmd};
pub use observation::{Observation, QualityKpis, Status};
pub use session::{CliError, Session, resolve_character, resolve_scene};
