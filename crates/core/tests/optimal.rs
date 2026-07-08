//! Optimal (two-phase) solver tests. Compiled only with `--features optimal`.

#![cfg(feature = "optimal")]

use rubic_core::solver::BeginnerSolver;
use rubic_core::{CubeState, Facelets, OptimalSolver, Sequence, Solver, Stage};

fn scramble(s: &str) -> Facelets {
    Facelets::SOLVED.apply_seq(&s.parse::<Sequence>().unwrap())
}

#[test]
fn optimal_solves_random_scrambles() {
    let solver = OptimalSolver::new();
    let mut x: u64 = 0x1234_5678_9abc_def0;
    let mut next = || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let faces = ["U", "R", "F", "D", "L", "B"];
    let suffix = ["", "'", "2"];
    for _ in 0..30 {
        let mut tokens = Vec::new();
        for _ in 0..20 {
            let f = faces[(next() % 6) as usize];
            let s = suffix[(next() % 3) as usize];
            tokens.push(format!("{f}{s}"));
        }
        let seq: Sequence = tokens.join(" ").parse().unwrap();
        let f = Facelets::SOLVED.apply_seq(&seq);
        let state = f.validate().unwrap();
        let sol = solver.solve(&state).unwrap();
        assert_eq!(
            f.apply_seq(&sol.to_sequence()),
            Facelets::SOLVED,
            "optimal solution did not solve {seq}"
        );
        assert!(
            sol.move_count() <= 25,
            "two-phase solution unexpectedly long: {}",
            sol.move_count()
        );
        assert!(sol.steps.iter().all(|step| step.stage == Stage::Optimal));
    }
}

#[test]
fn optimal_is_shorter_than_beginner() {
    let f = scramble("R U2 F' L D B R' U D2 F L' B2 R F");
    let state = f.validate().unwrap();
    let optimal = OptimalSolver::new().solve(&state).unwrap();
    let beginner = BeginnerSolver.solve(&state).unwrap();
    assert_eq!(f.apply_seq(&optimal.to_sequence()), Facelets::SOLVED);
    assert!(
        optimal.move_count() < beginner.move_count(),
        "optimal {} should be shorter than beginner {}",
        optimal.move_count(),
        beginner.move_count()
    );
}

#[test]
fn optimal_rejects_unsolvable() {
    let solver = OptimalSolver::new();
    let mut co = [0u8; 8];
    co[0] = 1;
    let bad = CubeState::new_unchecked(
        [0, 1, 2, 3, 4, 5, 6, 7],
        co,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        [0; 12],
    );
    assert!(solver.solve(&bad).is_err());
}
