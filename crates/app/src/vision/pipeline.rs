//! Accumulate captured faces into a cube.
//!
//! A [`Scan`] collects up to six faces of nine samples each (in URFDLB-local
//! row-major order) and, once complete, classifies them into a
//! [`Classified`](super::classify::Classified) cube. Arranging each physical
//! face into the correct facelet slot and rotation is the capture flow's job
//! (Phase B); this module assumes samples already arrive in facelet order.

use super::Rgb;
use super::classify::{Classified, classify as classify_samples};
use super::detect::{detect_face, detect_stickers};
use super::grid::fit_faces;
use super::sample::{sample_centers, sample_face};
use image::RgbImage;

/// A scan in progress: up to six captured faces, each nine samples. Face slot
/// `f` corresponds to [`rubic_core::Face`] index `f`.
#[derive(Debug, Clone, Default)]
pub struct Scan {
    faces: [Option<[Rgb; 9]>; 6],
}

impl Scan {
    /// A fresh scan with no captured faces.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Record the nine samples for face slot `f` (`0..6`).
    pub fn set_face(&mut self, f: usize, samples: [Rgb; 9]) {
        self.faces[f] = Some(samples);
    }

    /// How many of the six faces have been captured.
    #[must_use]
    pub fn captured_count(&self) -> usize {
        self.faces.iter().filter(|f| f.is_some()).count()
    }

    /// Whether all six faces are captured.
    #[must_use]
    pub fn is_complete(&self) -> bool {
        self.captured_count() == 6
    }

    /// Classify the full scan into a cube, or `None` until all six faces are in.
    #[must_use]
    pub fn classify(&self) -> Option<Classified> {
        let mut samples = [[0u8; 3]; 54];
        for (f, slot) in self.faces.iter().enumerate() {
            let face = (*slot)?;
            samples[f * 9..f * 9 + 9].copy_from_slice(&face);
        }
        Some(classify_samples(&samples))
    }
}

/// Detect and sample a face from a single frame, or `None` if none is found.
#[must_use]
pub fn capture_from_frame(frame: &RgbImage) -> Option<[Rgb; 9]> {
    detect_face(frame).map(|face| sample_face(&face))
}

/// Cell pitch of a fitted face (distance between adjacent predicted centers).
fn face_pitch(face: &[(f32, f32); 9]) -> f32 {
    (face[1].0 - face[0].0).hypot(face[1].1 - face[0].1)
}

/// Read a face's nine colors via robust grid-fitting: detect sticker cells, fit
/// face grids, take the most frontal one (largest cell pitch = closest / least
/// foreshortened), and sample its nine predicted cell centers. `None` if no
/// face grid is found.
///
/// This drives guided frontal capture: the user shows one face square-on, and
/// even with only a few cells cleanly detected the grid recovers all nine.
#[must_use]
pub fn read_face_grid(frame: &RgbImage) -> Option<[Rgb; 9]> {
    let stickers = detect_stickers(frame);
    let face = fit_faces(&stickers).into_iter().max_by(|a, b| {
        face_pitch(a)
            .partial_cmp(&face_pitch(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;
    let radius = (face_pitch(&face).max(8.0)) * 0.18;
    Some(sample_centers(frame, &face, radius))
}

/// Fraction of the frame's shorter side used by the centered alignment box.
pub const GUIDE_FRACTION: u32 = 3; // 3/5 of the shorter side
const GUIDE_DIVISOR: u32 = 5;

/// The centered square region (`x, y, side`) the guided alignment box samples.
#[must_use]
pub fn guide_region(w: u32, h: u32) -> (u32, u32, u32) {
    let side = w.min(h) * GUIDE_FRACTION / GUIDE_DIVISOR;
    ((w - side) / 2, (h - side) / 2, side)
}

/// Sample the nine colors of the centered alignment box (guided capture).
///
/// Unlike [`capture_from_frame`], this never fails: the user aligns the cube
/// face to fill the on-screen box, and this reads the fixed region — no fragile
/// face detection.
#[must_use]
pub fn capture_centered(frame: &RgbImage) -> [Rgb; 9] {
    let (w, h) = frame.dimensions();
    let (x, y, side) = guide_region(w, h);
    let square = image::imageops::crop_imm(frame, x, y, side, side).to_image();
    sample_face(&square)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::sticker_rgb;
    use rubic_core::{Face, Facelets, Sequence};

    fn face_rgb(f: Face) -> Rgb {
        let c = sticker_rgb(f);
        [
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
        ]
    }

    /// Render face slot `f` of `cube` as a camera frame: its nine stickers as a
    /// 3×3 grid on a plain background, so detect+sample can recover them.
    fn render_face_frame(cube: &Facelets, f: usize) -> RgbImage {
        let fsize = 90u32;
        let (ox, oy) = (20u32, 15u32);
        let (w, h) = (fsize + 2 * ox, fsize + 2 * oy);
        let cell = fsize / 3;
        RgbImage::from_fn(w, h, |x, y| {
            if x >= ox && x < ox + fsize && y >= oy && y < oy + fsize {
                let cx = ((x - ox) / cell).min(2);
                let cy = ((y - oy) / cell).min(2);
                let facelet = f * 9 + (cy * 3 + cx) as usize;
                image::Rgb(face_rgb(cube.get(facelet)))
            } else {
                image::Rgb([18, 18, 20])
            }
        })
    }

    fn scramble(s: &str) -> Facelets {
        Facelets::SOLVED.apply_seq(&s.parse::<Sequence>().unwrap())
    }

    #[test]
    fn full_scan_recovers_and_validates_the_cube() {
        let cube = scramble("R U R' U' F2 L D B' R2 U");
        let mut scan = Scan::new();
        for f in 0..6 {
            let frame = render_face_frame(&cube, f);
            let samples = capture_from_frame(&frame).expect("face detected");
            scan.set_face(f, samples);
        }
        let classified = scan.classify().expect("complete scan classifies");
        assert_eq!(classified.facelets, cube, "recovered cube mismatch");
        // The recovered cube is a real, solvable cube.
        assert!(classified.facelets.validate().is_ok());
    }

    #[test]
    fn capture_centered_reads_the_aligned_box() {
        // Render a face exactly into the guide region; the rest is background.
        let (w, h) = (200u32, 160u32);
        let (gx, gy, side) = guide_region(w, h);
        let cell = side / 3;
        let cube = scramble("R U F2 L' D B");
        let colors: [Rgb; 9] = std::array::from_fn(|i| face_rgb(cube.get(i)));
        let frame = RgbImage::from_fn(w, h, |x, y| {
            if x >= gx && x < gx + side && y >= gy && y < gy + side {
                let cx = ((x - gx) / cell).min(2);
                let cy = ((y - gy) / cell).min(2);
                image::Rgb(colors[(cy * 3 + cx) as usize])
            } else {
                image::Rgb([18, 18, 20])
            }
        });
        let got = capture_centered(&frame);
        for (g, want) in got.iter().zip(colors.iter()) {
            for k in 0..3 {
                assert!(
                    i32::from(g[k]).abs_diff(i32::from(want[k])) <= 8,
                    "cell drift: {g:?} vs {want:?}"
                );
            }
        }
    }

    /// Render a face with black lattice borders (like a real cube) on a plain
    /// background, so the edge-based detector engages.
    fn render_bordered_face(colors: [Rgb; 9]) -> RgbImage {
        let (cell, border, margin) = (70u32, 14u32, 120u32);
        let face = 3 * cell + 4 * border;
        let (w, h) = (face + 2 * margin, face + 2 * margin);
        RgbImage::from_fn(w, h, |x, y| {
            if x < margin || y < margin || x >= margin + face || y >= margin + face {
                return image::Rgb([210, 210, 210]); // plain background
            }
            let (lx, ly) = (x - margin, y - margin);
            let step = cell + border;
            let (ix, iy) = (lx % step, ly % step);
            if ix < border || iy < border {
                return image::Rgb([10, 10, 10]); // black lattice
            }
            let (cx, cy) = ((lx / step).min(2), (ly / step).min(2));
            image::Rgb(colors[(cy * 3 + cx) as usize])
        })
    }

    #[test]
    fn read_face_grid_recovers_bordered_face() {
        let colors: [Rgb; 9] = [
            [220, 30, 30],
            [30, 180, 60],
            [40, 60, 220],
            [240, 140, 20],
            [235, 230, 40],
            [240, 240, 240],
            [150, 30, 220],
            [30, 200, 200],
            [220, 40, 200],
        ];
        let frame = render_bordered_face(colors);
        let got = read_face_grid(&frame).expect("face grid read");
        for (g, want) in got.iter().zip(colors.iter()) {
            for k in 0..3 {
                assert!(
                    i32::from(g[k]).abs_diff(i32::from(want[k])) <= 20,
                    "cell drift: {g:?} vs {want:?}"
                );
            }
        }
    }

    #[test]
    fn incomplete_scan_does_not_classify() {
        let cube = Facelets::SOLVED;
        let mut scan = Scan::new();
        for f in 0..5 {
            let samples = capture_from_frame(&render_face_frame(&cube, f)).unwrap();
            scan.set_face(f, samples);
        }
        assert_eq!(scan.captured_count(), 5);
        assert!(!scan.is_complete());
        assert!(scan.classify().is_none());
    }
}
