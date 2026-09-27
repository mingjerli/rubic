//! From a camera frame to a face's nine colors, and from six faces to a cube.
//!
//! [`read_face_grid`] reads the face in view: detect sticker cells, fit face
//! grids, and sample the nine predicted cell centers of the most frontal one.
//! [`FaceSamples`] collects six captured faces (each in URFDLB-local row-major
//! order) and classifies them into a cube once complete. Arranging each
//! physical face into its facelet slot is the capture flow's job.

use super::Rgb;
use super::classify::classify as classify_samples;
use super::detect::detect_stickers;
use super::grid::fit_faces;
use super::sample::sample_centers;
use image::RgbImage;
use rubic_core::Facelets;

/// Up to six captured faces, each nine samples. Face slot `f` corresponds to
/// [`rubic_core::Face`] index `f`.
#[derive(Debug, Clone, Default)]
pub struct FaceSamples {
    faces: [Option<[Rgb; 9]>; 6],
}

impl FaceSamples {
    /// Record the nine samples for face slot `f` (`0..6`).
    pub fn set_face(&mut self, f: usize, samples: [Rgb; 9]) {
        self.faces[f] = Some(samples);
    }

    /// Whether face slot `f` has been captured.
    #[must_use]
    pub fn has_face(&self, f: usize) -> bool {
        self.faces.get(f).is_some_and(Option::is_some)
    }

    /// Classify the six faces into a cube, or `None` until all six are in.
    #[must_use]
    pub fn classify(&self) -> Option<Facelets> {
        let mut samples = [[0u8; 3]; 54];
        for (f, slot) in self.faces.iter().enumerate() {
            let face = (*slot)?;
            samples[f * 9..f * 9 + 9].copy_from_slice(&face);
        }
        Some(classify_samples(&samples))
    }
}

/// Cell pitch of a fitted face (distance between adjacent predicted centers).
fn face_pitch(face: &[(f32, f32); 9]) -> f32 {
    (face[1].0 - face[0].0).hypot(face[1].1 - face[0].1)
}

/// Read the face in view: its nine colors and the nine fitted cell centers (so
/// a live preview can draw the colors where they were read). `None` if no face
/// grid is found.
///
/// Detects sticker cells, fits face grids, and takes the most frontal one
/// (largest cell pitch = closest / least foreshortened). Even with only a few
/// cells cleanly detected, the grid recovers all nine.
#[must_use]
pub fn read_face_grid(frame: &RgbImage) -> Option<([Rgb; 9], [(f32, f32); 9])> {
    let stickers = detect_stickers(frame);
    let face = fit_faces(&stickers).into_iter().max_by(|a, b| {
        face_pitch(a)
            .partial_cmp(&face_pitch(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })?;
    // Reject only a face that falls well outside the frame (a small margin is
    // fine: sampling clamps, and a frame-filling face has edge cells near the
    // border). This keeps a large, mostly-in-view face readable.
    let (w, h) = (frame.width() as f32, frame.height() as f32);
    let (mx, my) = (w * 0.08, h * 0.08);
    let in_frame = face
        .iter()
        .all(|&(x, y)| x >= -mx && y >= -my && x < w + mx && y < h + my);
    if !in_frame {
        return None;
    }
    let radius = (face_pitch(&face).max(8.0)) * 0.18;
    Some((sample_centers(frame, &face, radius), face))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::fixtures::face_frame;
    use rubic_core::{Face, Sequence};

    fn scramble(s: &str) -> Facelets {
        Facelets::SOLVED.apply_seq(&s.parse::<Sequence>().unwrap())
    }

    #[test]
    fn six_read_faces_recover_and_validate_the_cube() {
        let cube = scramble("R U R' U' F2 L D B' R2 U");
        let mut faces = FaceSamples::default();
        for face in Face::ALL {
            let (samples, _) = read_face_grid(&face_frame(&cube, face)).expect("face read");
            faces.set_face(face.index(), samples);
        }
        let classified = faces.classify().expect("six faces classify");
        assert_eq!(classified, cube, "recovered cube mismatch");
        assert!(classified.validate().is_ok());
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
        let (got, _) = read_face_grid(&render_bordered_face(colors)).expect("face grid read");
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
    fn a_uniform_frame_has_no_face() {
        let frame = RgbImage::from_pixel(160, 120, image::Rgb([18, 18, 20]));
        assert!(read_face_grid(&frame).is_none());
    }

    #[test]
    fn five_faces_do_not_classify() {
        let mut faces = FaceSamples::default();
        for f in 0..5 {
            faces.set_face(f, [[128, 128, 128]; 9]);
        }
        assert!(faces.has_face(4) && !faces.has_face(5));
        assert!(faces.classify().is_none());
    }
}
