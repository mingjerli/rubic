//! Sample the 3×3 grid of a detected, squared-up face image.

use super::Rgb;
use image::RgbImage;

/// Read the nine cell colors of a square face image, row-major (top-left
/// first). Each cell's color is the per-channel median of a central patch, so
/// grid lines and edge pixels near cell borders don't skew the reading.
/// Fraction of each border ignored, so a slightly-too-large detected box (e.g.
/// one that caught a sliver of an adjacent face) doesn't shift the grid onto
/// gaps or the wrong stickers.
const INSET: f32 = 0.08;
/// Half-size of each cell's sampled patch, as a fraction of the cell.
const PATCH: f32 = 0.18;

#[must_use]
pub fn sample_face(img: &RgbImage) -> [Rgb; 9] {
    let (w, h) = img.dimensions();
    let (wf, hf) = (w as f32, h as f32);
    let (ox, oy) = (wf * INSET, hf * INSET);
    let cw = (wf - 2.0 * ox) / 3.0;
    let ch = (hf - 2.0 * oy) / 3.0;
    std::array::from_fn(|idx| {
        let (cx, cy) = ((idx % 3) as f32, (idx / 3) as f32);
        // Center of this cell, then a small patch around it.
        let ccx = ox + (cx + 0.5) * cw;
        let ccy = oy + (cy + 0.5) * ch;
        let x0 = (ccx - PATCH * cw).max(0.0) as u32;
        let y0 = (ccy - PATCH * ch).max(0.0) as u32;
        let x1 = ((ccx + PATCH * cw) as u32).min(w - 1).max(x0 + 1);
        let y1 = ((ccy + PATCH * ch) as u32).min(h - 1).max(y0 + 1);
        patch_median(img, x0, x1, y0, y1)
    })
}

/// Read the nine cell colors at explicit grid-cell centers (from grid-fitting),
/// row-major. Each color is the per-channel median of a small patch so glare
/// and grid lines near a cell edge don't skew it.
#[must_use]
pub fn sample_centers(img: &RgbImage, centers: &[(f32, f32); 9], radius: f32) -> [Rgb; 9] {
    let (w, h) = img.dimensions();
    std::array::from_fn(|i| {
        let (cx, cy) = centers[i];
        let x0 = (cx - radius).max(0.0) as u32;
        let y0 = (cy - radius).max(0.0) as u32;
        let x1 = ((cx + radius) as u32).min(w - 1).max(x0 + 1);
        let y1 = ((cy + radius) as u32).min(h - 1).max(y0 + 1);
        patch_median(img, x0, x1, y0, y1)
    })
}

/// Per-channel median color of the pixels in `[x0, x1) × [y0, y1)`.
fn patch_median(img: &RgbImage, x0: u32, x1: u32, y0: u32, y1: u32) -> Rgb {
    let mut chans: [Vec<u8>; 3] = [Vec::new(), Vec::new(), Vec::new()];
    for y in y0..y1 {
        for x in x0..x1 {
            let px = img.get_pixel(x, y).0;
            for c in 0..3 {
                chans[c].push(px[c]);
            }
        }
    }
    std::array::from_fn(|c| {
        let v = &mut chans[c];
        v.sort_unstable();
        v[v.len() / 2]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const COLORS: [Rgb; 9] = [
        [240, 240, 240],
        [200, 20, 20],
        [20, 140, 60],
        [230, 200, 20],
        [230, 120, 20],
        [20, 60, 200],
        [240, 240, 240],
        [200, 20, 20],
        [20, 140, 60],
    ];

    /// Build a `size`×`size` face image whose nine cells are `colors`.
    fn make_face(colors: [Rgb; 9], size: u32) -> RgbImage {
        let cell = size / 3;
        RgbImage::from_fn(size, size, |x, y| {
            let cx = (x / cell).min(2);
            let cy = (y / cell).min(2);
            image::Rgb(colors[(cy * 3 + cx) as usize])
        })
    }

    #[test]
    fn reads_nine_solid_cells_row_major() {
        let got = sample_face(&make_face(COLORS, 90));
        assert_eq!(got, COLORS);
    }

    #[test]
    fn robust_to_noise_and_grid_lines() {
        let size = 90;
        let cell = size / 3;
        let mut img = make_face(COLORS, size);
        // Draw black grid lines between cells and add mild per-pixel noise.
        for (x, y, px) in img.enumerate_pixels_mut() {
            if x % cell == 0 || y % cell == 0 {
                *px = image::Rgb([0, 0, 0]);
            } else {
                let jitter = i32::from((x ^ y) as u8 % 11) - 5;
                for c in &mut px.0 {
                    *c = (i32::from(*c) + jitter).clamp(0, 255) as u8;
                }
            }
        }
        // Median of a central patch ignores the borders; colors stay close.
        let got = sample_face(&img);
        for (g, want) in got.iter().zip(COLORS.iter()) {
            for k in 0..3 {
                assert!(
                    i32::from(g[k]).abs_diff(i32::from(want[k])) <= 6,
                    "cell channel drift too large: {g:?} vs {want:?}"
                );
            }
        }
    }
}
