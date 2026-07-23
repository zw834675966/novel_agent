// src/cli/mod.rs
pub mod args;
pub mod observation;
pub use args::{CharacterCmd, Cli, Commands, SceneCmd, ShowCmd, UseCmd};
pub use observation::{Observation, QualityKpis, Status};
