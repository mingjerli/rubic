//! A Scan in progress: which face comes next, the faces captured so far, the
//! live-filled Net, and the latest face reading from the camera.
//!
//! Pure: the camera system hands each detection to [`Scan::observe`], and a
//! Face capture commits exactly that reading, so what the HUD called "in view"
//! is what gets captured. With nothing in view, capture does nothing. It also
//! reports each frame to [`Scan::tick`], so the Scan knows whether the camera
//! is delivering images ([`CameraStatus`]). [`hud`] turns it all into words.

use rubic_core::{Face, PartialFacelets};

use crate::colors::sticker_rgb;
use crate::vision::Rgb;
use crate::vision::capture::CaptureFlow;
use crate::vision::color::{perceptual_point, point_distance_sq};

pub mod hud;
#[cfg(test)]
pub mod tests;

/// How long without a camera image before the Scan reports the camera as
/// unavailable (long enough for a browser permission prompt to appear).
const NO_IMAGE_GRACE_SECS: f32 = 3.0;

/// Whether the camera is delivering images.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraStatus {
    /// Opened, no image yet.
    Starting,
    Live,
    /// No image for a while: permission denied or pending, or the camera
    /// stopped.
    Unavailable,
}

/// When images arrived, in seconds on the app clock.
#[derive(Clone, Copy, Debug, Default)]
struct CameraClock {
    started: Option<f32>,
    last_image: Option<f32>,
    now: f32,
}

#[derive(Clone, Debug)]
pub struct Scan {
    capture: CaptureFlow,
    /// The Net as filled so far, approximating each captured sticker's color.
    live: PartialFacelets,
    /// The latest face reading, if a face is in view.
    in_view: Option<[Rgb; 9]>,
    camera: CameraClock,
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
            camera: CameraClock::default(),
        }
    }

    /// Report one camera poll at `now` (seconds), and whether it brought an
    /// image.
    pub fn tick(&mut self, now: f32, image: bool) {
        let clock = &mut self.camera;
        clock.now = now;
        clock.started.get_or_insert(now);
        if image {
            clock.last_image = Some(now);
        }
    }

    /// Whether the camera is delivering images, as of the latest tick.
    #[must_use]
    pub fn camera_status(&self) -> CameraStatus {
        let clock = &self.camera;
        let since = clock.last_image.or(clock.started).unwrap_or(clock.now);
        if clock.now - since > NO_IMAGE_GRACE_SECS {
            CameraStatus::Unavailable
        } else if clock.last_image.is_some() {
            CameraStatus::Live
        } else {
            CameraStatus::Starting
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
        let completed = self.capture.advance();
        // The next face needs a fresh reading, not the previous face's.
        self.in_view = None;
        if !completed {
            return None;
        }
        Some(match self.capture.finish() {
            Some(cube) => PartialFacelets::from_facelets(&cube),
            None => self.live.clone(),
        })
    }

    /// Go back a face to retake it (its capture is kept until overwritten).
    pub fn prev_face(&mut self) {
        self.capture.step_back();
        self.in_view = None;
    }

    /// Discard every captured face and start again from the first (the
    /// camera keeps running).
    pub fn restart(&mut self) {
        *self = Self {
            camera: self.camera,
            ..Self::new()
        };
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
