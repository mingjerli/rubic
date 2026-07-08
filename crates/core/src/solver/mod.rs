//! Solvers: turn a solved-checked cube into a sequence of human-readable steps.
//!
//! Two implementations share the [`Solver`] trait so the app can offer the
//! user a choice: [`beginner::BeginnerSolver`] (layer-by-layer, the same method
//! as the printable cheat sheet) and, later, an optimal solver. Each `Step`
//! carries its move list plus a stage label and human note, which the cheat
//! sheet and the animation both consume.

pub mod beginner;
mod cube;
#[cfg(feature = "optimal")]
pub mod optimal;
mod search;

use crate::moves::{Move, Sequence};
use crate::state::CubeState;

pub use beginner::BeginnerSolver;
#[cfg(feature = "optimal")]
pub use optimal::OptimalSolver;

/// A solver that produces a step-by-step solution for a solvable cube.
pub trait Solver {
    /// Solve `cube`, returning ordered steps whose moves, applied in order,
    /// reach the solved state.
    ///
    /// # Errors
    /// Returns [`SolveError`] if the cube is not solvable or the solver cannot
    /// complete it.
    fn solve(&self, cube: &CubeState) -> Result<Solution, SolveError>;
}

/// The stage of the human method a step belongs to (also used to group the
/// cheat sheet). `Optimal` is used by the optimal solver, which has no stages.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stage {
    /// Bottom-layer cross.
    BottomCross,
    /// Bottom-layer corners (first layer complete).
    BottomCorners,
    /// Middle-layer edges.
    MiddleEdges,
    /// Top cross (last-layer edge orientation).
    TopCross,
    /// Top face (last-layer corner orientation).
    TopFace,
    /// Permute last-layer corners.
    TopCorners,
    /// Permute last-layer edges.
    TopEdges,
    /// A single stage produced by the optimal solver.
    Optimal,
}

/// One step of a solution: a short move sequence with human context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Step {
    /// The moves that make up this step.
    pub moves: Vec<Move>,
    /// Which stage of the method this step belongs to.
    pub stage: Stage,
    /// A short human explanation of what this step accomplishes.
    pub note: String,
}

/// A full solution as an ordered list of steps.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Solution {
    /// The ordered steps.
    pub steps: Vec<Step>,
}

impl Solution {
    /// Flatten every step's moves into one sequence, in order.
    #[must_use]
    pub fn to_sequence(&self) -> Sequence {
        Sequence(
            self.steps
                .iter()
                .flat_map(|s| s.moves.iter().copied())
                .collect(),
        )
    }

    /// Total number of moves across all steps.
    #[must_use]
    pub fn move_count(&self) -> usize {
        self.steps.iter().map(|s| s.moves.len()).sum()
    }
}

/// Why a solve failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveError {
    /// The cube is not in a solvable state.
    Unsolvable,
}

impl std::fmt::Display for SolveError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SolveError::Unsolvable => f.write_str("cube is not solvable"),
        }
    }
}

impl std::error::Error for SolveError {}
