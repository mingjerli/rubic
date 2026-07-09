//! Orbit / pan / zoom camera.
//!
//! Left-drag orbits, right- or middle-drag pans, the scroll wheel zooms, and
//! `Home` or `0` resets to the default three-quarter view. State lives in the
//! [`OrbitCamera`] resource and is integrated into the camera transform each
//! frame.
//!
//! Touch input (drag = orbit, pinch = zoom) is deferred: Bevy exposes
//! `TouchInput` events, but a robust one/two-finger gesture recogniser is more
//! than this milestone needs. The mouse path below is the reference.

use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;

use crate::types::{MainCamera, OrbitCamera};

const ORBIT_SPEED: f32 = 0.005;
const PAN_SPEED: f32 = 0.0015;
const ZOOM_SPEED: f32 = 0.6;
const MIN_RADIUS: f32 = 3.0;
const MAX_RADIUS: f32 = 30.0;
const MAX_PITCH: f32 = 1.54; // just under 90 degrees

/// Spawn the 3D camera. Its transform is overwritten each frame by
/// [`orbit_camera`] from the [`OrbitCamera`] resource.
pub fn setup_camera(mut commands: Commands) {
    commands.spawn((
        Camera3d::default(),
        Transform::from_xyz(6.0, 6.0, 9.0).looking_at(Vec3::ZERO, Vec3::Y),
        MainCamera,
    ));
}

/// Build the world-space camera rotation from yaw/pitch.
fn orientation(orbit: &OrbitCamera) -> Quat {
    Quat::from_rotation_y(orbit.yaw) * Quat::from_rotation_x(-orbit.pitch)
}

/// Read mouse/keys, update [`OrbitCamera`], and write the camera transform.
pub fn orbit_camera(
    mut orbit: ResMut<OrbitCamera>,
    buttons: Res<ButtonInput<MouseButton>>,
    keys: Res<ButtonInput<KeyCode>>,
    mut motion: EventReader<MouseMotion>,
    mut wheel: EventReader<MouseWheel>,
    suppressed: Res<crate::play::OrbitSuppressed>,
    mut camera: Query<&mut Transform, With<MainCamera>>,
) {
    if keys.just_pressed(KeyCode::Home) || keys.just_pressed(KeyCode::Digit0) {
        *orbit = OrbitCamera::DEFAULT;
    }

    let mut drag = Vec2::ZERO;
    for ev in motion.read() {
        drag += ev.delta;
    }
    let mut scroll = 0.0;
    for ev in wheel.read() {
        scroll += ev.y;
    }

    // Left-drag orbits — unless a drag is turning a cube layer (play mode).
    if buttons.pressed(MouseButton::Left) && !suppressed.0 {
        orbit.yaw -= drag.x * ORBIT_SPEED;
        orbit.pitch = (orbit.pitch + drag.y * ORBIT_SPEED).clamp(-MAX_PITCH, MAX_PITCH);
    }

    if buttons.pressed(MouseButton::Right) || buttons.pressed(MouseButton::Middle) {
        let rot = orientation(&orbit);
        let right = rot * Vec3::X;
        let up = rot * Vec3::Y;
        let scale = orbit.radius * PAN_SPEED;
        orbit.focus += (-right * drag.x + up * drag.y) * scale;
    }

    if scroll.abs() > f32::EPSILON {
        orbit.radius = (orbit.radius - scroll * ZOOM_SPEED).clamp(MIN_RADIUS, MAX_RADIUS);
    }

    if let Ok(mut transform) = camera.single_mut() {
        let rot = orientation(&orbit);
        let position = orbit.focus + rot * Vec3::new(0.0, 0.0, orbit.radius);
        *transform = Transform::from_translation(position).looking_at(orbit.focus, Vec3::Y);
    }
}
