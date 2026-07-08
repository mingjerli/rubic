//! Application mode: painting the starting cube vs. playing/solving it.
//!
//! Input mode paints a `PartialFacelets` (see `paint.rs`); Solve mode runs the
//! existing manual-turn and solve-playback systems. `main.rs` gates systems on
//! the [`in_input`] / [`in_solve`] run conditions.

use bevy::prelude::*;

/// Whether the user is entering a cube or playing/solving it.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AppMode {
    /// Painting the starting configuration.
    #[default]
    Input,
    /// Manual play and solve playback.
    Solve,
}

impl AppMode {
    /// Short label for the HUD.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            AppMode::Input => "INPUT - paint your cube",
            AppMode::Solve => "SOLVE / PLAY",
        }
    }
}

/// Run condition: currently in input mode.
#[must_use]
pub fn in_input(mode: Res<AppMode>) -> bool {
    *mode == AppMode::Input
}

/// Run condition: currently in solve mode.
#[must_use]
pub fn in_solve(mode: Res<AppMode>) -> bool {
    *mode == AppMode::Solve
}
