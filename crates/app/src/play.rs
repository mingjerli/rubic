//! Play by dragging: in Solve mode, drag across a sticker to turn its layer.
//!
//! A drag that starts on a sticker picks the cube axis the drag runs along, and
//! turns the face-layer perpendicular to it that contains that sticker — the
//! standard twisty-puzzle gesture. Works for mouse and touch (both arrive as
//! Bevy pointer drag events). While such a drag is active, camera orbit is
//! suppressed so the two don't fight.

use bevy::prelude::*;
use rubic_core::{Amount, Face, Move};

use crate::geometry::facelet_geometry;
use crate::mode::AppMode;
use crate::types::{MainCamera, Sticker, TurnQueue};

/// Set while a layer-turning drag is in progress, so [`crate::camera`] doesn't
/// also orbit.
#[derive(Resource, Default)]
pub struct OrbitSuppressed(pub bool);

/// The six signed unit cube axes.
const AXES: [[i32; 3]; 6] = [
    [1, 0, 0],
    [-1, 0, 0],
    [0, 1, 0],
    [0, -1, 0],
    [0, 0, 1],
    [0, 0, -1],
];

fn ivec3(v: [i32; 3]) -> Vec3 {
    Vec3::new(v[0] as f32, v[1] as f32, v[2] as f32)
}

fn face_from_normal(v: [i32; 3]) -> Option<Face> {
    Some(match v {
        [0, 1, 0] => Face::U,
        [1, 0, 0] => Face::R,
        [0, 0, 1] => Face::F,
        [0, -1, 0] => Face::D,
        [-1, 0, 0] => Face::L,
        [0, 0, -1] => Face::B,
        _ => return None,
    })
}

/// The signed cube axis (perpendicular to `n`) best aligned with `dir`.
fn axis_along(dir: Vec3, n: Vec3) -> Option<Vec3> {
    let mut best: Option<Vec3> = None;
    let mut best_dot = 0.3; // require a clear direction
    for a in AXES {
        let av = ivec3(a);
        if av.dot(n).abs() > 0.5 {
            continue; // parallel to the face normal
        }
        let d = av.dot(dir);
        if d > best_dot {
            best_dot = d;
            best = Some(av);
        }
    }
    best
}

/// Turn implied by a drag of `screen` pixels starting on facelet `facelet`,
/// given the camera orientation. `None` for a tiny drag or a middle slice
/// (which isn't a face move).
#[must_use]
pub fn move_from_drag(facelet: usize, screen: Vec2, cam: &Transform) -> Option<Move> {
    if screen.length() < 6.0 {
        return None; // treat as a tap/click, not a drag
    }
    let (cubie, normal) = facelet_geometry(facelet);
    let n = ivec3(normal);
    // Screen delta -> world direction (screen y points down).
    let world = *cam.right() * screen.x - *cam.up() * screen.y;
    let in_plane = (world - n * world.dot(n)).normalize_or_zero();
    let a = axis_along(in_plane, n)?;
    // Rotation axis of the layer, and the layer coordinate this sticker sits at.
    let r = a.cross(n);
    let ri = [r.x.round() as i32, r.y.round() as i32, r.z.round() as i32];
    let coord = cubie[0] * ri[0] + cubie[1] * ri[1] + cubie[2] * ri[2];
    if coord == 0 {
        return None; // middle slice: no face turn
    }
    let s = coord.signum();
    let face = face_from_normal([ri[0] * s, ri[1] * s, ri[2] * s])?;
    // The turned layer should follow the drag (verified on-screen: the grabbed
    // sticker moves the way you swipe).
    let amount = if coord > 0 { Amount::Cw } else { Amount::Ccw };
    Some(Move { face, amount })
}

/// Observer: a drag beginning on a sticker (in Solve mode) suppresses orbit.
pub fn on_drag_start(
    drag: Trigger<Pointer<DragStart>>,
    stickers: Query<&Sticker>,
    mode: Res<AppMode>,
    mut suppressed: ResMut<OrbitSuppressed>,
) {
    if *mode == AppMode::Solve && stickers.get(drag.target()).is_ok() {
        suppressed.0 = true;
    }
}

/// Observer: a drag ending on a sticker (in Solve mode) turns its layer.
pub fn on_drag_end(
    drag: Trigger<Pointer<DragEnd>>,
    stickers: Query<&Sticker>,
    cam: Query<&GlobalTransform, With<MainCamera>>,
    mode: Res<AppMode>,
    mut queue: ResMut<TurnQueue>,
    mut suppressed: ResMut<OrbitSuppressed>,
) {
    suppressed.0 = false;
    if *mode != AppMode::Solve {
        return;
    }
    let (Ok(sticker), Ok(cam)) = (stickers.get(drag.target()), cam.single()) else {
        return;
    };
    if !queue.is_idle() {
        return; // one turn at a time
    }
    if let Some(mv) = move_from_drag(sticker.facelet, drag.distance, &cam.compute_transform()) {
        queue.enqueue(mv);
    }
}
