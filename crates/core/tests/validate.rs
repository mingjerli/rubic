//! Cubie-model extraction and solvability tests.
//!
//! The strongest check is `all_engine_scrambles_are_solvable`: any state
//! reachable by real moves must validate, which pins down that the corner/edge
//! orientation and parity conventions are self-consistent.

use rubic_core::{CubeError, CubeState, Facelets, Sequence};

fn scramble(s: &str) -> Facelets {
    Facelets::SOLVED.apply_seq(&s.parse::<Sequence>().unwrap())
}

const ID_C: [u8; 8] = [0, 1, 2, 3, 4, 5, 6, 7];
const ID_E: [u8; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11];

#[test]
fn solved_validates_to_identity() {
    let state = Facelets::SOLVED.validate().unwrap();
    assert_eq!(state, CubeState::SOLVED);
    assert!(state.is_solved());
}

#[test]
fn solved_state_paints_solved_facelets() {
    assert_eq!(CubeState::SOLVED.to_facelets(), Facelets::SOLVED);
}

#[test]
fn validate_round_trips_through_facelets() {
    let f = scramble("R U2 F' L D B R' U D2 F");
    let state = f.validate().unwrap();
    assert_eq!(state.to_facelets(), f);
}

#[test]
fn all_engine_scrambles_are_solvable() {
    // Deterministic xorshift PRNG — no external deps, reproducible.
    let mut x: u64 = 0x9E37_79B9_7F4A_7C15;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let faces = ["U", "R", "F", "D", "L", "B"];
    let suffix = ["", "'", "2"];
    for _ in 0..500 {
        let mut tokens = Vec::new();
        for _ in 0..25 {
            let f = faces[(next() % 6) as usize];
            let s = suffix[(next() % 3) as usize];
            tokens.push(format!("{f}{s}"));
        }
        let seq: Sequence = tokens.join(" ").parse().unwrap();
        let cube = Facelets::SOLVED.apply_seq(&seq);
        assert!(
            cube.validate().is_ok(),
            "engine scramble reported unsolvable: {seq}"
        );
    }
}

#[test]
fn single_corner_twist_is_unsolvable() {
    let mut co = [0u8; 8];
    co[0] = 1;
    let bad = CubeState::new_unchecked(ID_C, co, ID_E, [0; 12]).to_facelets();
    assert_eq!(bad.validate().unwrap_err(), CubeError::CornerTwist);
}

#[test]
fn single_edge_flip_is_unsolvable() {
    let mut eo = [0u8; 12];
    eo[0] = 1;
    let bad = CubeState::new_unchecked(ID_C, [0; 8], ID_E, eo).to_facelets();
    assert_eq!(bad.validate().unwrap_err(), CubeError::EdgeFlip);
}

#[test]
fn two_swapped_corners_is_unsolvable() {
    let mut cp = ID_C;
    cp.swap(0, 1);
    let bad = CubeState::new_unchecked(cp, [0; 8], ID_E, [0; 12]).to_facelets();
    assert_eq!(bad.validate().unwrap_err(), CubeError::PermutationParity);
}

#[test]
fn nonsense_colors_are_invalid() {
    let mut chars: Vec<char> = Facelets::SOLVED.to_string().chars().collect();
    chars[0] = 'R';
    let f: Facelets = chars.into_iter().collect::<String>().parse().unwrap();
    assert!(f.validate().is_err());
}
