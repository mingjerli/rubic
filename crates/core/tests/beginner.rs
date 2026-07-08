//! Property tests for the beginner (layer-by-layer) solver.
//!
//! A deterministic xorshift PRNG generates random scrambles; every resulting
//! cube must be solved by [`BeginnerSolver`], using only layer-by-layer stages
//! in method order. An unsolvable cube must be rejected.

use rubic_core::solver::BeginnerSolver;
use rubic_core::{Amount, CubeState, Face, Facelets, Move, Sequence, SolveError, Solver, Stage};

/// Deterministic xorshift64 PRNG (no external crates).
struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    fn index(&mut self, n: u64) -> usize {
        usize::try_from(self.next() % n).expect("index fits usize")
    }
}

const FACES: [Face; 6] = [Face::U, Face::R, Face::F, Face::D, Face::L, Face::B];
const AMOUNTS: [Amount; 3] = [Amount::Cw, Amount::Ccw, Amount::Double];

fn random_scramble(rng: &mut Rng, len: usize) -> Sequence {
    let mut moves = Vec::with_capacity(len);
    for _ in 0..len {
        let face = FACES[rng.index(6)];
        let amount = AMOUNTS[rng.index(3)];
        moves.push(Move { face, amount });
    }
    Sequence(moves)
}

/// The method-order index of a stage (must be non-decreasing across a solution).
fn stage_order(stage: Stage) -> u8 {
    match stage {
        Stage::BottomCross => 0,
        Stage::BottomCorners => 1,
        Stage::MiddleEdges => 2,
        Stage::TopCross => 3,
        Stage::TopFace => 4,
        Stage::TopCorners => 5,
        Stage::TopEdges => 6,
        Stage::Optimal => 255,
    }
}

#[test]
fn solves_1000_random_scrambles() {
    let mut rng = Rng(0x9e37_79b9_7f4a_7c15);
    for i in 0..1000 {
        let scramble = random_scramble(&mut rng, 25);
        let f = Facelets::SOLVED.apply_seq(&scramble);
        let state = f
            .validate()
            .unwrap_or_else(|e| panic!("scramble {i} invalid: {e}"));

        let solution = BeginnerSolver
            .solve(&state)
            .unwrap_or_else(|e| panic!("scramble {i} unsolved: {e}"));

        // Solution actually solves the cube.
        let solved = f.apply_seq(&solution.to_sequence());
        assert_eq!(solved, Facelets::SOLVED, "scramble {i} not solved");

        // Every step uses a layer-by-layer stage, in non-decreasing order.
        let mut last = 0u8;
        for step in &solution.steps {
            assert_ne!(step.stage, Stage::Optimal, "scramble {i} used Optimal");
            let order = stage_order(step.stage);
            assert!(order >= last, "scramble {i} stages out of order");
            last = order;
        }
    }
}

#[test]
fn unsolvable_cube_is_rejected() {
    // A single twisted corner: valid piece layout, but unsolvable.
    let mut co = [0u8; 8];
    co[0] = 1;
    let state = CubeState::new_unchecked(
        [0, 1, 2, 3, 4, 5, 6, 7],
        co,
        [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        [0; 12],
    );
    assert_eq!(BeginnerSolver.solve(&state), Err(SolveError::Unsolvable));
}
