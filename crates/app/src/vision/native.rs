//! Native webcam [`CameraSource`] via the `nokhwa` crate (desktop only).
//!
//! **Compile-verified only.** Live capture needs a real camera and is validated
//! on-device; this environment has no camera, so behavior here is not exercised
//! by tests. Frames are converted through raw RGB bytes so this module does not
//! couple to `nokhwa`'s internal `image` version.

use super::source::CameraSource;
use image::RgbImage;
use nokhwa::Camera;
use nokhwa::pixel_format::RgbFormat;
use nokhwa::utils::{CameraIndex, RequestedFormat, RequestedFormatType};

/// A webcam opened through `nokhwa`.
pub struct NativeCamera {
    camera: Camera,
}

impl NativeCamera {
    /// Open the default (first) camera and start streaming.
    ///
    /// # Errors
    /// Returns a message if no camera is available or the stream cannot start.
    pub fn open_default() -> Result<Self, String> {
        let index = CameraIndex::Index(0);
        let format =
            RequestedFormat::new::<RgbFormat>(RequestedFormatType::AbsoluteHighestFrameRate);
        let mut camera = Camera::new(index, format).map_err(|e| e.to_string())?;
        camera.open_stream().map_err(|e| e.to_string())?;
        Ok(Self { camera })
    }
}

impl CameraSource for NativeCamera {
    fn next_frame(&mut self) -> Option<RgbImage> {
        let frame = self.camera.frame().ok()?;
        let decoded = frame.decode_image::<RgbFormat>().ok()?;
        let (w, h) = (decoded.width(), decoded.height());
        // `into_raw()` yields a plain `Vec<u8>`, decoupling from nokhwa's own
        // `image` version; rebuild it as our `image` crate's `RgbImage`.
        RgbImage::from_raw(w, h, decoded.into_raw())
    }
}
