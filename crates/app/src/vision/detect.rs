//! Locate a cube face in a camera frame.
//!
//! Phase A keeps this deliberately simple (per the "don't over-complicate"
//! directive): find the foreground region that stands out from the background,
//! take its bounding box, and crop the largest centered square. Full contour /
//! perspective handling is only added later if on-device tuning needs it.

use super::Rgb;
use image::RgbImage;

/// Minimum face side in pixels (at least 3 px per cell).
const MIN_SIDE: u32 = 9;
/// How far a pixel must be from the background (Euclidean RGB) to be foreground.
const FOREGROUND_THRESHOLD: f32 = 45.0;

/// Find the cube face and return it cropped to a square, or `None` if no
/// plausible face-sized region is found.
#[must_use]
pub fn detect_face(frame: &RgbImage) -> Option<RgbImage> {
    let (w, h) = frame.dimensions();
    if w < MIN_SIDE || h < MIN_SIDE {
        return None;
    }

    let bg = background_color(frame);
    let thresh_sq = FOREGROUND_THRESHOLD * FOREGROUND_THRESHOLD;

    // Bounding box of all foreground (non-background) pixels.
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (w, h, 0u32, 0u32);
    let mut found = false;
    for (x, y, px) in frame.enumerate_pixels() {
        if rgb_dist_sq(px.0, bg) > thresh_sq {
            found = true;
            min_x = min_x.min(x);
            min_y = min_y.min(y);
            max_x = max_x.max(x);
            max_y = max_y.max(y);
        }
    }
    if !found {
        return None;
    }

    let bw = max_x - min_x + 1;
    let bh = max_y - min_y + 1;
    let side = bw.min(bh);
    if side < MIN_SIDE {
        return None;
    }

    // Largest square centered in the bounding box.
    let x = min_x + (bw - side) / 2;
    let y = min_y + (bh - side) / 2;
    Some(image::imageops::crop_imm(frame, x, y, side, side).to_image())
}

/// Estimate the background color as the median of the frame's border pixels.
fn background_color(frame: &RgbImage) -> Rgb {
    let (w, h) = frame.dimensions();
    let mut chans: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    let mut push = |px: &image::Rgb<u8>| {
        for (chan, &v) in chans.iter_mut().zip(px.0.iter()) {
            chan.push(v);
        }
    };
    for x in 0..w {
        push(frame.get_pixel(x, 0));
        push(frame.get_pixel(x, h - 1));
    }
    for y in 0..h {
        push(frame.get_pixel(0, y));
        push(frame.get_pixel(w - 1, y));
    }
    std::array::from_fn(|c| {
        let v = &mut chans[c];
        v.sort_unstable();
        v[v.len() / 2]
    })
}

/// Squared Euclidean distance between two RGB colors.
fn rgb_dist_sq(a: Rgb, b: Rgb) -> f32 {
    (0..3)
        .map(|c| {
            let d = f32::from(a[c]) - f32::from(b[c]);
            d * d
        })
        .sum()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::Rgb;
    use crate::vision::sample::sample_face;

    const FACE: [[u8; 3]; 9] = [
        [240, 240, 240],
        [200, 20, 20],
        [20, 140, 60],
        [230, 200, 20],
        [230, 120, 20],
        [20, 60, 200],
        [200, 20, 20],
        [20, 140, 60],
        [240, 240, 240],
    ];

    /// A frame with a square cube face painted on a plain background.
    fn frame_with_face(bg: Rgb, fx: u32, fy: u32, fsize: u32, w: u32, h: u32) -> RgbImage {
        let cell = fsize / 3;
        RgbImage::from_fn(w, h, |x, y| {
            if x >= fx && x < fx + fsize && y >= fy && y < fy + fsize {
                let cx = ((x - fx) / cell).min(2);
                let cy = ((y - fy) / cell).min(2);
                image::Rgb(FACE[(cy * 3 + cx) as usize])
            } else {
                image::Rgb(bg)
            }
        })
    }

    #[test]
    fn finds_face_and_recovers_its_colors() {
        let frame = frame_with_face([15, 15, 18], 30, 20, 60, 120, 100);
        let cropped = detect_face(&frame).expect("face found");
        let got = sample_face(&cropped);
        for (g, want) in got.iter().zip(FACE.iter()) {
            for k in 0..3 {
                assert!(
                    i32::from(g[k]).abs_diff(i32::from(want[k])) <= 8,
                    "cell drift: {g:?} vs {want:?}"
                );
            }
        }
    }

    #[test]
    fn uniform_frame_has_no_face() {
        let frame = RgbImage::from_pixel(80, 80, image::Rgb([15, 15, 18]));
        assert!(detect_face(&frame).is_none());
    }
}
