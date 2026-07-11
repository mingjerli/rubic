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
            AppMode::Input => "INPUT",
            AppMode::Camera => "CAMERA",
            AppMode::Solve => "SOLVE",
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

/// The stage of Input mode: picking a setup method, or actively entering a cube.
///
/// Only meaningful while [`AppMode::Input`]. On open the user first *chooses*
/// how to enter their cube (shuffle / manual / camera); the 2D net + palette
/// only appear once they start editing (manual paint or post-scan review).
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InputStage {
    /// The method picker (the opening screen).
    #[default]
    ChooseMethod,
    /// Painting by hand or reviewing a scanned cube; the 2D layout is live.
    Editing,
}

/// Run condition: in Input mode and actively editing a cube (not the picker).
#[must_use]
pub fn editing_input(mode: Res<AppMode>, stage: Res<InputStage>) -> bool {
    *mode == AppMode::Input && *stage == InputStage::Editing
}
