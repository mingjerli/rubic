//! Guided capture-flow state machine.
//!
//! Pure logic (no camera, no Bevy): the app guides the user to present the six
//! faces in [`CAPTURE_ORDER`]; each frame's detected samples are fed in, and a
//! face is auto-captured once its reading is *stable* across
//! [`STABILITY_FRAMES`] consecutive frames. Each captured face is routed to its
//! URFDLB slot, so the final [`Scan`] classifies correctly regardless of the
//! order faces are shown. A manual capture is also available.
//!
//! Orientation (holding a face the right way up) is guided by on-screen
//! instructions and fixed in the review step; the CV does not infer it.

use super::Rgb;
use super::classify::Classified;
use super::pipeline::Scan;
use rubic_core::Face;

/// The order the user is guided to present faces.
pub const CAPTURE_ORDER: [Face; 6] = Face::ALL;

/// Consecutive stable frames required to auto-capture a face.
pub const STABILITY_FRAMES: usize = 4;

/// Max per-channel difference for two readings to count as "the same".
pub const STABILITY_TOLERANCE: u8 = 14;

/// What happened when a frame was processed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CaptureEvent {
    /// No face detected this frame.
    Idle,
    /// A face is being tracked; `stable_frames` in a row so far.
    Tracking(usize),
    /// A face was just captured (auto or manual).
    Captured(Face),
    /// The sixth face was just captured; the scan is complete.
    Completed,
}

/// Drives capture of the six faces.
#[derive(Debug, Clone, Default)]
pub struct CaptureFlow {
    step: usize,
    scan: Scan,
    recent: Vec<[Rgb; 9]>,
}

impl CaptureFlow {
    /// A fresh flow targeting the first face.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// The face the user should present next, or `None` when complete.
    #[must_use]
    pub fn current_target(&self) -> Option<Face> {
        CAPTURE_ORDER.get(self.step).copied()
    }

    /// How many faces have been captured.
    #[must_use]
    pub fn captured_count(&self) -> usize {
        self.step
    }

    /// Whether all six faces are captured.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.step >= 6
    }

    /// The classified cube once complete, else `None`.
    #[must_use]
    pub fn finish(&self) -> Option<Classified> {
        self.scan.classify()
    }

    /// Discard all progress and start over.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Process one frame's detected samples (`None` if no face was found).
    pub fn on_frame(&mut self, detected: Option<[Rgb; 9]>) -> CaptureEvent {
        if self.is_complete() {
            return CaptureEvent::Completed;
        }
        let Some(samples) = detected else {
            self.recent.clear();
            return CaptureEvent::Idle;
        };

        // Extend the stability run only while readings stay close.
        if let Some(last) = self.recent.last() {
            if !close(*last, samples) {
                self.recent.clear();
            }
        }
        self.recent.push(samples);

        if self.recent.len() >= STABILITY_FRAMES {
            self.commit(samples)
        } else {
            CaptureEvent::Tracking(self.recent.len())
        }
    }

    /// Manually capture the current target from the given samples.
    pub fn force_capture(&mut self, samples: [Rgb; 9]) -> CaptureEvent {
        if self.is_complete() {
            return CaptureEvent::Completed;
        }
        self.commit(samples)
    }

    /// Route `samples` to the current target's slot and advance.
    fn commit(&mut self, samples: [Rgb; 9]) -> CaptureEvent {
        let face = CAPTURE_ORDER[self.step];
        self.scan.set_face(face.index(), samples);
        self.step += 1;
        self.recent.clear();
        if self.is_complete() {
            CaptureEvent::Completed
        } else {
            CaptureEvent::Captured(face)
        }
    }
}

/// Whether two readings agree within [`STABILITY_TOLERANCE`] on every channel.
fn close(a: [Rgb; 9], b: [Rgb; 9]) -> bool {
    a.iter().zip(b.iter()).all(|(pa, pb)| {
        pa.iter()
            .zip(pb.iter())
            .all(|(&x, &y)| x.abs_diff(y) <= STABILITY_TOLERANCE)
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::sticker_rgb;
    use crate::vision::pipeline::capture_from_frame;
    use image::RgbImage;
    use rubic_core::{Facelets, Sequence};

    fn face_rgb(f: Face) -> Rgb {
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

    /// Canonical-order samples for face slot `f` of `cube`.
    fn samples_for(cube: &Facelets, f: usize) -> [Rgb; 9] {
        capture_from_frame(&render_face_frame(cube, f)).unwrap()
    }

    #[test]
    fn nothing_detected_never_captures() {
        let mut flow = CaptureFlow::new();
        for _ in 0..10 {
            assert_eq!(flow.on_frame(None), CaptureEvent::Idle);
        }
        assert_eq!(flow.captured_count(), 0);
    }

    #[test]
    fn stable_face_captures_after_threshold() {
        let s = samples_for(&Facelets::SOLVED, 0);
        let mut flow = CaptureFlow::new();
        for i in 1..STABILITY_FRAMES {
            assert_eq!(flow.on_frame(Some(s)), CaptureEvent::Tracking(i));
        }
        assert_eq!(
            flow.on_frame(Some(s)),
            CaptureEvent::Captured(CAPTURE_ORDER[0])
        );
        assert_eq!(flow.captured_count(), 1);
        assert_eq!(flow.current_target(), Some(CAPTURE_ORDER[1]));
    }

    #[test]
    fn jitter_resets_stability() {
        let mut flow = CaptureFlow::new();
        // Alternating very different readings never stabilize.
        for i in 0..STABILITY_FRAMES * 2 {
            let s = if i % 2 == 0 {
                [[240, 240, 240]; 9]
            } else {
                [[20, 20, 200]; 9]
            };
            flow.on_frame(Some(s));
        }
        assert_eq!(flow.captured_count(), 0);
    }

    #[test]
    fn force_capture_locks_immediately() {
        let s = samples_for(&Facelets::SOLVED, 0);
        let mut flow = CaptureFlow::new();
        assert_eq!(
            flow.force_capture(s),
            CaptureEvent::Captured(CAPTURE_ORDER[0])
        );
        assert_eq!(flow.captured_count(), 1);
    }

    #[test]
    fn full_guided_scan_completes_and_classifies() {
        let cube = Facelets::SOLVED.apply_seq(&"R U R' U' F2 L D B'".parse::<Sequence>().unwrap());
        let mut flow = CaptureFlow::new();
        for f in 0..6 {
            let s = samples_for(&cube, f);
            let mut event = CaptureEvent::Idle;
            for _ in 0..STABILITY_FRAMES {
                event = flow.on_frame(Some(s));
            }
            if f < 5 {
                assert_eq!(event, CaptureEvent::Captured(CAPTURE_ORDER[f]));
            } else {
                assert_eq!(event, CaptureEvent::Completed);
            }
        }
        assert!(flow.is_complete());
        assert_eq!(flow.finish().unwrap().facelets, cube);
    }
}
