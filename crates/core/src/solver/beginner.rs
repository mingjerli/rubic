//! Beginner (layer-by-layer) solver.
//!
//! Placeholder: the full staged implementation is added in its own TDD cycle.

use crate::solver::{Solution, SolveError, Solver};
use crate::state::CubeState;

/// Layer-by-layer solver producing human-readable, stage-labelled steps.
#[derive(Debug, Default, Clone, Copy)]
pub struct BeginnerSolver;

impl Solver for BeginnerSolver {
    fn solve(&self, cube: &CubeState) -> Result<Solution, SolveError> {
        if cube.solvability().is_err() {
            return Err(SolveError::Unsolvable);
        }
        // TODO: staged layer-by-layer implementation.
        unimplemented!("beginner solver not yet implemented")
    }
}
