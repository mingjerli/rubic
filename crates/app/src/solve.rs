//! Solving and step playback.
//!
//! `Solve(Beginner)` solves with the layer-by-layer [`BeginnerSolver`];
//! `Solve(Optimal)` with the Kociemba [`OptimalSolver`] (built once, reused).
//! The resulting solution is stepped one move at a time: `PlayPause` toggles
//! auto-advance, `StepForward` / `StepBack` step (backward by animating the
//! inverse move). Each move
//! is enqueued on the shared [`TurnQueue`], which applies it to [`CubeRes`] when
//! the animation lands, so the cursor and the rendered state stay in lockstep.
//!
//! The pure playback math (flattening a solution, mapping moves to steps,
//! replaying to a cursor) lives in [`playback`] and is unit-tested.

use bevy::prelude::*;
use rubic_core::solver::BeginnerSolver;
use rubic_core::{Move, OptimalSolver, Solver};

use crate::action::{Action, SolverChoice};
use crate::mode::AppMode;
use crate::types::{CubeRes, TurnQueue};

/// Long-lived solver instances. [`OptimalSolver`] builds its pruning tables
/// once here so repeated solves are cheap.
#[derive(Resource)]
pub struct Solvers {
    /// The optimal (two-phase) solver.
    pub optimal: OptimalSolver,
    /// The layer-by-layer beginner solver.
    pub beginner: BeginnerSolver,
}

impl Default for Solvers {
    fn default() -> Self {
        Self {
            optimal: OptimalSolver::new(),
            beginner: BeginnerSolver,
        }
    }
}

/// Per-move label carried for HUD display.
#[derive(Debug, Clone)]
pub struct MoveLabel {
    /// 1-based index of the solution step this move belongs to.
    pub step: usize,
    /// The step's stage name.
    pub stage: &'static str,
}

/// A flattened, navigable solution.
#[derive(Debug, Clone)]
pub struct Player {
    /// Every move, in order.
    pub moves: Vec<Move>,
    /// Parallel labels, one per move.
    pub labels: Vec<MoveLabel>,
    /// Number of solution steps.
    pub step_count: usize,
    /// Moves applied so far (`0..=moves.len()`).
    pub cursor: usize,
    /// Whether auto-advance is on.
    pub playing: bool,
    /// Which solver produced this.
    pub solver_name: &'static str,
}

/// Holds the current solution, if any.
#[derive(Resource, Default)]
pub struct SolvePlayer {
    /// The active playback, or `None` before a solve.
    pub player: Option<Player>,
}

/// Pure playback helpers, unit-tested without Bevy.
pub mod playback {
    use super::{MoveLabel, Player};
    use rubic_core::{Move, Solution};

    /// Flatten a solution's steps into one move list.
    #[must_use]
    pub fn flatten_moves(sol: &Solution) -> Vec<Move> {
        sol.to_sequence().0
    }

    /// One [`MoveLabel`] per move, tying it back to its solution step.
    #[must_use]
    pub fn move_labels(sol: &Solution) -> Vec<MoveLabel> {
        let mut out = Vec::new();
        for (i, step) in sol.steps.iter().enumerate() {
            for _ in &step.moves {
                out.push(MoveLabel {
                    step: i + 1,
                    stage: step.stage.name(),
                });
            }
        }
        out
    }

    /// The cube state after applying the first `k` moves to `initial`.
    ///
    /// This is the "replay from the start to step `k`" reconstruction the spec
    /// mentions; the running app instead steps backward by animating the
    /// inverse move, so this pure form is exercised by the unit tests.
    #[cfg(test)]
    #[must_use]
    pub fn state_at(
        initial: rubic_core::Facelets,
        moves: &[Move],
        k: usize,
    ) -> rubic_core::Facelets {
        let mut f = initial;
        for &m in moves.iter().take(k) {
            f = f.apply(m);
        }
        f
    }

    /// Build a fresh player positioned at the start.
    #[must_use]
    pub fn build_player(sol: &Solution, solver_name: &'static str) -> Player {
        Player {
            moves: flatten_moves(sol),
            labels: move_labels(sol),
            step_count: sol.steps.len(),
            cursor: 0,
            playing: false,
            solver_name,
        }
    }
}

impl Player {
    /// Advance the cursor and return the move to animate, if one remains.
    pub fn next_move(&mut self) -> Option<Move> {
        let mv = self.moves.get(self.cursor).copied()?;
        self.cursor += 1;
        Some(mv)
    }

    /// Rewind the cursor and return the inverse move to animate.
    pub fn previous_move(&mut self) -> Option<Move> {
        let previous = self.cursor.checked_sub(1)?;
        let mv = self.moves.get(previous).copied()?;
        self.cursor = previous;
        Some(mv.inverse())
    }

    /// Total move count.
    #[must_use]
    pub fn total(&self) -> usize {
        self.moves.len()
    }

    /// Whether every move has been applied.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.cursor >= self.moves.len()
    }

    /// The label most relevant to the current cursor (the move about to be
    /// applied, or the final move when finished).
    #[must_use]
    pub fn current_label(&self) -> Option<&MoveLabel> {
        if self.moves.is_empty() {
            return None;
        }
        let idx = self.cursor.min(self.moves.len() - 1);
        self.labels.get(idx)
    }

    /// A one-line HUD summary of playback position.
    #[must_use]
    pub fn hud(&self) -> String {
        // Kept short so it fits the status corner without overrunning other UI;
        // the verbose step note is dropped (the stage name conveys the step).
        let base = format!("{} · {}/{}", self.solver_name, self.cursor, self.total());
        match self.current_label() {
            Some(l) => format!("{base} · step {}/{}: {}", l.step, self.step_count, l.stage),
            None => format!("{base} · solved"),
        }
    }
}

/// Startup: cache the solvers (this builds the optimal solver's tables once).
pub fn setup_solvers(mut commands: Commands) {
    commands.insert_resource(Solvers::default());
}

/// `Solve(choice)`: solve the current cube with the beginner / optimal solver.
/// Ignored while a turn is still animating.
pub fn solve_input(
    mut actions: EventReader<Action>,
    mode: Res<AppMode>,
    cube: Res<CubeRes>,
    solvers: Res<Solvers>,
    mut player: ResMut<SolvePlayer>,
    queue: Res<TurnQueue>,
) {
    for action in actions.read() {
        let Action::Solve(choice) = *action else {
            continue;
        };
        if *mode != AppMode::Solve || !queue.is_idle() {
            continue;
        }
        let Ok(state) = cube.0.validate() else {
            continue; // HUD already reports the invalid state.
        };
        let (result, name) = match choice {
            SolverChoice::Optimal => (solvers.optimal.solve(&state), "Optimal"),
            SolverChoice::Beginner => (solvers.beginner.solve(&state), "Beginner"),
        };
        if let Ok(solution) = result {
            player.player = Some(playback::build_player(&solution, name));
        }
    }
}

/// `PlayPause` toggles auto-advance; `StepForward` / `StepBack` step one move
/// (only when no turn is animating).
pub fn player_controls(
    mut actions: EventReader<Action>,
    mode: Res<AppMode>,
    mut player: ResMut<SolvePlayer>,
    mut queue: ResMut<TurnQueue>,
) {
    for action in actions.read() {
        let Some(p) = player.player.as_mut() else {
            continue;
        };
        if *mode != AppMode::Solve {
            continue;
        }
        match action {
            Action::PlayPause => p.playing = !p.playing,
            Action::StepForward | Action::StepBack if queue.is_idle() => {
                p.playing = false;
                let mv = if *action == Action::StepForward {
                    if p.finished() { None } else { p.next_move() }
                } else {
                    p.previous_move()
                };
                if let Some(mv) = mv {
                    queue.enqueue(mv);
                }
            }
            _ => {}
        }
    }
}

/// While playing, enqueue the next move whenever the queue goes idle.
pub fn auto_advance(mut player: ResMut<SolvePlayer>, mut queue: ResMut<TurnQueue>) {
    let Some(p) = player.player.as_mut() else {
        return;
    };
    if p.playing && queue.is_idle() {
        if let Some(mv) = p.next_move() {
            queue.enqueue(mv);
        } else {
            p.playing = false;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::playback::{build_player, flatten_moves, move_labels, state_at};
    use rubic_core::solver::BeginnerSolver;
    use rubic_core::{Facelets, Sequence, Solver};
    use std::str::FromStr;

    fn scrambled() -> Facelets {
        let seq = Sequence::from_str("R U R' U' F2 L D B'").unwrap();
        Facelets::SOLVED.apply_seq(&seq)
    }

    #[test]
    fn flatten_matches_move_count() {
        let state = scrambled().validate().unwrap();
        let sol = BeginnerSolver.solve(&state).unwrap();
        assert_eq!(flatten_moves(&sol).len(), sol.move_count());
    }

    #[test]
    fn labels_align_one_per_move() {
        let state = scrambled().validate().unwrap();
        let sol = BeginnerSolver.solve(&state).unwrap();
        assert_eq!(move_labels(&sol).len(), flatten_moves(&sol).len());
    }

    #[test]
    fn full_replay_solves_the_cube() {
        let start = scrambled();
        let state = start.validate().unwrap();
        let sol = BeginnerSolver.solve(&state).unwrap();
        let moves = flatten_moves(&sol);
        let end = state_at(start, &moves, moves.len());
        assert!(end.validate().unwrap().is_solved());
    }

    #[test]
    fn stepping_back_and_forth_is_consistent() {
        let start = scrambled();
        let state = start.validate().unwrap();
        let sol = BeginnerSolver.solve(&state).unwrap();
        let moves = flatten_moves(&sol);
        // state after k+1 moves, then undo the last move, equals state after k.
        for k in 0..moves.len() {
            let forward = state_at(start, &moves, k + 1);
            let back = forward.apply(moves[k].inverse());
            assert_eq!(back, state_at(start, &moves, k));
        }
    }

    #[test]
    fn player_replays_and_rewinds_without_crossing_boundaries() {
        let start = scrambled();
        let sol = BeginnerSolver.solve(&start.validate().unwrap()).unwrap();
        let mut player = build_player(&sol, "Beginner");
        let mut cube = start;
        assert_eq!(player.previous_move(), None);
        while let Some(mv) = player.next_move() {
            cube = cube.apply(mv);
        }
        assert_eq!(cube, Facelets::SOLVED);
        assert_eq!(player.cursor, player.total());
        assert_eq!(player.next_move(), None);
        while let Some(mv) = player.previous_move() {
            cube = cube.apply(mv);
        }
        assert_eq!(cube, start);
        assert_eq!(player.cursor, 0);
        assert_eq!(player.previous_move(), None);
    }

    #[test]
    fn empty_solution_has_no_playback_moves() {
        let mut player = build_player(&rubic_core::Solution::default(), "Beginner");
        assert_eq!(player.next_move(), None);
        assert_eq!(player.previous_move(), None);
        assert!(player.finished());
        assert_eq!(player.cursor, 0);
    }

    #[test]
    fn new_player_starts_at_zero() {
        let state = scrambled().validate().unwrap();
        let sol = BeginnerSolver.solve(&state).unwrap();
        let p = build_player(&sol, "Beginner");
        assert_eq!(p.cursor, 0);
        assert!(!p.playing);
        assert_eq!(p.total(), sol.move_count());
    }

    #[test]
    fn hud_mentions_solver_and_counter() {
        let state = scrambled().validate().unwrap();
        let sol = BeginnerSolver.solve(&state).unwrap();
        let p = build_player(&sol, "Beginner");
        let hud = p.hud();
        assert!(hud.contains("Beginner"));
        assert!(hud.contains(&format!("/{}", p.total())));
    }
}
