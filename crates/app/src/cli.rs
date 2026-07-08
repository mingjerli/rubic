//! Command-line interface. Running `rubic` with no arguments launches the GUI;
//! `--scramble` and `--facelets` seed the starting cube state.
//!
//! Called by `main.rs`. The `initial_facelets` conversion is pure so it is
//! unit-tested without launching Bevy.

use clap::{Parser, Subcommand};
use rubic_core::{Facelets, Sequence};
use std::path::PathBuf;
use std::str::FromStr;

/// Interactive 3x3 Rubik's Cube (Bevy GUI).
///
/// With no subcommand the cube starts solved (or from `--scramble` /
/// `--facelets`) in the GUI. The `cheatsheet` subcommand prints the printable
/// solving guide instead of launching the GUI.
#[derive(Parser, Debug, Clone, Default)]
#[command(name = "rubic", version, about)]
pub struct Cli {
    /// Optional subcommand; when omitted, the GUI launches.
    #[command(subcommand)]
    pub command: Option<Command>,

    /// Scramble to apply to a solved cube, in standard notation, e.g.
    /// `"R U R' U2 F"`.
    #[arg(long)]
    pub scramble: Option<String>,

    /// Explicit start state: 54 face letters in URFDLB order
    /// (`UUUUUUUUURRR...`). Takes precedence over `--scramble`.
    #[arg(long)]
    pub facelets: Option<String>,
}

/// Subcommands that do something other than launch the GUI.
#[derive(Subcommand, Debug, Clone)]
pub enum Command {
    /// Print the printable beginner-method cheat sheet.
    Cheatsheet {
        /// Emit Markdown instead of the default self-contained HTML.
        #[arg(long)]
        markdown: bool,

        /// Write to this file instead of standard output.
        #[arg(long, short)]
        output: Option<PathBuf>,
    },
}

/// Run the `cheatsheet` subcommand: render and print or save the guide.
///
/// # Errors
/// Returns a message if writing the output file fails.
pub fn run_cheatsheet(markdown: bool, output: Option<&PathBuf>) -> Result<(), String> {
    let content = if markdown {
        rubic_core::cheatsheet::markdown()
    } else {
        rubic_core::cheatsheet::html()
    };
    match output {
        Some(path) => {
            std::fs::write(path, content).map_err(|e| format!("could not write {path:?}: {e}"))?;
            println!("wrote cheat sheet to {}", path.display());
        }
        None => print!("{content}"),
    }
    Ok(())
}

/// Build the starting cube from the parsed arguments.
///
/// Precedence: `--facelets` (explicit state) wins over `--scramble`, which
/// wins over the default solved cube.
///
/// # Errors
/// Returns a human-readable message if a facelet string or scramble cannot be
/// parsed.
pub fn initial_facelets(cli: &Cli) -> Result<Facelets, String> {
    if let Some(raw) = &cli.facelets {
        return Facelets::from_str(raw.trim()).map_err(|e| format!("invalid --facelets: {e}"));
    }
    if let Some(raw) = &cli.scramble {
        let seq = Sequence::from_str(raw.trim()).map_err(|e| format!("invalid --scramble: {e}"))?;
        return Ok(Facelets::SOLVED.apply_seq(&seq));
    }
    Ok(Facelets::SOLVED)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_is_solved() {
        let cli = Cli::default();
        assert_eq!(initial_facelets(&cli).unwrap(), Facelets::SOLVED);
    }

    #[test]
    fn scramble_is_applied_to_solved() {
        let cli = Cli {
            command: None,
            scramble: Some("R".into()),
            facelets: None,
        };
        let expected = Facelets::SOLVED.apply_seq(&Sequence::from_str("R").unwrap());
        assert_eq!(initial_facelets(&cli).unwrap(), expected);
    }

    #[test]
    fn facelets_round_trip() {
        let s = Facelets::SOLVED.to_string();
        let cli = Cli {
            command: None,
            scramble: None,
            facelets: Some(s.clone()),
        };
        assert_eq!(initial_facelets(&cli).unwrap().to_string(), s);
    }

    #[test]
    fn facelets_beats_scramble() {
        let s = Facelets::SOLVED.to_string();
        let cli = Cli {
            command: None,
            scramble: Some("R U R'".into()),
            facelets: Some(s.clone()),
        };
        // Explicit facelets should win, yielding the solved cube.
        assert_eq!(initial_facelets(&cli).unwrap(), Facelets::SOLVED);
    }

    #[test]
    fn bad_scramble_errors() {
        let cli = Cli {
            command: None,
            scramble: Some("R X Q".into()),
            facelets: None,
        };
        assert!(initial_facelets(&cli).is_err());
    }

    #[test]
    fn bad_facelets_errors() {
        let cli = Cli {
            command: None,
            scramble: None,
            facelets: Some("too short".into()),
        };
        assert!(initial_facelets(&cli).is_err());
    }
}
