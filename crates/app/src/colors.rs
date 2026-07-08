//! Sticker color mapping. Pure and unit-testable: no Bevy types leak in here so
//! the color choices can be verified without a renderer.
//!
//! Called by `cube_render.rs` (to build materials) and indirectly by `ui.rs`.
//! The scheme is the standard Western coloring required by the spec:
//! U=white, R=red, F=green, D=yellow, L=orange, B=blue.

use rubic_core::Face;

/// sRGB triples for each face's sticker color, `0.0..=1.0`.
///
/// These are plain sRGB component triples so they can be tested independently
/// of any rendering backend; `cube_render` converts them into materials.
#[must_use]
pub fn sticker_rgb(face: Face) -> [f32; 3] {
    match face {
        Face::U => [0.95, 0.95, 0.95], // white
        Face::R => [0.72, 0.07, 0.07], // red
        Face::F => [0.00, 0.55, 0.20], // green
        Face::D => [0.95, 0.82, 0.05], // yellow
        Face::L => [0.90, 0.45, 0.05], // orange
        Face::B => [0.00, 0.30, 0.72], // blue
    }
}

/// Color of interior (hidden) cubie faces and the cubie body: near-black.
#[must_use]
pub fn body_rgb() -> [f32; 3] {
    [0.04, 0.04, 0.05]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_face_has_a_distinct_color() {
        let mut seen = Vec::new();
        for face in Face::ALL {
            let rgb = sticker_rgb(face);
            assert!(
                !seen.contains(&rgb),
                "duplicate color for {}",
                face.to_char()
            );
            seen.push(rgb);
        }
        assert_eq!(seen.len(), 6);
    }

    #[test]
    fn components_are_in_unit_range() {
        for face in Face::ALL {
            for c in sticker_rgb(face) {
                assert!((0.0..=1.0).contains(&c));
            }
        }
        for c in body_rgb() {
            assert!((0.0..=1.0).contains(&c));
        }
    }

    #[test]
    fn white_is_brightest_and_matches_up() {
        // Sanity check that U maps to the brightest (white) swatch.
        let up_sum: f32 = sticker_rgb(Face::U).iter().sum();
        for face in [Face::R, Face::F, Face::D, Face::L, Face::B] {
            let sum: f32 = sticker_rgb(face).iter().sum();
            assert!(up_sum >= sum);
        }
    }
}
