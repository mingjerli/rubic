//! Find the sticker cells in a camera frame.
//!
//! Each sticker is a region enclosed by edges: its own border plus the cube's
//! dark lattice. [`detect_stickers`] finds those regions from [`sticker_edges`]
//! and keeps the ones that are sticker-sized, roughly square, sticker-colored,
//! and part of a grid. Grid fitting ([`super::grid`]) then recovers whole faces
//! from them, including cells detection missed.

use super::Rgb;
use image::{GrayImage, Luma, RgbImage};
use imageproc::contours::find_contours;
use imageproc::distance_transform::Norm;
use imageproc::morphology::dilate;
use imageproc::point::Point;

/// A detected sticker: its axis-aligned bounding box `(x0, y0, x1, y1)`.
pub type StickerBox = (f32, f32, f32, f32);

/// Saturation `(max-min)/max` of an RGB pixel, `0.0..=1.0`.
fn saturation(px: Rgb) -> f32 {
    let (r, g, b) = (f32::from(px[0]), f32::from(px[1]), f32::from(px[2]));
    let max = r.max(g).max(b);
    let min = r.min(g).min(b);
    if max <= 1.0 { 0.0 } else { (max - min) / max }
}

/// Canny edges of the frame: sticker borders and the lattice. Canny finds the
/// border even between same-colored neighbors, where a color mask would merge
/// them. (Also what `capture-debug` saves, so it shows what detection sees.)
#[must_use]
pub fn sticker_edges(frame: &RgbImage) -> GrayImage {
    let gray = image::imageops::grayscale(frame);
    imageproc::edges::canny(&gray, 24.0, 72.0)
}

/// Detect individual sticker cells (any color, including white).
#[must_use]
pub fn detect_stickers(frame: &RgbImage) -> Vec<StickerBox> {
    let (w, h) = frame.dimensions();
    let frame_area = (w * h) as f32;
    // Upper bound is generous so a close-up cube (large stickers) still reads.
    let (sticker_min, sticker_max) = (frame_area * 0.0008, frame_area * 0.06);

    // Dilating the edges closes small gaps so each cell is its own connected
    // interior.
    let edges = dilate(&sticker_edges(frame), Norm::LInf, 3);
    let interior = GrayImage::from_fn(w, h, |x, y| {
        if edges.get_pixel(x, y).0[0] > 0 {
            Luma([0])
        } else {
            Luma([255])
        }
    });

    let mut out = Vec::new();
    for contour in &find_contours::<i32>(&interior) {
        let (x0, y0, x1, y1) = bbox_f(&contour.points);
        let (bw, bh) = (x1 - x0, y1 - y0);
        let bbox_area = bw * bh;
        let aspect = bw / bh;
        if bbox_area < sticker_min || bbox_area > sticker_max || !(0.5..=2.0).contains(&aspect) {
            continue;
        }
        // Keep only cells whose center looks like a sticker (colored or white),
        // dropping dark lattice gaps and shadowed background regions.
        let cx = ((x0 + x1) / 2.0) as u32;
        let cy = ((y0 + y1) / 2.0) as u32;
        let px = frame.get_pixel(cx.min(w - 1), cy.min(h - 1)).0;
        let value = f32::from(px[0].max(px[1]).max(px[2])) / 255.0;
        if saturation(px) < 0.18 && value < 0.4 {
            continue;
        }
        out.push((x0, y0, x1, y1));
    }
    keep_gridded(out)
}

/// Keep only stickers that have at least two nearby neighbors, so isolated
/// background specks are dropped and only cells that are part of a face grid
/// survive.
fn keep_gridded(boxes: Vec<StickerBox>) -> Vec<StickerBox> {
    let boxes = dedupe(boxes);
    if boxes.len() < 4 {
        return boxes;
    }
    let centers: Vec<(f32, f32)> = boxes
        .iter()
        .map(|&(x0, y0, x1, y1)| ((x0 + x1) / 2.0, (y0 + y1) / 2.0))
        .collect();
    let mut widths: Vec<f32> = boxes.iter().map(|&(x0, _, x1, _)| x1 - x0).collect();
    widths.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let radius = widths[widths.len() / 2] * 2.3;

    boxes
        .iter()
        .enumerate()
        .filter(|&(i, _)| {
            let (cx, cy) = centers[i];
            centers
                .iter()
                .enumerate()
                .filter(|&(j, &(ox, oy))| j != i && (cx - ox).hypot(cy - oy) < radius)
                .count()
                >= 2
        })
        .map(|(_, &b)| b)
        .collect()
}

/// Drop near-duplicate boxes (same cell found as inner + outer contour): keep a
/// box only if its center isn't within a third of its size of an earlier one.
fn dedupe(boxes: Vec<StickerBox>) -> Vec<StickerBox> {
    let mut kept: Vec<StickerBox> = Vec::new();
    for b in boxes {
        let (cx, cy) = ((b.0 + b.2) / 2.0, (b.1 + b.3) / 2.0);
        let near = (b.2 - b.0).min(b.3 - b.1) / 3.0;
        let dup = kept.iter().any(|k| {
            let (kx, ky) = ((k.0 + k.2) / 2.0, (k.1 + k.3) / 2.0);
            (cx - kx).hypot(cy - ky) < near
        });
        if !dup {
            kept.push(b);
        }
    }
    kept
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::fixtures::face_frame;
    use rubic_core::{Face, Facelets};

    #[test]
    fn finds_the_nine_stickers_of_a_face() {
        let frame = face_frame(&Facelets::SOLVED, Face::F);
        assert_eq!(detect_stickers(&frame).len(), 9);
    }

    #[test]
    fn a_uniform_frame_has_no_stickers() {
        let frame = RgbImage::from_pixel(160, 120, image::Rgb([18, 18, 20]));
        assert!(detect_stickers(&frame).is_empty());
    }

    /// Dev loop: run detection against the saved camera fixtures and dump an
    /// overlay to /tmp for inspection. Run with:
    /// `cargo test -p rubic --features camera fixture_stickers -- --ignored --nocapture`
    #[test]
    #[ignore = "dev tool: iterates detection against real camera fixtures"]
    fn fixture_stickers() {
        for name in ["corner", "frontal"] {
            let img = image::open(format!("tests/fixtures/{name}.png"))
                .expect("fixture present")
                .to_rgb8();
            let stickers = detect_stickers(&img);
            let faces = crate::vision::grid::fit_faces(&stickers);
            eprintln!(
                "fixture {name}: {} stickers, {} face(s)",
                stickers.len(),
                faces.len()
            );
            let mut overlay = img.clone();
            for &(x0, y0, x1, y1) in &stickers {
                let rect = imageproc::rect::Rect::at(x0 as i32, y0 as i32)
                    .of_size((x1 - x0).max(1.0) as u32, (y1 - y0).max(1.0) as u32);
                imageproc::drawing::draw_hollow_rect_mut(
                    &mut overlay,
                    rect,
                    image::Rgb([40, 255, 80]),
                );
            }
            for cells in &faces {
                // Cell pitch -> sampling patch radius.
                let pitch = ((cells[1].0 - cells[0].0).hypot(cells[1].1 - cells[0].1)).max(8.0);
                let colors = crate::vision::sample::sample_centers(&img, cells, pitch * 0.18);
                for (i, &(cx, cy)) in cells.iter().enumerate() {
                    // Filled with the SAMPLED color + white ring: if it blends
                    // into the sticker, the reading is correct.
                    let center = (cx as i32, cy as i32);
                    imageproc::drawing::draw_filled_circle_mut(
                        &mut overlay,
                        center,
                        18,
                        image::Rgb([255, 255, 255]),
                    );
                    imageproc::drawing::draw_filled_circle_mut(
                        &mut overlay,
                        center,
                        14,
                        image::Rgb(colors[i]),
                    );
                }
            }
            overlay
                .save(format!("/tmp/fixture-{name}.png"))
                .expect("save overlay");
        }
    }
}
