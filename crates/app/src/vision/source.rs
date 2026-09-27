//! Camera frame source abstraction.
//!
//! The rest of the vision pipeline is platform-independent; only *where frames
//! come from* differs. [`CameraSource`] is that seam: the native webcam source
//! (`nokhwa`) and the web source (`getUserMedia`) implement it, and the Bevy
//! camera wiring drives whichever it was given. In tests, `ReplaySource` plays
//! back a fixed list of frames.

use image::RgbImage;

/// A source of camera frames as RGB images.
pub trait CameraSource {
    /// The most recent frame, or `None` if none is available this tick.
    fn next_frame(&mut self) -> Option<RgbImage>;
}

/// A [`CameraSource`] that replays a fixed queue of frames, then yields `None`.
#[cfg(test)]
pub struct ReplaySource {
    frames: std::collections::VecDeque<RgbImage>,
}

#[cfg(test)]
impl ReplaySource {
    /// Build a replay source from frames (yielded front-to-back).
    #[must_use]
    pub fn new(frames: Vec<RgbImage>) -> Self {
        Self {
            frames: frames.into(),
        }
    }
}

#[cfg(test)]
impl CameraSource for ReplaySource {
    fn next_frame(&mut self) -> Option<RgbImage> {
        self.frames.pop_front()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::fixtures::face_frame;
    use rubic_core::{Face, Facelets};

    #[test]
    fn replay_source_yields_frames_then_none() {
        let f = face_frame(&Facelets::SOLVED, Face::U);
        let mut src = ReplaySource::new(vec![f.clone(), f]);
        assert!(src.next_frame().is_some());
        assert!(src.next_frame().is_some());
        assert!(src.next_frame().is_none());
    }
}
