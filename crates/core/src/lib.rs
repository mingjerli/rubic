//! # rubic-core
//!
//! Pure-Rust core for a 3x3 Rubik's Cube: state model, move engine, input
//! validation, and solvers (beginner layer-by-layer + optimal).
//!
//! This crate has no rendering dependencies so it can be unit-tested in
//! isolation and reused by the Bevy application layer and CLI.

pub mod color;
pub mod complete;
pub mod engine;
pub mod facelets;
pub mod moves;
pub mod state;

pub use color::Face;
pub use complete::{Completion, PartialFacelets};
pub use facelets::{Facelets, ParseFaceletsError};
pub use moves::{Amount, Move, ParseMoveError, Sequence};
pub use state::{CubeError, CubeState};
