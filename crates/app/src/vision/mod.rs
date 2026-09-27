//! Camera cube input: the pure computer-vision core.
//!
//! This module turns camera frames into cube colors, with no camera or GPU
//! dependency, so it is unit-testable offline. Everything here operates on
//! in-memory [`image::RgbImage`] buffers; only the frame [`source`]s touch a
//! real camera.
//!
//! Pipeline: [`detect`] finds sticker cells → [`grid`] fits whole faces to them
//! → [`sample`] reads each face's nine colors ([`pipeline::read_face_grid`]
//! puts these together) → [`capture`] collects the six faces the user presents
//! → [`classify`] maps the 54 colors to faces relative to the six centers.

pub mod capture;
pub mod classify;
pub mod color;
pub mod detect;
#[cfg(test)]
pub mod fixtures;
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
