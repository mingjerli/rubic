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
///
/// A real cube has exactly nine stickers of each color, so we don't classify
/// stickers independently (which lets counts drift to 10/8 under lighting and
/// produces an invalid cube). Instead we solve a capacity-constrained
/// assignment — each of the six colors gets exactly nine stickers — greedily by
/// smallest color distance. This forces borderline stickers (red vs orange,
/// white vs yellow) onto whichever color still has room, matching the cube's
/// real structure.
#[must_use]
pub fn classify(samples: &[Rgb; 54]) -> Classified {
    // The six center stickers are the reference color for their face.
    let centers: [[f32; 3]; 6] = std::array::from_fn(|f| perceptual_point(samples[f * 9 + 4]));
    let points: [[f32; 3]; 54] = std::array::from_fn(|i| perceptual_point(samples[i]));

    let mut assigned: [Option<usize>; 54] = [None; 54];
    let mut counts = [0usize; 6];
    // Centers are fixed to their own face.
    for f in 0..6 {
        assigned[f * 9 + 4] = Some(f);
        counts[f] += 1;
    }

    // All (cost, sticker, color) pairs for non-centers, cheapest first.
    let mut pairs: Vec<(f32, usize, usize)> = Vec::with_capacity(48 * 6);
    for (i, point) in points.iter().enumerate() {
        if i % 9 == 4 {
            continue; // center already assigned
        }
        for (f, center) in centers.iter().enumerate() {
            pairs.push((point_distance_sq(*point, *center), i, f));
        }
    }
    pairs.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal));
    for (_, i, f) in pairs {
        if assigned[i].is_none() && counts[f] < 9 {
            assigned[i] = Some(f);
            counts[f] += 1;
        }
    }

    let faces: [Face; 54] = std::array::from_fn(|i| Face::ALL[assigned[i].unwrap_or(0)]);

    // Confidence: smallest gap between the nearest and second-nearest center.
    let mut min_margin = f32::INFINITY;
    for point in &points {
        let (mut best, mut second) = (f32::INFINITY, f32::INFINITY);
        for center in &centers {
            let d = point_distance_sq(*point, *center);
            if d < best {
                second = best;
                best = d;
            } else if d < second {
                second = d;
            }
        }
        min_margin = min_margin.min(second.sqrt() - best.sqrt());
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

    #[test]
    fn always_exactly_nine_of_each_color() {
        // Even with heavy per-sticker noise that would drift an independent
        // classifier's counts, the constrained assignment yields 9 of each.
        let f = scramble("R U2 F' L D B R' F2 U");
        let mut s = samples_from(&f);
        for (i, px) in s.iter_mut().enumerate() {
            let n = (i as i32 % 9) - 4;
            for c in px.iter_mut() {
                *c = (i32::from(*c) + n * 7).clamp(0, 255) as u8;
            }
        }
        let out = classify(&s);
        let mut counts = std::collections::HashMap::new();
        for ch in out.facelets.to_string().chars() {
            *counts.entry(ch).or_insert(0) += 1;
        }
        assert_eq!(counts.len(), 6, "should use exactly six colors");
        assert!(counts.values().all(|&c| c == 9), "9 of each: {counts:?}");
    }
}
