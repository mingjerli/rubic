//! Random scrambles for Shuffle: the cube the player then solves (see
//! `flow`, which commits it and switches to Solve).

use rubic_core::{Amount, Face, Facelets, Move};

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
