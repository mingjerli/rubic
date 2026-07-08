//! Camera frame source abstraction.
//!
//! The rest of the vision pipeline is platform-independent; only *where frames
//! come from* differs. [`CameraSource`] is that seam: the native webcam source
//! (`nokhwa`) and the web source (`getUserMedia`) implement it, and the Bevy
//! camera mode drives whichever it was given. [`ReplaySource`] is a source
//! backed by a fixed list of frames, used for headless tests and demos.

use image::RgbImage;
use std::collections::VecDeque;

/// A source of camera frames as RGB images.
pub trait CameraSource {
    /// The most recent frame, or `None` if none is available this tick.
    fn next_frame(&mut self) -> Option<RgbImage>;
}

/// A [`CameraSource`] that replays a fixed queue of frames, then yields `None`.
pub struct ReplaySource {
    frames: VecDeque<RgbImage>,
}

impl ReplaySource {
    /// Build a replay source from frames (yielded front-to-back).
    #[must_use]
    pub fn new(frames: Vec<RgbImage>) -> Self {
        Self {
            frames: frames.into(),
        }
    }
}

impl CameraSource for ReplaySource {
    fn next_frame(&mut self) -> Option<RgbImage> {
        self.frames.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::sticker_rgb;
    use crate::vision::capture::{CaptureFlow, STABILITY_FRAMES};
    use crate::vision::pipeline::capture_from_frame;
    use rubic_core::{Face, Facelets, Sequence};

    fn face_rgb(f: Face) -> [u8; 3] {
        let c = sticker_rgb(f);
        [
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
        ]
    }

    fn render_face_frame(cube: &Facelets, f: usize) -> RgbImage {
        let fsize = 90u32;
        let (ox, oy) = (20u32, 15u32);
        let cell = fsize / 3;
        RgbImage::from_fn(fsize + 2 * ox, fsize + 2 * oy, |x, y| {
            if x >= ox && x < ox + fsize && y >= oy && y < oy + fsize {
                let cx = ((x - ox) / cell).min(2);
                let cy = ((y - oy) / cell).min(2);
                image::Rgb(face_rgb(cube.get(f * 9 + (cy * 3 + cx) as usize)))
            } else {
                image::Rgb([18, 18, 20])
            }
        })
    }

    #[test]
    fn replay_source_yields_frames_then_none() {
        let f = render_face_frame(&Facelets::SOLVED, 0);
        let mut src = ReplaySource::new(vec![f.clone(), f]);
        assert!(src.next_frame().is_some());
        assert!(src.next_frame().is_some());
        assert!(src.next_frame().is_none());
    }

    #[test]
    fn driving_capture_from_a_source_recovers_the_cube() {
        let cube = Facelets::SOLVED.apply_seq(&"R U R' U' F2 L D B'".parse::<Sequence>().unwrap());
        // Each face held steady for the stability window.
        let mut frames = Vec::new();
        for f in 0..6 {
            for _ in 0..STABILITY_FRAMES {
                frames.push(render_face_frame(&cube, f));
            }
        }
        let mut src = ReplaySource::new(frames);
        let mut flow = CaptureFlow::new();
        while let Some(frame) = src.next_frame() {
            flow.on_frame(capture_from_frame(&frame));
        }
        assert!(flow.is_complete());
        assert_eq!(flow.finish().unwrap().facelets, cube);
    }
}
