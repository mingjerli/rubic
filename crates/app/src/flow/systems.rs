//! The Bevy side of the Flow: one system feeds each [`Action`] through
//! [`Flow::step`] and applies the resulting [`Effect`]s.

use std::collections::VecDeque;

use bevy::prelude::*;

use super::{Effect, Flow, FlowKind, StepContext};
use crate::action::Action;
use crate::session::CubeSession;
use crate::solve::Solvers;

/// Step the Flow through this frame's Actions and apply their Effects.
///
/// An Effect can fail in a way the Flow must hear about: if the camera won't
/// open, a `StartOver` is fed back in the same pass, so the app never lingers
/// in a Scan with no camera.
pub fn apply_actions(
    mut actions: EventReader<Action>,
    time: Res<Time>,
    solvers: Option<Res<Solvers>>,
    mut flow: ResMut<Flow>,
    mut session: CubeSession,
    #[cfg(feature = "camera")] mut feed: NonSendMut<crate::camera_scan::CameraFeed>,
    mut nonce: Local<u64>,
) {
    let mut pending: VecDeque<Action> = actions.read().copied().collect();
    while let Some(action) = pending.pop_front() {
        // Vary the Shuffle seed across Actions, even within one frame.
        *nonce = nonce.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let ctx = StepContext {
            cube: session.facelets(),
            seed: (time.elapsed().as_nanos() as u64) ^ *nonce,
        };
        let (next, effects) = std::mem::take(&mut *flow).step(action, &ctx);
        *flow = next;
        for effect in effects {
            let follow_up: Option<Action> = match effect {
                Effect::ReplaceCube(cube) => {
                    session.replace(cube);
                    None
                }
                Effect::CancelPlayback => {
                    session.cancel_playback();
                    None
                }
                Effect::Turn(mv) => {
                    session.turn(mv);
                    None
                }
                Effect::Playback(cmd) => {
                    session.playback(cmd, solvers.as_deref());
                    None
                }
                #[cfg(feature = "camera")]
                Effect::OpenCamera => {
                    if feed.0.is_none() {
                        feed.0 = crate::camera_scan::open_source();
                    }
                    feed.0.is_none().then_some(Action::StartOver)
                }
                #[cfg(feature = "camera")]
                Effect::CloseCamera => {
                    feed.0 = None;
                    None
                }
                #[cfg(not(feature = "camera"))]
                Effect::OpenCamera | Effect::CloseCamera => None,
            };
            pending.extend(follow_up);
        }
    }
}

/// Run condition: solving.
#[must_use]
pub fn in_solving(flow: Res<Flow>) -> bool {
    flow.kind() == FlowKind::Solving
}

/// Run condition: editing a cube (painting, or reviewing a Scan).
#[must_use]
pub fn in_editing(flow: Res<Flow>) -> bool {
    flow.kind() == FlowKind::Editing
}
