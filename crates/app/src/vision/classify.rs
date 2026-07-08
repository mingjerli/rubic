//! Classify sampled colors into cube faces, relative to the six centers.

use super::Rgb;
use super::color::{perceptual_point, point_distance_sq};
use rubic_core::{Face, Facelets};

/// The outcome of classifying 54 samples.
#[derive(Debug, Clone)]
pub struct Classified {
    /// The classified cube (every sticker assigned to a face color).
    pub facelets: Facelets,
    /// Smallest classification margin across all stickers (larger = more
    /// confident); the gap between the nearest and second-nearest center.
    pub min_margin: f32,
}

/// Classify 54 RGB samples (URFDLB facelet order) into a [`Facelets`], using the
/// six center samples (index `f*9+4`) as the reference color for each face.
#[must_use]
pub fn classify(samples: &[Rgb; 54]) -> Classified {
    // The six center stickers are the reference color for their face.
    let centers: [[f32; 3]; 6] = std::array::from_fn(|f| perceptual_point(samples[f * 9 + 4]));

    let mut faces = [Face::U; 54];
    let mut min_margin = f32::INFINITY;

    for (i, slot) in faces.iter_mut().enumerate() {
        let point = perceptual_point(samples[i]);

        // Nearest and second-nearest center (by distance).
        let (mut best_d, mut best_f, mut second_d) = (f32::INFINITY, 0usize, f32::INFINITY);
        for (f, center) in centers.iter().enumerate() {
            let d = point_distance_sq(point, *center);
            if d < best_d {
                second_d = best_d;
                best_d = d;
                best_f = f;
            } else if d < second_d {
                second_d = d;
            }
        }

        *slot = Face::ALL[best_f];
        let margin = second_d.sqrt() - best_d.sqrt();
        min_margin = min_margin.min(margin);
    }

    let string: String = faces.iter().map(|f| f.to_char()).collect();
    let facelets = string.parse::<Facelets>().expect("54 valid face labels");
    Classified {
        facelets,
        min_margin,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::colors::sticker_rgb;
    use rubic_core::{Face, Sequence};

    /// Render a face's sticker color as an 8-bit RGB sample.
    fn face_rgb(f: Face) -> Rgb {
        let c = sticker_rgb(f);
        [
            (c[0] * 255.0) as u8,
            (c[1] * 255.0) as u8,
            (c[2] * 255.0) as u8,
        ]
    }

    /// Build 54 clean samples from a cube's sticker colors.
    fn samples_from(f: &Facelets) -> [Rgb; 54] {
        std::array::from_fn(|i| face_rgb(f.get(i)))
    }

    fn scramble(s: &str) -> Facelets {
        Facelets::SOLVED.apply_seq(&s.parse::<Sequence>().unwrap())
    }

    #[test]
    fn solved_classifies_to_solved() {
        let out = classify(&samples_from(&Facelets::SOLVED));
        assert_eq!(out.facelets, Facelets::SOLVED);
    }

    #[test]
    fn scramble_is_recovered_exactly() {
        let f = scramble("R U R' U' F2 L D B'");
        let out = classify(&samples_from(&f));
        assert_eq!(out.facelets, f);
    }

    #[test]
    fn robust_to_uniform_dim_lighting() {
        // Relative classification: dimming every sample keeps the centers as
        // valid references, so the cube is still recovered.
        let f = scramble("R U2 F' L D B R'");
        let mut s = samples_from(&f);
        for px in &mut s {
            for c in px.iter_mut() {
                *c = (f32::from(*c) * 0.55) as u8;
            }
        }
        assert_eq!(classify(&s).facelets, f);
    }

    #[test]
    fn margin_is_positive_for_clean_input() {
        assert!(classify(&samples_from(&Facelets::SOLVED)).min_margin > 0.0);
    }
}
