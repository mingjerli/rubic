//! Manual play and reset.
//!
//! Keys `U D L R F B` turn that face clockwise; holding `Shift` turns it
//! counter-clockwise. `Backspace` resets to a solved cube. Turns are enqueued
//! on the shared [`TurnQueue`] (only while it is idle, so animations never
//! overlap) and any active solve playback is discarded, since a manual turn
//! diverges from the stored solution.
//!
//! Sticker painting via mouse picking is deferred (see the crate-level notes in
//! `main.rs`); the starting state is configured through the CLI instead, and
//! the live validation HUD covers the on-screen status requirement.

use bevy::prelude::*;
use rubic_core::{Amount, Face, Facelets, Move};

use crate::solve::SolvePlayer;
use crate::types::{CubeRes, TurnQueue};

/// Map a pressed key to the face it turns, if any.
fn key_to_face(key: KeyCode) -> Option<Face> {
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

/// The six face keys, checked each frame.
const FACE_KEYS: [KeyCode; 6] = [
    KeyCode::KeyU,
    KeyCode::KeyD,
    KeyCode::KeyL,
    KeyCode::KeyR,
    KeyCode::KeyF,
    KeyCode::KeyB,
];

/// Handle manual face turns and the reset key.
pub fn manual_input(
    keys: Res<ButtonInput<KeyCode>>,
    mut queue: ResMut<TurnQueue>,
    mut cube: ResMut<CubeRes>,
    mut player: ResMut<SolvePlayer>,
) {
    if keys.just_pressed(KeyCode::Backspace) {
        cube.0 = Facelets::SOLVED;
        queue.pending.clear();
        queue.active = None;
        player.player = None;
        return;
    }

    if !queue.is_idle() {
        return;
    }

    let ccw = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let amount = if ccw { Amount::Ccw } else { Amount::Cw };

    for key in FACE_KEYS {
        if keys.just_pressed(key) {
            if let Some(face) = key_to_face(key) {
                queue.enqueue(Move { face, amount });
                // A manual turn invalidates any stored solution.
                player.player = None;
                return;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn face_keys_map_to_all_six_faces() {
        let faces: Vec<Face> = FACE_KEYS.iter().filter_map(|&k| key_to_face(k)).collect();
        assert_eq!(faces.len(), 6);
        for face in Face::ALL {
            assert!(faces.contains(&face), "missing {}", face.to_char());
        }
    }

    #[test]
    fn non_face_key_maps_to_none() {
        assert_eq!(key_to_face(KeyCode::Space), None);
        assert_eq!(key_to_face(KeyCode::Backspace), None);
    }
}
