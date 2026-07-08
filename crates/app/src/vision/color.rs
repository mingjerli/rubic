//! Color helpers for classification.
//!
//! Classification compares colors in a small perceptual space that separates
//! the cube's six colors robustly under lighting changes: an HSV-derived point
//! `(s·cos h, s·sin h, w·v)`. Chroma (hue + saturation) lands on the x/y plane
//! and brightness on z, so red/orange separate by hue and white separates from
//! colored stickers by its low chroma (near the origin).
//!
//! Brightness (`v`) is deliberately **down-weighted** (`BRIGHTNESS_WEIGHT`):
//! uneven lighting, glare, and shadow across the cube shift `v` far more than
//! hue/chroma, so leaning on hue+chroma makes classification much less
//! light-sensitive. `v` still carries a little weight so genuinely achromatic
//! shades don't collapse together.

use super::Rgb;

/// How much the brightness axis counts relative to hue/chroma. Small, so
/// lighting/glare/shadow (which mostly move brightness) barely affect distance.
const BRIGHTNESS_WEIGHT: f32 = 0.35;

/// Convert an RGB sample to a chroma/brightness point `(s·cos h, s·sin h, w·v)`.
#[must_use]
pub fn perceptual_point(rgb: Rgb) -> [f32; 3] {
    let r = f32::from(rgb[0]) / 255.0;
    let g = f32::from(rgb[1]) / 255.0;
    let b = f32::from(rgb[2]) / 255.0;

    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    let chroma = max - min;

    // HSV value and saturation.
    let v = max;
    let s = if max <= f32::EPSILON {
        0.0
    } else {
        chroma / max
    };

    // HSV hue in radians (0 when achromatic).
    let hue = if chroma <= f32::EPSILON {
        0.0
    } else if (max - r).abs() < f32::EPSILON {
        std::f32::consts::FRAC_PI_3 * (((g - b) / chroma) % 6.0)
    } else if (max - g).abs() < f32::EPSILON {
        std::f32::consts::FRAC_PI_3 * ((b - r) / chroma + 2.0)
    } else {
        std::f32::consts::FRAC_PI_3 * ((r - g) / chroma + 4.0)
    };

    [s * hue.cos(), s * hue.sin(), BRIGHTNESS_WEIGHT * v]
}

/// Squared Euclidean distance between two perceptual points.
#[must_use]
pub fn point_distance_sq(a: [f32; 3], b: [f32; 3]) -> f32 {
    let dx = a[0] - b[0];
    let dy = a[1] - b[1];
    let dz = a[2] - b[2];
    dx * dx + dy * dy + dz * dz
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dist(a: Rgb, b: Rgb) -> f32 {
        point_distance_sq(perceptual_point(a), perceptual_point(b)).sqrt()
    }

    #[test]
    fn identical_colors_have_zero_distance() {
        assert!(dist([200, 30, 30], [200, 30, 30]).abs() < 1e-6);
    }

    #[test]
    fn white_and_yellow_are_separable() {
        // White (low saturation) vs yellow (high saturation) must be distinct.
        let d = dist([240, 240, 240], [240, 220, 20]);
        assert!(d > 0.3, "white/yellow too close: {d}");
    }

    #[test]
    fn red_and_orange_are_separable() {
        let d = dist([200, 20, 20], [230, 120, 20]);
        assert!(d > 0.2, "red/orange too close: {d}");
    }

    #[test]
    fn brightness_shift_moves_less_than_hue_change() {
        // A lighting (brightness) shift of the same hue should be closer than a
        // change to a different cube color.
        let same_hue_dimmer = dist([200, 20, 20], [150, 15, 15]);
        let different_color = dist([200, 20, 20], [20, 120, 60]);
        assert!(
            same_hue_dimmer < different_color,
            "dimmer {same_hue_dimmer} should beat different {different_color}"
        );
    }
}
