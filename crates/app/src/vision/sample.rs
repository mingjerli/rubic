//! Sample a face's nine sticker colors at its fitted cell centers.

use super::Rgb;
use image::RgbImage;

/// Read the nine cell colors at explicit grid-cell centers (from grid-fitting),
/// row-major. Each color is the per-channel median of a small patch so glare
/// and grid lines near a cell edge don't skew it.
#[must_use]
pub fn sample_centers(img: &RgbImage, centers: &[(f32, f32); 9], radius: f32) -> [Rgb; 9] {
    let (w, h) = img.dimensions();
    std::array::from_fn(|i| {
        // Clamp the cell center into the image, then bound the patch to a valid
        // half-open range (a predicted cell can fall outside a partial frame).
        let cx = centers[i].0.clamp(0.0, (w - 1) as f32);
        let cy = centers[i].1.clamp(0.0, (h - 1) as f32);
        let x0 = (cx - radius).max(0.0) as u32;
        let y0 = (cy - radius).max(0.0) as u32;
        let x1 = (((cx + radius) as u32) + 1).clamp(x0 + 1, w);
        let y1 = (((cy + radius) as u32) + 1).clamp(y0 + 1, h);
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

    /// A 90x90 face image whose nine 30px cells are `colors`.
    fn make_face(colors: [Rgb; 9]) -> RgbImage {
        RgbImage::from_fn(90, 90, |x, y| {
            let (cx, cy) = ((x / 30).min(2), (y / 30).min(2));
            image::Rgb(colors[(cy * 3 + cx) as usize])
        })
    }

    /// The nine cell centers of [`make_face`], row-major.
    fn cell_centers() -> [(f32, f32); 9] {
        std::array::from_fn(|i| (15.0 + 30.0 * (i % 3) as f32, 15.0 + 30.0 * (i / 3) as f32))
    }

    #[test]
    fn reads_nine_solid_cells_row_major() {
        assert_eq!(
            sample_centers(&make_face(COLORS), &cell_centers(), 5.0),
            COLORS
        );
    }

    #[test]
    fn sample_centers_handles_out_of_bounds() {
        // Centers off the top-left and bottom-right must clamp, not panic.
        let centers = [
            (-50.0, -50.0),
            (45.0, -10.0),
            (200.0, -5.0),
            (-5.0, 45.0),
            (45.0, 45.0),
            (95.0, 45.0),
            (-10.0, 200.0),
            (45.0, 200.0),
            (200.0, 200.0),
        ];
        let _ = sample_centers(&make_face(COLORS), &centers, 8.0); // no panic
    }

    #[test]
    fn robust_to_noise_and_grid_lines() {
        let mut img = make_face(COLORS);
        // Draw black grid lines between cells and add mild per-pixel noise.
        for (x, y, px) in img.enumerate_pixels_mut() {
            if x % 30 == 0 || y % 30 == 0 {
                *px = image::Rgb([0, 0, 0]);
            } else {
                let jitter = i32::from((x ^ y) as u8 % 11) - 5;
                for c in &mut px.0 {
                    *c = (i32::from(*c) + jitter).clamp(0, 255) as u8;
                }
            }
        }
        // The median of a central patch ignores the borders; colors stay close.
        let got = sample_centers(&img, &cell_centers(), 5.0);
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
