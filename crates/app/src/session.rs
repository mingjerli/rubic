//! Shared cube mutations: keep animation, playback, and facelets consistent.

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rubic_core::{Facelets, Move};

use rubic_core::Solver;

use crate::action::SolverChoice;
use crate::flow::PlaybackCmd;
use crate::solve::{SolvePlayer, Solvers, playback::build_player};
use crate::types::{CubeRes, Cubie, TurnQueue};

/// State needed when a user changes the cube outside solution playback.
#[derive(SystemParam)]
pub struct CubeSession<'w, 's> {
    cube: ResMut<'w, CubeRes>,
    queue: ResMut<'w, TurnQueue>,
    player: ResMut<'w, SolvePlayer>,
    cubies: Query<'w, 's, (&'static Cubie, &'static mut Transform)>,
}

impl CubeSession<'_, '_> {
    /// The committed cube, excluding any turn still being animated.
    pub fn facelets(&self) -> Facelets {
        self.cube.0
    }

    /// Discard playback and unfinished turns, restoring the committed view.
    pub fn cancel_playback(&mut self) {
        self.player.player = None;
        self.queue.pending.clear();
        self.queue.active = None;
        for (cubie, mut transform) in &mut self.cubies {
            transform.translation = cubie.home;
            transform.rotation = Quat::IDENTITY;
        }
    }

    /// Replace the cube without leaving moves from the previous state queued.
    pub fn replace(&mut self, facelets: Facelets) {
        self.cancel_playback();
        self.cube.0 = facelets;
    }

    /// Start a manual turn only when idle, invalidating the stored solution.
    pub fn turn(&mut self, mv: Move) {
        if self.queue.is_idle() {
            self.player.player = None;
            self.queue.enqueue(mv);
        }
    }

    /// Run a Playback command. Solving and stepping wait for any animating
    /// turn to land (the command is dropped otherwise), so the player's cursor
    /// stays in lockstep with the committed cube.
    pub fn playback(&mut self, cmd: PlaybackCmd, solvers: Option<&Solvers>) {
        match cmd {
            PlaybackCmd::Solve(choice) => {
                let (Some(solvers), true) = (solvers, self.queue.is_idle()) else {
                    return;
                };
                let Ok(state) = self.cube.0.validate() else {
                    return; // The status line already reports the invalid cube.
                };
                let (result, name) = match choice {
                    SolverChoice::Optimal => (solvers.optimal.solve(&state), "Optimal"),
                    SolverChoice::Beginner => (solvers.beginner.solve(&state), "Beginner"),
                };
                if let Ok(solution) = result {
                    self.player.player = Some(build_player(&solution, name));
                }
            }
            PlaybackCmd::PlayPause => {
                if let Some(p) = self.player.player.as_mut() {
                    p.playing = !p.playing;
                }
            }
            PlaybackCmd::StepForward | PlaybackCmd::StepBack => {
                let (Some(p), true) = (self.player.player.as_mut(), self.queue.is_idle()) else {
                    return;
                };
                p.playing = false;
                let mv = if cmd == PlaybackCmd::StepForward {
                    if p.finished() { None } else { p.next_move() }
                } else {
                    p.previous_move()
                };
                if let Some(mv) = mv {
                    self.queue.enqueue(mv);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests;
