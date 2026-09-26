//! A Scan in progress: which face comes next, the faces captured so far, the
//! live-filled Net, and the latest face reading from the camera.
//!
//! Pure: the camera system hands each detection to [`Scan::observe`], and a
//! Face capture commits exactly that reading, so what the HUD called "in view"
//! is what gets captured. With nothing in view, capture does nothing.

use rubic_core::{Face, PartialFacelets};

use crate::colors::sticker_rgb;
use crate::vision::Rgb;
use crate::vision::capture::{CaptureEvent, CaptureFlow};
use crate::vision::color::{perceptual_point, point_distance_sq};

#[derive(Clone, Debug)]
pub struct Scan {
    capture: CaptureFlow,
    /// The Net as filled so far, approximating each captured sticker's color.
    live: PartialFacelets,
    /// The latest face reading, if a face is in view.
    in_view: Option<[Rgb; 9]>,
}

impl Default for Scan {
    fn default() -> Self {
        Self::new()
    }
}

impl Scan {
    /// A fresh Scan on the first face, with only the centers on the Net.
    #[must_use]
    pub fn new() -> Self {
        Self {
            capture: CaptureFlow::new(),
            live: PartialFacelets::new(),
            in_view: None,
        }
    }

    /// Record the latest face reading (`None` when no face is detected).
    pub fn observe(&mut self, reading: Option<[Rgb; 9]>) {
        self.in_view = reading;
    }

    /// Whether a face is in view and ready to capture.
    #[must_use]
    pub fn in_view(&self) -> bool {
        self.in_view.is_some()
    }

    /// Capture (or retake) the current face from the reading in view, filling
    /// it onto the live Net. Returns whether anything was captured.
    pub fn capture(&mut self) -> bool {
        let (Some(samples), Some(face)) = (self.in_view, self.capture.current_target()) else {
            return false;
        };
        self.capture.capture(samples);
        for (k, &sample) in samples.iter().enumerate() {
            self.live = self
                .live
                .set(face.index() * 9 + k, nearest_scheme_face(sample));
        }
        true
    }

    /// Move on to the next face once the current one is captured. After the
    /// sixth face, returns the scanned cube for Editing.
    pub fn next_face(&mut self) -> Option<PartialFacelets> {
        let completed = self.capture.advance() == CaptureEvent::Completed;
        // The next face needs a fresh reading, not the previous face's.
        self.in_view = None;
        if !completed {
            return None;
        }
        Some(match self.capture.finish() {
            Some(classified) => PartialFacelets::from_facelets(&classified.facelets),
            None => self.live.clone(),
        })
    }

    /// Go back a face to retake it (its capture is kept until overwritten).
    pub fn prev_face(&mut self) {
        self.capture.step_back();
        self.in_view = None;
    }

    /// Discard every captured face and start again from the first.
    pub fn restart(&mut self) {
        *self = Self::new();
    }

    /// The Net as filled so far.
    #[must_use]
    pub fn live(&self) -> &PartialFacelets {
        &self.live
    }

    /// The face to present next, or `None` once all six are captured.
    #[must_use]
    pub fn target(&self) -> Option<Face> {
        self.capture.current_target()
    }

    /// Index (0-based) of the face being presented.
    #[must_use]
    pub fn index(&self) -> usize {
        self.capture.current_index()
    }

    /// Whether the face being presented has already been captured.
    #[must_use]
    pub fn current_captured(&self) -> bool {
        self.capture.current_captured()
    }
}

/// Perceptual point of a face's ideal scheme color.
fn scheme_point(face: Face) -> [f32; 3] {
    let c = sticker_rgb(face);
    perceptual_point([
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
    ])
}

/// Nearest scheme face color to a sampled sticker, for the live Net. (The final
/// [`crate::vision::classify`] pass is relative/cluster-based; this is a quick
/// per-face approximation for instant feedback.)
fn nearest_scheme_face(sample: Rgb) -> Face {
    let p = perceptual_point(sample);
    Face::ALL
        .into_iter()
        .min_by(|&a, &b| {
            point_distance_sq(p, scheme_point(a))
                .partial_cmp(&point_distance_sq(p, scheme_point(b)))
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(Face::U)
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::vision::capture::CAPTURE_ORDER;
    use rubic_core::{Completion, Facelets};

    /// A face's nine stickers in its ideal scheme color.
    pub fn solid(face: Face) -> [Rgb; 9] {
        let c = sticker_rgb(face);
        [[
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
        ]; 9]
    }

    /// The reading of face `face` of a solved cube, as the camera would see it.
    pub fn solved_reading(face: Face) -> [Rgb; 9] {
        std::array::from_fn(|k| {
            let c = sticker_rgb(Facelets::SOLVED.get(face.index() * 9 + k));
            [
                (c[0] * 255.0) as u8,
                (c[1] * 255.0) as u8,
                (c[2] * 255.0) as u8,
            ]
        })
    }

    #[test]
    fn capture_fills_the_live_net_with_the_face_in_view() {
        let mut scan = Scan::new();
        let face = scan.target().unwrap();
        scan.observe(Some(solid(face)));
        assert!(scan.capture());
        assert!(scan.current_captured());
        for k in 0..9 {
            assert_eq!(scan.live().get(face.index() * 9 + k), Some(face));
        }
    }

    #[test]
    fn capture_with_nothing_in_view_captures_nothing() {
        let mut scan = Scan::new();
        scan.observe(None);
        assert!(!scan.capture());
        assert!(!scan.current_captured());
    }

    #[test]
    fn capture_commits_the_latest_reading() {
        let mut scan = Scan::new();
        let face = scan.target().unwrap();
        let other = Face::ALL.into_iter().find(|&f| f != face).unwrap();
        scan.observe(Some(solid(other)));
        scan.observe(Some(solid(face)));
        scan.capture();
        assert_eq!(scan.live().get(face.index() * 9), Some(face));
    }

    #[test]
    fn moving_faces_needs_a_fresh_reading() {
        let mut scan = Scan::new();
        scan.observe(Some(solid(scan.target().unwrap())));
        scan.capture();
        scan.next_face();
        assert!(!scan.in_view());
        assert!(
            !scan.capture(),
            "the previous face's reading must not be reused"
        );
    }

    #[test]
    fn next_face_waits_for_a_capture() {
        let mut scan = Scan::new();
        assert!(scan.next_face().is_none());
        assert_eq!(scan.index(), 0);
    }

    #[test]
    fn six_captures_hand_off_the_scanned_cube() {
        let mut scan = Scan::new();
        let mut handoff = None;
        for face in CAPTURE_ORDER {
            scan.observe(Some(solved_reading(face)));
            assert!(scan.capture());
            handoff = scan.next_face();
        }
        let partial = handoff.expect("the sixth Next hands off");
        assert!(matches!(partial.analyze(), Completion::Unique(_)));
    }

    #[test]
    fn prev_face_goes_back_and_restart_clears_everything() {
        let mut scan = Scan::new();
        scan.observe(Some(solid(scan.target().unwrap())));
        scan.capture();
        scan.next_face();
        scan.prev_face();
        assert_eq!(scan.index(), 0);
        assert!(scan.current_captured());
        scan.restart();
        assert_eq!(scan.index(), 0);
        assert!(!scan.current_captured());
        assert_eq!(scan.live().known_count(), 0);
    }
}
