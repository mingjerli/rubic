//! Pure, testable summary of a cube's validation state for the on-screen HUD.
//!
//! Uses `rubic-core`'s [`PartialFacelets::analyze`] so the same code path that
//! would drive a paint UI also produces the status line. Called by `ui.rs`.

use rubic_core::{Completion, Facelets, PartialFacelets};

/// A one-line human summary of whether the current cube is solved, solvable,
/// still ambiguous, or impossible.
#[must_use]
pub fn status_line(facelets: &Facelets) -> String {
    match PartialFacelets::from_facelets(facelets).analyze() {
        Completion::Unique(state) => {
            if state.is_solved() {
                "Solved!".to_string()
            } else {
                "Valid - solvable".to_string()
            }
        }
        Completion::NeedMore { known } => format!("Incomplete - {known}/48 stickers known"),
        Completion::Impossible(err) => format!("Invalid - {err}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rubic_core::Sequence;
    use std::str::FromStr;

    #[test]
    fn solved_cube_reports_solved() {
        assert_eq!(status_line(&Facelets::SOLVED), "Solved!");
    }

    #[test]
    fn scrambled_valid_cube_reports_solvable() {
        let seq = Sequence::from_str("R U R' U' F2 L D").unwrap();
        let f = Facelets::SOLVED.apply_seq(&seq);
        assert_eq!(status_line(&f), "Valid - solvable");
    }

    #[test]
    fn nonsense_cube_reports_invalid() {
        // All 54 stickers the same color cannot be a real cube.
        let f = Facelets::from_str(&"U".repeat(54)).unwrap();
        let line = status_line(&f);
        assert!(line.starts_with("Invalid"), "got: {line}");
    }
}
