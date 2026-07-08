//! Layer-turn animation.
//!
//! Drains the shared [`TurnQueue`]: for each queued [`rubic_core::Move`] it
//! rotates that face's nine cubies about the face axis over a short duration,
//! then, on completion, applies the move to [`CubeRes`] (the renderer repaints
//! stickers from that) and snaps the cubies back to their home transforms.
//! Because the renderer is facelet-driven, cubies never actually permute - they
//! visually sweep 90 degrees and reset, which reads as a real turn while
//! keeping state management trivial.

use bevy::prelude::*;

use crate::geometry::{self, cubie_in_layer};
use crate::types::{ActiveTurn, CubeRes, Cubie, TurnQueue};

/// Seconds a single quarter/half turn takes to animate.
const TURN_DURATION: f32 = 0.16;

/// Smoothstep easing so turns start and stop gently.
fn ease(t: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// Advance any active turn and start the next queued one.
pub fn drive_turns(
    time: Res<Time>,
    mut queue: ResMut<TurnQueue>,
    mut cube: ResMut<CubeRes>,
    mut cubies: Query<(&Cubie, &mut Transform)>,
) {
    // Start a new turn if idle.
    if queue.active.is_none() {
        if let Some(mv) = queue.pending.pop_front() {
            let (axis, total_angle) = geometry::move_rotation(mv);
            queue.active = Some(ActiveTurn {
                mv,
                elapsed: 0.0,
                duration: TURN_DURATION,
                axis: Vec3::from_array(axis),
                total_angle,
                applied: 0.0,
            });
        }
    }

    let Some(mut turn) = queue.active else {
        return;
    };

    turn.elapsed += time.delta_secs();
    let progress = (turn.elapsed / turn.duration).clamp(0.0, 1.0);
    let target = turn.total_angle * ease(progress);
    let delta = target - turn.applied;

    if delta.abs() > f32::EPSILON {
        let rotation = Quat::from_axis_angle(turn.axis, delta);
        for (cubie, mut transform) in &mut cubies {
            if cubie_in_layer(cubie.cell, turn.mv.face) {
                transform.rotate_around(Vec3::ZERO, rotation);
            }
        }
        turn.applied = target;
    }

    if progress >= 1.0 {
        // Land the state, then reset the visual layer to home.
        cube.0 = cube.0.apply(turn.mv);
        for (cubie, mut transform) in &mut cubies {
            if cubie_in_layer(cubie.cell, turn.mv.face) {
                transform.translation = cubie.home;
                transform.rotation = Quat::IDENTITY;
            }
        }
        queue.active = None;
    } else {
        queue.active = Some(turn);
    }
}

#[cfg(test)]
mod tests {
    use super::ease;

    #[test]
    fn ease_endpoints() {
        assert!((ease(0.0)).abs() < 1e-6);
        assert!((ease(1.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn ease_is_monotonic_and_clamped() {
        assert!((ease(-1.0)).abs() < 1e-6);
        assert!((ease(2.0) - 1.0).abs() < 1e-6);
        let mut prev = ease(0.0);
        for i in 1..=10 {
            let t = i as f32 / 10.0;
            let v = ease(t);
            assert!(v >= prev - 1e-6, "ease must not decrease");
            prev = v;
        }
    }

    #[test]
    fn ease_midpoint_is_half() {
        assert!((ease(0.5) - 0.5).abs() < 1e-6);
    }
}
