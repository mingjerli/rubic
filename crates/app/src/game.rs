//! Playable game: generate a random scramble and drop into play mode.
//!
//! Rather than only entering a cube by hand or camera, `G` (or the "New game"
//! button) scrambles a solved cube and switches to Solve mode, where the player
//! turns faces to solve it (and can still ask a solver for help).

use bevy::prelude::*;
use rubic_core::{Amount, Face, Facelets, Move, PartialFacelets};

use crate::mode::AppMode;
use crate::paint::InputState;
use crate::session::CubeSession;

/// Number of face turns in a generated scramble.
const SCRAMBLE_LEN: usize = 25;

/// Build a random scrambled (but always solvable) cube from a seed. Uses a tiny
/// xorshift over the seed to pick moves — no RNG dependency — and never turns
/// the same face twice in a row, so the scramble stays effective.
#[must_use]
pub fn scrambled_cube(seed: u64) -> Facelets {
    const AMOUNTS: [Amount; 3] = [Amount::Cw, Amount::Ccw, Amount::Double];

    let mut state = seed | 1; // avoid the all-zero xorshift fixed point
    let mut cube = Facelets::SOLVED;
    let mut last = usize::MAX;
    let mut n = 0;
    while n < SCRAMBLE_LEN {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let face = (state % 6) as usize;
        if face == last {
            continue;
        }
        last = face;
        let turn = ((state >> 8) % 3) as usize;
        cube = cube.apply(Move {
            face: Face::ALL[face],
            amount: AMOUNTS[turn],
        });
        n += 1;
    }
    cube
}

/// `G` scrambles the cube into a fresh puzzle and switches to play (Solve) mode.
pub fn scramble_input(
    keys: Res<ButtonInput<KeyCode>>,
    time: Res<Time>,
    mut session: CubeSession,
    mut input: ResMut<InputState>,
    mut mode: ResMut<AppMode>,
    mut nonce: Local<u64>,
) {
    if !keys.just_pressed(KeyCode::KeyG) {
        return;
    }
    // Vary the seed across presses even within the same frame.
    *nonce = nonce.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let seed = (time.elapsed().as_nanos() as u64) ^ *nonce;
    let scrambled = scrambled_cube(seed);
    session.replace(scrambled);
    input.partial = PartialFacelets::from_facelets(&scrambled);
    *mode = AppMode::Solve;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scramble_is_valid_and_not_solved() {
        for seed in [1u64, 42, 9999, 0xDEAD_BEEF] {
            let cube = scrambled_cube(seed);
            assert!(cube.validate().is_ok(), "scramble must be solvable");
            assert_ne!(cube, Facelets::SOLVED, "scramble must not be solved");
        }
    }

    #[test]
    fn different_seeds_give_different_scrambles() {
        assert_ne!(scrambled_cube(1), scrambled_cube(2));
    }
}
