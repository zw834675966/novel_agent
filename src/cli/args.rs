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
    Show {
        id_or_name: String,
    },
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
    Show {
        id_or_name: String,
    },
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
