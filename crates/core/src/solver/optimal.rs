//! Optimal solver: Kociemba's two-phase algorithm via the `kewb` crate.
//!
//! Produces a short solution (typically ~20 moves) as a single
//! [`Stage::Optimal`] step. Enabled by the `optimal` cargo feature so the core
//! stays dependency-light when it is not needed.
//!
//! Our facelet string ([`Facelets`](crate::Facelets) in URFDLB order) matches
//! `kewb`'s facelet convention exactly, so it is fed straight through with no
//! remapping. Correctness is guaranteed end-to-end by the property test, which
//! applies every returned solution through our own move engine.

use crate::color::Face;
use crate::moves::{Amount, Move};
use crate::solver::{Solution, SolveError, Solver, Stage, Step};
use crate::state::CubeState;
use kewb::{CubieCube, DataTable, FaceCube, Solver as KewbSolver};

/// Kociemba two-phase optimal solver.
///
/// Holds the generated move/pruning tables so repeated solves are cheap; build
/// one with [`OptimalSolver::new`] (table generation happens once, there).
pub struct OptimalSolver {
    table: DataTable,
    max_length: u8,
}

impl OptimalSolver {
    /// Build the solver, generating the two-phase tables (a one-time cost).
    #[must_use]
    pub fn new() -> Self {
        Self {
            table: DataTable::default(),
            max_length: 25,
        }
    }
}

impl Default for OptimalSolver {
    fn default() -> Self {
        Self::new()
    }
}

impl Solver for OptimalSolver {
    fn solve(&self, cube: &CubeState) -> Result<Solution, SolveError> {
        if cube.solvability().is_err() {
            return Err(SolveError::Unsolvable);
        }
        let facelet_str = cube.to_facelets().to_string();
        let face = FaceCube::try_from(facelet_str.as_str()).map_err(|_| SolveError::Unsolvable)?;
        let kewb_cube = CubieCube::try_from(&face).map_err(|_| SolveError::Unsolvable)?;

        // With `timeout = None`, kewb returns as soon as it finds a solution,
        // iterating depth upward so the result is short and the call is fast.
        let mut solver = KewbSolver::new(&self.table, self.max_length, None);
        let solution = solver.solve(kewb_cube).ok_or(SolveError::Unsolvable)?;

        let moves = solution.get_all_moves().into_iter().map(map_move).collect();
        Ok(Solution {
            steps: vec![Step {
                moves,
                stage: Stage::Optimal,
                note: "Optimal two-phase (Kociemba) solution.".to_string(),
            }],
        })
    }
}

/// Map a `kewb` move to our [`Move`] type via standard notation.
///
/// `kewb`'s `Display` always emits valid notation (`U`, `U2`, `U'`), so parsing
/// always succeeds; the fallback is unreachable.
fn map_move(m: kewb::Move) -> Move {
    m.to_string().parse().unwrap_or(Move {
        face: Face::U,
        amount: Amount::Cw,
    })
}
