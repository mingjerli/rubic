//! Shared ECS components and data-only resources.
//!
//! Kept free of systems so every feature module (`cube_render`, `camera`,
//! `input`, `animation`, `solve`, `ui`) can depend on the same definitions
//! without cyclic coupling. Systems live in their respective modules.

use bevy::prelude::*;
use rubic_core::{Facelets, Move};
use std::collections::VecDeque;

/// The single source of truth for the cube: its 54 facelets. Rendering syncs
/// from this; animations and solves mutate it.
#[derive(Resource, Debug, Clone, Copy)]
pub struct CubeRes(pub Facelets);

/// Marker for the orbit camera entity.
#[derive(Component)]
pub struct MainCamera;

/// A cubie body. `cell` is its fixed home cell in `{-1,0,1}^3`; the renderer
/// never permutes cubies (it repaints stickers from [`CubeRes`] instead), so
/// the home transform is a pure translation.
#[derive(Component, Debug, Clone, Copy)]
pub struct Cubie {
    /// Fixed home cell in `{-1,0,1}^3`.
    pub cell: [i32; 3],
    /// Home translation in world space.
    pub home: Vec3,
}

/// A colored sticker quad, tagged with the facelet it displays.
#[derive(Component, Debug, Clone, Copy)]
pub struct Sticker {
    /// Facelet index into [`rubic_core::Facelets`] (`0..54`).
    pub facelet: usize,
}

/// Preallocated sticker materials so the sync system swaps handles instead of
/// mutating assets.
#[derive(Resource)]
pub struct StickerMaterials {
    /// One material per face color, indexed by [`rubic_core::Face::index`].
    pub by_face: [Handle<StandardMaterial>; 6],
    /// Neutral material for unknown stickers during color input.
    pub unknown: Handle<StandardMaterial>,
}

/// Orbit-camera state, integrated each frame into the camera transform.
#[derive(Resource, Debug, Clone, Copy)]
pub struct OrbitCamera {
    /// Point the camera looks at.
    pub focus: Vec3,
    /// Distance from the focus.
    pub radius: f32,
    /// Horizontal angle (radians).
    pub yaw: f32,
    /// Vertical angle (radians).
    pub pitch: f32,
}

impl OrbitCamera {
    /// The default three-quarter view.
    pub const DEFAULT: OrbitCamera = OrbitCamera {
        focus: Vec3::ZERO,
        radius: 9.0,
        yaw: std::f32::consts::FRAC_PI_4,
        pitch: 0.6,
    };
}

impl Default for OrbitCamera {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// An in-progress visual layer turn.
#[derive(Debug, Clone, Copy)]
pub struct ActiveTurn {
    /// The move being animated (applied to state on finish).
    pub mv: Move,
    /// Seconds elapsed so far.
    pub elapsed: f32,
    /// Total duration in seconds.
    pub duration: f32,
    /// Rotation axis (unit).
    pub axis: Vec3,
    /// Total signed angle to sweep (radians).
    pub total_angle: f32,
    /// Angle already applied to the layer's transforms (radians).
    pub applied: f32,
}

/// Queue of pending turns plus the one currently animating. Only one turn runs
/// at a time; input systems refuse to enqueue while busy, keeping the solve
/// player's cursor in lockstep with [`CubeRes`].
#[derive(Resource, Default)]
pub struct TurnQueue {
    /// Moves waiting to animate.
    pub pending: VecDeque<Move>,
    /// The currently animating turn, if any.
    pub active: Option<ActiveTurn>,
}

impl TurnQueue {
    /// Whether a new turn may start (nothing pending or animating).
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.active.is_none() && self.pending.is_empty()
    }

    /// Queue a move to animate.
    pub fn enqueue(&mut self, mv: Move) {
        self.pending.push_back(mv);
    }
}

/// Marker for the on-screen status text (validation + solver + step counter).
#[derive(Component)]
pub struct StatusText;

/// Marker for UI shown only on wide (desktop) windows — verbose
/// keyboard/orientation reference text that would clutter a phone screen where
/// touch controls guide the user instead.
#[derive(Component)]
pub struct DesktopOnly;
