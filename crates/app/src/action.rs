//! Actions: what the user asks the app to do, whatever the source.
//!
//! Every input source is an adapter that emits [`Action`] events: the keyboard
//! (through [`keymap`]), the top bar and camera bar buttons, Net and palette
//! clicks, 3D sticker clicks, and drag-to-turn. The systems that change the cube
//! or the app mode consume Actions and never look at keys, so a button and its
//! key can't drift apart.
//!
//! Adapters run in [`ActionSources`]; consumers run after it, every frame, and
//! decide from the current mode whether an Action applies. (Running every frame
//! matters: an event outlives one frame, so a consumer gated by a run condition
//! could later pick up an Action meant for a different mode.)

use bevy::prelude::*;
use rubic_core::{Amount, Face, Move};

use crate::mode::{AppMode, InputStage};
use crate::paint::PALETTE;

/// Which solver to run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SolverChoice {
    Beginner,
    Optimal,
}

/// Something the user asks the app to do.
#[derive(Event, Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    // Setup.
    /// Replace the cube with a random scramble and go to Solve.
    Shuffle,
    /// From the method picker: paint a cube by hand, from blank.
    Manual,
    /// From the method picker: open the camera and start a Scan.
    StartCamera,
    /// Editing: take the entered cube into Solve (only when it is Ready).
    Confirm,
    /// Abandon the current cube (or Scan) and return to the method picker.
    StartOver,
    /// Solve: go back to Editing, seeded from the current cube.
    Edit,

    // Editing.
    /// Paint facelet `0..54` with the Brush.
    Paint(usize),
    SelectBrush(Face),
    /// Clear every painted sticker back to unknown.
    ClearPaint,

    // Scan.
    /// Capture (or retake) the face in view.
    Capture,
    NextFace,
    PrevFace,
    /// Discard every captured face and start the Scan again.
    RestartScan,

    // Solve.
    Turn(Move),
    /// Reset the cube to solved.
    ResetCube,
    Solve(SolverChoice),
    PlayPause,
    StepForward,
    StepBack,
}

/// Where every input adapter runs; Action consumers run after it.
#[derive(SystemSet, Clone, Debug, PartialEq, Eq, Hash)]
pub struct ActionSources;

/// Number keys `1..=6` pick the palette color at that position.
const PALETTE_DIGITS: [KeyCode; 6] = [
    KeyCode::Digit1,
    KeyCode::Digit2,
    KeyCode::Digit3,
    KeyCode::Digit4,
    KeyCode::Digit5,
    KeyCode::Digit6,
];

fn palette_face(key: KeyCode) -> Option<Face> {
    PALETTE_DIGITS
        .iter()
        .position(|&k| k == key)
        .map(|i| PALETTE[i])
}

/// Keys `U D L R F B` turn that face.
fn turn_face(key: KeyCode) -> Option<Face> {
    match key {
        KeyCode::KeyU => Some(Face::U),
        KeyCode::KeyD => Some(Face::D),
        KeyCode::KeyL => Some(Face::L),
        KeyCode::KeyR => Some(Face::R),
        KeyCode::KeyF => Some(Face::F),
        KeyCode::KeyB => Some(Face::B),
        _ => None,
    }
}

/// The Action a key press means in the current state, if any. `shift` reverses
/// a face turn.
#[must_use]
pub fn keymap(mode: AppMode, stage: InputStage, key: KeyCode, shift: bool) -> Option<Action> {
    use KeyCode as K;
    // Shuffle works from anywhere.
    if key == K::KeyG {
        return Some(Action::Shuffle);
    }
    match mode {
        AppMode::Input => match stage {
            InputStage::ChooseMethod => match key {
                K::KeyM => Some(Action::Manual),
                K::KeyC if cfg!(feature = "camera") => Some(Action::StartCamera),
                _ => None,
            },
            InputStage::Editing => match key {
                K::Escape => Some(Action::StartOver),
                K::Enter | K::NumpadEnter | K::Tab => Some(Action::Confirm),
                K::Delete => Some(Action::ClearPaint),
                _ => palette_face(key).map(Action::SelectBrush),
            },
        },
        AppMode::Camera => match key {
            K::Escape | K::Tab => Some(Action::StartOver),
            K::KeyR => Some(Action::RestartScan),
            K::Enter | K::NumpadEnter | K::Space => Some(Action::Capture),
            K::ArrowRight | K::KeyN => Some(Action::NextFace),
            K::ArrowLeft | K::KeyP => Some(Action::PrevFace),
            _ => None,
        },
        AppMode::Solve => match key {
            K::Tab => Some(Action::Edit),
            K::Backspace => Some(Action::ResetCube),
            K::Digit1 => Some(Action::Solve(SolverChoice::Beginner)),
            K::Digit2 => Some(Action::Solve(SolverChoice::Optimal)),
            K::Space => Some(Action::PlayPause),
            K::ArrowRight | K::KeyN => Some(Action::StepForward),
            K::ArrowLeft | K::KeyP => Some(Action::StepBack),
            _ => turn_face(key).map(|face| {
                let amount = if shift { Amount::Ccw } else { Amount::Cw };
                Action::Turn(Move { face, amount })
            }),
        },
    }
}

/// Keyboard adapter: each key pressed this frame becomes its Action.
pub fn keyboard_actions(
    keys: Res<ButtonInput<KeyCode>>,
    mode: Res<AppMode>,
    stage: Res<InputStage>,
    mut actions: EventWriter<Action>,
) {
    let shift = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    for &key in keys.get_just_pressed() {
        if let Some(action) = keymap(*mode, *stage, key, shift) {
            actions.write(action);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use InputStage::{ChooseMethod, Editing};

    fn key(mode: AppMode, stage: InputStage, key: KeyCode) -> Option<Action> {
        keymap(mode, stage, key, false)
    }

    #[test]
    fn g_shuffles_from_every_state() {
        for (mode, stage) in [
            (AppMode::Input, ChooseMethod),
            (AppMode::Input, Editing),
            (AppMode::Camera, ChooseMethod),
            (AppMode::Solve, Editing),
        ] {
            assert_eq!(key(mode, stage, KeyCode::KeyG), Some(Action::Shuffle));
        }
    }

    #[test]
    fn method_picker_keys_choose_a_setup_method() {
        assert_eq!(
            key(AppMode::Input, ChooseMethod, KeyCode::KeyM),
            Some(Action::Manual)
        );
        let camera = key(AppMode::Input, ChooseMethod, KeyCode::KeyC);
        assert_eq!(camera.is_some(), cfg!(feature = "camera"));
        // Nothing else on the picker: a stray key can't discard anything.
        assert_eq!(key(AppMode::Input, ChooseMethod, KeyCode::Escape), None);
        assert_eq!(key(AppMode::Input, ChooseMethod, KeyCode::Enter), None);
    }

    #[test]
    fn space_captures_while_scanning_and_plays_while_solving() {
        let space = |mode, stage| key(mode, stage, KeyCode::Space);
        assert_eq!(space(AppMode::Camera, ChooseMethod), Some(Action::Capture));
        assert_eq!(space(AppMode::Solve, Editing), Some(Action::PlayPause));
        assert_eq!(space(AppMode::Input, Editing), None);
    }

    #[test]
    fn r_restarts_a_scan_but_turns_the_r_face_while_solving() {
        assert_eq!(
            key(AppMode::Camera, ChooseMethod, KeyCode::KeyR),
            Some(Action::RestartScan)
        );
        assert_eq!(
            key(AppMode::Solve, Editing, KeyCode::KeyR),
            Some(Action::Turn("R".parse().unwrap()))
        );
        assert_eq!(
            keymap(AppMode::Solve, Editing, KeyCode::KeyR, true),
            Some(Action::Turn("R'".parse().unwrap()))
        );
    }

    #[test]
    fn digits_pick_a_color_while_editing_and_a_solver_while_solving() {
        assert_eq!(
            key(AppMode::Input, Editing, KeyCode::Digit1),
            Some(Action::SelectBrush(PALETTE[0]))
        );
        assert_eq!(
            key(AppMode::Input, Editing, KeyCode::Digit6),
            Some(Action::SelectBrush(PALETTE[5]))
        );
        assert_eq!(
            key(AppMode::Solve, Editing, KeyCode::Digit1),
            Some(Action::Solve(SolverChoice::Beginner))
        );
        assert_eq!(
            key(AppMode::Solve, Editing, KeyCode::Digit2),
            Some(Action::Solve(SolverChoice::Optimal))
        );
    }

    #[test]
    fn tab_confirms_edits_returns_to_editing_or_leaves_a_scan() {
        assert_eq!(
            key(AppMode::Input, Editing, KeyCode::Tab),
            Some(Action::Confirm)
        );
        assert_eq!(
            key(AppMode::Solve, Editing, KeyCode::Tab),
            Some(Action::Edit)
        );
        assert_eq!(
            key(AppMode::Camera, ChooseMethod, KeyCode::Tab),
            Some(Action::StartOver)
        );
    }

    #[test]
    fn face_keys_turn_all_six_faces_only_while_solving() {
        for (k, face) in [
            (KeyCode::KeyU, Face::U),
            (KeyCode::KeyD, Face::D),
            (KeyCode::KeyL, Face::L),
            (KeyCode::KeyR, Face::R),
            (KeyCode::KeyF, Face::F),
            (KeyCode::KeyB, Face::B),
        ] {
            let cw = Move {
                face,
                amount: Amount::Cw,
            };
            assert_eq!(key(AppMode::Solve, Editing, k), Some(Action::Turn(cw)));
            assert!(!matches!(
                key(AppMode::Input, Editing, k),
                Some(Action::Turn(_))
            ));
        }
    }
}
