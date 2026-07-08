//! Application mode: painting the starting cube vs. playing/solving it.
//!
//! Input mode paints a `PartialFacelets` (see `paint.rs`); Solve mode runs the
//! existing manual-turn and solve-playback systems. `main.rs` gates systems on
//! the [`in_input`] / [`in_solve`] run conditions.

use bevy::prelude::*;

/// Whether the user is entering a cube (by paint or camera) or playing/solving.
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum AppMode {
    /// Painting the starting configuration.
    #[default]
    Input,
    /// Scanning the cube with the camera (spec 0002); constructed only when the
    /// `camera` feature is enabled.
    #[cfg_attr(not(feature = "camera"), allow(dead_code))]
    Camera,
    /// Manual play and solve playback.
    Solve,
}

impl AppMode {
    /// Short label for the HUD.
    #[must_use]
    pub fn label(self) -> &'static str {
        match self {
            AppMode::Input => "INPUT - paint your cube",
            AppMode::Camera => "CAMERA - scan your cube",
            AppMode::Solve => "SOLVE / PLAY",
        }
    }
}

/// Run condition: currently in camera-scan mode.
#[cfg(feature = "camera")]
#[must_use]
pub fn in_camera(mode: Res<AppMode>) -> bool {
    *mode == AppMode::Camera
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
