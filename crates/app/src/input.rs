//! Manual play and reset.
//!
//! `Turn` Actions (face keys, or dragging a sticker) turn a face; `ResetCube`
//! resets to a solved cube. Turns are enqueued on the shared
//! [`crate::types::TurnQueue`] (only while it is idle, so animations never
//! overlap) and any active solve playback is discarded, since a manual turn
//! diverges from the stored solution.
//!
//! Sticker painting is handled separately by [`crate::paint`].

use bevy::prelude::*;
use rubic_core::Facelets;

use crate::action::Action;
use crate::mode::AppMode;
use crate::session::CubeSession;

/// Apply manual turns and the reset in Solve mode.
pub fn manual_input(
    mut actions: EventReader<Action>,
    mode: Res<AppMode>,
    mut session: CubeSession,
) {
    for action in actions.read() {
        if *mode != AppMode::Solve {
            continue;
        }
        match *action {
            Action::ResetCube => session.replace(Facelets::SOLVED),
            Action::Turn(mv) => session.turn(mv),
            _ => {}
        }
    }
}
