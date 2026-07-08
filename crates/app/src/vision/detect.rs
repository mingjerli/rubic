//! Locate a cube face in a camera frame.
//!
//! Phase A keeps this deliberately simple (per the "don't over-complicate"
//! directive): find the foreground region that stands out from the background,
//! take its bounding box, and crop the largest centered square. Full contour /
//! perspective handling is only added later if on-device tuning needs it.

use super::Rgb;
use super::sample::sample_face;
use image::{GrayImage, Luma, RgbImage};
use imageproc::contours::find_contours;
use imageproc::distance_transform::Norm;
use imageproc::drawing::draw_line_segment_mut;
use imageproc::geometric_transformations::{Interpolation, Projection, warp};
use imageproc::morphology::{dilate, erode};
use imageproc::point::Point;

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

// --- Automatic quad detection (contours + perspective warp) -----------------

/// Side length the detected face is warped to before sampling.
const WARP_SIZE: u32 = 120;

/// Corners of a detected face, ordered top-left, top-right, bottom-right,
/// bottom-left, in frame pixel coordinates.
pub type Quad = [(f32, f32); 4];

/// Saturation `(max-min)/max` of an RGB pixel, `0.0..=1.0`.
fn saturation(px: Rgb) -> f32 {
    let (r, g, b) = (f32::from(px[0]), f32::from(px[1]), f32::from(px[2]));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max <= 1.0 { 0.0 } else { (max - min) / max }
}

/// Minimum saturation for a pixel to count as a colored sticker.
const SAT_THRESHOLD: f32 = 0.30;

/// Detect the cube face as the largest square-ish saturated blob near the
/// center.
///
/// Against a plain background the colored stickers form one connected region
/// (they merge across the thin black grid lines; white stickers are interior
/// holes), which shows up as a single blob distinct from the dark background.
/// We take its bounding box: every face edge is touched by a colored sticker,
/// so white corners don't shrink it. Centrality and a size range reject
/// background specks and anything spanning the whole frame.
#[must_use]
pub fn detect_face_quad(frame: &RgbImage) -> Option<Quad> {
    let (w, h) = frame.dimensions();
    let mask = GrayImage::from_fn(w, h, |x, y| {
        if saturation(frame.get_pixel(x, y).0) > SAT_THRESHOLD {
            Luma([255])
        } else {
            Luma([0])
        }
    });
    // Dilate to bridge the black grid lines so the face is one blob.
    let mask = dilate(&mask, Norm::LInf, 6);
    let contours = find_contours::<i32>(&mask);

    let frame_area = (w * h) as f32;
    let min_area = frame_area * 0.02;
    let max_area = frame_area * 0.7;
    let (rx0, ry0, rx1, ry1) = (
        w as f32 * 0.1,
        h as f32 * 0.02,
        w as f32 * 0.9,
        h as f32 * 0.98,
    );

    let mut best: Option<((f32, f32, f32, f32), f32)> = None;
    for contour in &contours {
        if contour.points.len() < 30 {
            continue;
        }
        let (bx0, by0, bx1, by1) = bbox_f(&contour.points);
        let (bw, bh) = (bx1 - bx0, by1 - by0);
        let bbox_area = bw * bh;
        let aspect = bw / bh;
        if bbox_area < min_area || bbox_area > max_area || !(0.6..=1.6).contains(&aspect) {
            continue;
        }
        let (cx, cy) = ((bx0 + bx1) / 2.0, (by0 + by1) / 2.0);
        if cx < rx0 || cx > rx1 || cy < ry0 || cy > ry1 {
            continue;
        }
        if best.is_none_or(|(_, a)| bbox_area > a) {
            best = Some(((bx0, by0, bx1, by1), bbox_area));
        }
    }
    best.map(|((x0, y0, x1, y1), _)| [(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
}

/// A detected sticker: its axis-aligned bounding box `(x0, y0, x1, y1)`.
pub type StickerBox = (f32, f32, f32, f32);

/// Luma below this counts as the black grid lattice / dark border.
const DARK_THRESHOLD: u8 = 90;

/// Detect individual sticker cells (any color, including white) by the black
/// grid that separates them.
///
/// The cube's black lattice isolates each sticker into its own bright cell.
/// Masking out dark pixels (the lattice) and eroding a little separates the
/// cells; each resulting connected component that is sticker-sized and roughly
/// square is one sticker. Unlike a saturation mask, this finds white stickers.
#[must_use]
pub fn detect_stickers(frame: &RgbImage) -> Vec<StickerBox> {
    let (w, h) = frame.dimensions();
    let gray = image::imageops::grayscale(frame);
    // Bright (non-lattice) regions -> candidate sticker cells + background.
    let light = GrayImage::from_fn(w, h, |x, y| {
        if gray.get_pixel(x, y).0[0] < DARK_THRESHOLD {
            Luma([0])
        } else {
            Luma([255])
        }
    });
    // Shrink the bright regions (thicken the lattice) so touching cells split.
    let light = erode(&light, Norm::LInf, 4);
    let contours = find_contours::<i32>(&light);

    let frame_area = (w * h) as f32;
    let (sticker_min, sticker_max) = (frame_area * 0.0008, frame_area * 0.03);
    let mut out = Vec::new();
    for contour in &contours {
        let (x0, y0, x1, y1) = bbox_f(&contour.points);
        let (bw, bh) = (x1 - x0, y1 - y0);
        let bbox_area = bw * bh;
        let aspect = bw / bh;
        if bbox_area < sticker_min || bbox_area > sticker_max || !(0.5..=2.0).contains(&aspect) {
            continue;
        }
        out.push((x0, y0, x1, y1));
    }
    out
}

/// The dilated saturation mask used by [`detect_face_quad`], for debugging.
#[must_use]
pub fn debug_saturation_mask(frame: &RgbImage) -> GrayImage {
    let (w, h) = frame.dimensions();
    let mask = GrayImage::from_fn(w, h, |x, y| {
        if saturation(frame.get_pixel(x, y).0) > SAT_THRESHOLD {
            Luma([255])
        } else {
            Luma([0])
        }
    });
    dilate(&mask, Norm::LInf, 3)
}

/// Axis-aligned bounding box `(x0, y0, x1, y1)` of a contour's points.
fn bbox_f(points: &[Point<i32>]) -> (f32, f32, f32, f32) {
    let (mut x0, mut y0, mut x1, mut y1) = (i32::MAX, i32::MAX, i32::MIN, i32::MIN);
    for p in points {
        x0 = x0.min(p.x);
        y0 = y0.min(p.y);
        x1 = x1.max(p.x);
        y1 = y1.max(p.y);
    }
    (x0 as f32, y0 as f32, x1 as f32, y1 as f32)
}

/// The four extreme corners of a contour (min/max of `x±y`).
fn quad_corners(points: &[Point<i32>]) -> Quad {
    let (mut tl, mut br, mut tr, mut bl) = (points[0], points[0], points[0], points[0]);
    for p in points {
        if p.x + p.y < tl.x + tl.y {
            tl = *p;
        }
        if p.x + p.y > br.x + br.y {
            br = *p;
        }
        if p.x - p.y > tr.x - tr.y {
            tr = *p;
        }
        if p.x - p.y < bl.x - bl.y {
            bl = *p;
        }
    }
    let f = |p: Point<i32>| (p.x as f32, p.y as f32);
    [f(tl), f(tr), f(br), f(bl)]
}

fn dist(a: (f32, f32), b: (f32, f32)) -> f32 {
    (a.0 - b.0).hypot(a.1 - b.1)
}

/// Shoelace area of the (ordered) quad.
fn quad_area(c: Quad) -> f32 {
    let mut area = 0.0;
    for i in 0..4 {
        let (x1, y1) = c[i];
        let (x2, y2) = c[(i + 1) % 4];
        area += x1 * y2 - x2 * y1;
    }
    area.abs() / 2.0
}

/// Whether the quad's sides are all similar in length (roughly a square).
fn is_squareish(c: Quad) -> bool {
    let sides = [
        dist(c[0], c[1]),
        dist(c[1], c[2]),
        dist(c[2], c[3]),
        dist(c[3], c[0]),
    ];
    let min = sides.iter().copied().fold(f32::MAX, f32::min);
    let max = sides.iter().copied().fold(0.0, f32::max);
    min > 4.0 && max / min < 2.2
}

/// Perspective-warp the quad region flat to a `size`x`size` image.
#[must_use]
pub fn warp_face(frame: &RgbImage, corners: Quad, size: u32) -> Option<RgbImage> {
    let s = size as f32;
    let dst = [(0.0, 0.0), (s, 0.0), (s, s), (0.0, s)];
    let projection = Projection::from_control_points(corners, dst)?;
    let warped = warp(
        frame,
        &projection,
        Interpolation::Bilinear,
        image::Rgb([0, 0, 0]),
    );
    Some(image::imageops::crop_imm(&warped, 0, 0, size, size).to_image())
}

/// Detect, warp, and sample a face automatically, returning the nine colors and
/// the detected quad (for drawing an overlay).
#[must_use]
pub fn capture_quad(frame: &RgbImage) -> Option<([Rgb; 9], Quad)> {
    let corners = detect_face_quad(frame)?;
    let face = warp_face(frame, corners, WARP_SIZE)?;
    Some((sample_face(&face), corners))
}

/// Draw the quad outline onto an image (to show what was detected).
pub fn draw_quad(img: &mut RgbImage, quad: Quad, color: image::Rgb<u8>) {
    for i in 0..4 {
        draw_line_segment_mut(img, quad[i], quad[(i + 1) % 4], color);
    }
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

    #[test]
    fn no_quad_in_uniform_frame() {
        // No saturated pixels -> no sticker candidates -> no detection.
        let frame = RgbImage::from_pixel(120, 120, image::Rgb([15, 15, 18]));
        assert!(detect_face_quad(&frame).is_none());
    }

    #[test]
    fn warp_face_squares_a_quad() {
        // A synthetic solid-color square warps to a same-color square.
        let frame = RgbImage::from_pixel(200, 200, image::Rgb([20, 140, 60]));
        let quad = [(40.0, 40.0), (160.0, 40.0), (160.0, 160.0), (40.0, 160.0)];
        let warped = warp_face(&frame, quad, 60).expect("warp");
        let mid = warped.get_pixel(30, 30).0;
        assert!(
            mid[1] > mid[0] && mid[1] > mid[2],
            "expected green, got {mid:?}"
        );
    }
}
