//! Manual play and reset.
//!
//! Keys `U D L R F B` turn that face clockwise; holding `Shift` turns it
//! counter-clockwise. `Backspace` resets to a solved cube. Turns are enqueued
//! on the shared [`crate::types::TurnQueue`] (only while it is idle, so animations never
//! overlap) and any active solve playback is discarded, since a manual turn
//! diverges from the stored solution.
//!
//! Sticker painting is handled separately by [`crate::paint`].

use bevy::prelude::*;
use rubic_core::{Amount, Face, Facelets, Move};

use crate::session::CubeSession;

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
pub fn manual_input(keys: Res<ButtonInput<KeyCode>>, mut session: CubeSession) {
    if keys.just_pressed(KeyCode::Backspace) {
        session.replace(Facelets::SOLVED);
        return;
    }

    let ccw = keys.pressed(KeyCode::ShiftLeft) || keys.pressed(KeyCode::ShiftRight);
    let amount = if ccw { Amount::Ccw } else { Amount::Cw };

    for key in FACE_KEYS {
        if keys.just_pressed(key) {
            if let Some(face) = key_to_face(key) {
                session.turn(Move { face, amount });
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
