//! Cube color input: painting a `PartialFacelets` on the 2D net or the 3D cube.
//!
//! One [`InputState`] is the single source of truth for input; both the net
//! (`net.rs`) and 3D sticker picking write to it, and both views render from it.
//! When the painted state is uniquely determined, [`mode_control`] confirms it
//! into [`CubeRes`] and switches to Solve mode.

use bevy::prelude::*;
use rubic_core::{Completion, Face, Facelets, PartialFacelets};

use crate::mode::AppMode;
use crate::types::{CubeRes, Sticker, StickerMaterials};

/// Palette order (also the number-key order `1..=6`).
pub const PALETTE: [Face; 6] = Face::ALL;

/// The in-progress input: a partial cube and the selected paint color.
#[derive(Resource)]
pub struct InputState {
    /// Known/unknown stickers so far.
    pub partial: PartialFacelets,
    /// The color the next paint applies.
    pub brush: Face,
}

impl InputState {
    /// Seed from a full cube (e.g. the CLI start state).
    #[must_use]
    pub fn seeded(f: &Facelets) -> Self {
        Self {
            partial: PartialFacelets::from_facelets(f),
            brush: Face::U,
        }
    }

    /// A blank input with only the centers known.
    #[must_use]
    pub fn empty() -> Self {
        Self {
            partial: PartialFacelets::new(),
            brush: Face::U,
        }
    }

    /// Paint facelet `i` with the current brush. Centers are locked.
    pub fn paint(&mut self, i: usize) {
        if i % 9 != 4 {
            self.partial = self.partial.set(i, self.brush);
        }
    }

    /// Select the paint color.
    pub fn select(&mut self, face: Face) {
        self.brush = face;
    }

    /// Clear every non-center sticker back to unknown.
    pub fn clear(&mut self) {
        self.partial = PartialFacelets::new();
    }

    /// Current completion status.
    #[must_use]
    pub fn completion(&self) -> Completion {
        self.partial.analyze()
    }
}

/// Human status line for the input HUD.
#[must_use]
pub fn input_status(input: &InputState) -> String {
    match input.completion() {
        Completion::Unique(state) => {
            if state.is_solved() {
                "solved".to_string()
            } else {
                "ready - Enter to solve".to_string()
            }
        }
        Completion::NeedMore { known } => format!("{known}/48 painted"),
        Completion::Impossible(err) => format!("impossible - {err}"),
    }
}

/// Number keys `1..=6` select the paint color; `Delete` clears to unknown.
pub fn palette_keys(keys: Res<ButtonInput<KeyCode>>, mut input: ResMut<InputState>) {
    const DIGITS: [KeyCode; 6] = [
        KeyCode::Digit1,
        KeyCode::Digit2,
        KeyCode::Digit3,
        KeyCode::Digit4,
        KeyCode::Digit5,
        KeyCode::Digit6,
    ];
    for (i, key) in DIGITS.iter().enumerate() {
        if keys.just_pressed(*key) {
            input.select(PALETTE[i]);
        }
    }
    if keys.just_pressed(KeyCode::Delete) {
        input.clear();
    }
}

/// `Tab` toggles Input/Solve; `Enter` confirms input into the solvable cube.
///
/// Confirming only succeeds when the painted state is uniquely determined; the
/// status HUD explains why it is not otherwise.
pub fn mode_control(
    keys: Res<ButtonInput<KeyCode>>,
    mut mode: ResMut<AppMode>,
    mut input: ResMut<InputState>,
    mut cube: ResMut<CubeRes>,
) {
    let toggle = keys.just_pressed(KeyCode::Tab);
    let confirm = keys.just_pressed(KeyCode::Enter) || keys.just_pressed(KeyCode::NumpadEnter);

    match *mode {
        AppMode::Input => {
            if (toggle || confirm) && try_confirm(&input, &mut cube) {
                *mode = AppMode::Solve;
            }
        }
        AppMode::Solve => {
            if toggle {
                // Return to editing, seeded from the current cube.
                input.partial = PartialFacelets::from_facelets(&cube.0);
                *mode = AppMode::Input;
            }
        }
        // Camera-scan mode manages its own transitions (see `camera_scan`).
        AppMode::Camera => {}
    }
}

/// If the input is uniquely determined, write it into `cube` and report success.
fn try_confirm(input: &InputState, cube: &mut CubeRes) -> bool {
    if let Completion::Unique(state) = input.partial.analyze() {
        cube.0 = state.to_facelets();
        true
    } else {
        false
    }
}

/// While in input mode, paint the 3D stickers from the partial state (unknown
/// stickers show the neutral "unknown" material).
pub fn sync_input_stickers(
    input: Res<InputState>,
    mats: Res<StickerMaterials>,
    mut stickers: Query<(&Sticker, &mut MeshMaterial3d<StandardMaterial>)>,
) {
    for (sticker, mut material) in &mut stickers {
        let desired = match input.partial.get(sticker.facelet) {
            Some(face) => &mats.by_face[face.index()],
            None => &mats.unknown,
        };
        if material.0.id() != desired.id() {
            material.0 = desired.clone();
        }
    }
}

/// Observer: clicking a 3D sticker paints it (input mode only).
pub fn on_sticker_click(
    click: Trigger<Pointer<Click>>,
    stickers: Query<&Sticker>,
    mode: Res<AppMode>,
    mut input: ResMut<InputState>,
) {
    if *mode != AppMode::Input {
        return;
    }
    if let Ok(sticker) = stickers.get(click.target()) {
        input.paint(sticker.facelet);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_is_incomplete() {
        assert!(matches!(
            InputState::empty().completion(),
            Completion::NeedMore { .. }
        ));
    }

    #[test]
    fn painting_a_solved_cube_is_unique() {
        let mut s = InputState::empty();
        for i in 0..54 {
            s.select(Face::ALL[i / 9]);
            s.paint(i);
        }
        assert!(matches!(s.completion(), Completion::Unique(_)));
    }

    #[test]
    fn centers_are_locked() {
        let mut s = InputState::empty();
        s.select(Face::R);
        s.paint(4); // U center
        assert_eq!(s.partial.get(4), Some(Face::U));
    }

    #[test]
    fn select_changes_brush() {
        let mut s = InputState::empty();
        s.select(Face::B);
        assert_eq!(s.brush, Face::B);
    }

    #[test]
    fn clear_returns_to_centers_only() {
        let mut s = InputState::seeded(&Facelets::SOLVED);
        s.clear();
        assert_eq!(s.partial.known_count(), 0);
    }
}
