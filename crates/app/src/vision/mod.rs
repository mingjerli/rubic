//! Camera cube input — pure computer-vision core (Phase A of spec 0002).
//!
//! This module turns camera frames into cube colors, with no camera or GPU
//! dependency so it is fully unit-testable offline. The platform-specific frame
//! *sources* (native webcam, browser `getUserMedia`) are added in later phases;
//! everything here operates on in-memory [`image::RgbImage`] buffers.
//!
//! Pipeline: [`detect`] finds a face in a frame → [`sample`] reads its 3×3 grid
//! of colors → [`classify`] maps colors to faces relative to the six centers →
//! [`pipeline`] accumulates six faces into a cube.
//!
//! Phase A delivers and tests this core ahead of the UI/camera wiring (Phase B),
//! so its functions are exercised by unit tests but not yet called by the
//! binary; the module-wide `dead_code` allow reflects that and is removed once
//! Phase B consumes it.
#![allow(dead_code)]

pub mod capture;
pub mod classify;
pub mod color;
pub mod detect;
pub mod grid;
pub mod pipeline;
pub mod sample;
pub mod source;

/// Native webcam frame source (desktop only). Compile-verified; live capture is
/// validated on hardware.
#[cfg(all(feature = "camera-native", not(target_arch = "wasm32")))]
pub mod native;

/// Browser webcam frame source (web only) via `getUserMedia`. Compile-verified;
/// validated on device.
#[cfg(all(feature = "camera-web", target_arch = "wasm32"))]
pub mod web_camera;

/// An RGB color sample, `[r, g, b]` each `0..=255`.
pub type Rgb = [u8; 3];
