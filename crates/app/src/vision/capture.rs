//! Guided capture of the six faces.
//!
//! Pure logic (no camera, no Bevy): the user presents the faces in
//! [`CAPTURE_ORDER`], capturing (and retaking) each one before moving on. Each
//! captured face is routed to its URFDLB slot, so the finished samples classify
//! correctly regardless of the order faces are shown.
//!
//! Orientation (holding a face the right way up) is guided by on-screen
//! instructions and fixed in the review step; the CV does not infer it.

use super::Rgb;
use super::pipeline::FaceSamples;
use rubic_core::{Face, Facelets};

/// The order the user is guided to present faces.
pub const CAPTURE_ORDER: [Face; 6] = Face::ALL;

/// Drives capture of the six faces.
#[derive(Debug, Clone, Default)]
pub struct CaptureFlow {
    step: usize,
    samples: FaceSamples,
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

    /// Index (0-based) of the face currently being presented.
    #[must_use]
    pub fn current_index(&self) -> usize {
        self.step
    }

    /// Whether the face currently being presented has already been captured
    /// (so the HUD can offer "retake or next").
    #[must_use]
    pub fn current_captured(&self) -> bool {
        self.current_target()
            .is_some_and(|f| self.samples.has_face(f.index()))
    }

    /// Whether all six faces are captured.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.step >= CAPTURE_ORDER.len()
    }

    /// The classified cube once complete, else `None`.
    #[must_use]
    pub fn finish(&self) -> Option<Facelets> {
        self.samples.classify()
    }

    /// Record the current face's samples **without advancing**, so the same
    /// side can be retaken until it looks right. Overwrites any prior capture.
    pub fn capture(&mut self, samples: [Rgb; 9]) {
        if let Some(face) = self.current_target() {
            self.samples.set_face(face.index(), samples);
        }
    }

    /// Move on to the next face, but only once the current one is captured.
    /// Returns whether all six faces are now done.
    pub fn advance(&mut self) -> bool {
        if self.current_captured() {
            self.step += 1;
        }
        self.is_complete()
    }

    /// Step back to the previous face to retake it (its capture is kept until
    /// overwritten).
    pub fn step_back(&mut self) {
        self.step = self.step.saturating_sub(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::fixtures::face_rgb;
    use rubic_core::Sequence;

    /// Face `face` of `cube`, as ideal camera colors.
    fn samples_for(cube: &Facelets, face: Face) -> [Rgb; 9] {
        std::array::from_fn(|k| face_rgb(cube.get(face.index() * 9 + k)))
    }

    #[test]
    fn advance_waits_for_a_capture() {
        let mut flow = CaptureFlow::new();
        assert!(!flow.advance());
        assert_eq!(flow.current_index(), 0);
    }

    #[test]
    fn capture_retakes_without_advancing() {
        let mut flow = CaptureFlow::new();
        flow.capture([[0, 0, 0]; 9]);
        flow.capture(samples_for(&Facelets::SOLVED, CAPTURE_ORDER[0]));
        assert!(flow.current_captured());
        assert_eq!(flow.current_index(), 0);
    }

    #[test]
    fn step_back_keeps_the_earlier_capture() {
        let mut flow = CaptureFlow::new();
        flow.capture(samples_for(&Facelets::SOLVED, CAPTURE_ORDER[0]));
        flow.advance();
        flow.step_back();
        assert_eq!(flow.current_index(), 0);
        assert!(flow.current_captured());
        flow.step_back();
        assert_eq!(
            flow.current_index(),
            0,
            "can't step back past the first face"
        );
    }

    #[test]
    fn six_captures_complete_and_classify() {
        let cube = Facelets::SOLVED.apply_seq(&"R U R' U' F2 L D B'".parse::<Sequence>().unwrap());
        let mut flow = CaptureFlow::new();
        for (i, &face) in CAPTURE_ORDER.iter().enumerate() {
            assert!(flow.finish().is_none(), "incomplete scans don't classify");
            flow.capture(samples_for(&cube, face));
            assert_eq!(flow.advance(), i + 1 == CAPTURE_ORDER.len());
        }
        assert_eq!(flow.current_target(), None);
        assert_eq!(flow.finish(), Some(cube));
    }
}
