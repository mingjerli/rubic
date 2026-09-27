//! Synthetic camera frames for tests: one face of a known cube, drawn like a
//! real cube held up to the camera (dark body, dark gaps between stickers,
//! a dim background).

use image::RgbImage;
use rubic_core::{Face, Facelets};

use crate::colors::sticker_rgb;
use crate::vision::Rgb;

/// A sticker's ideal color as camera RGB.
#[must_use]
pub fn face_rgb(face: Face) -> Rgb {
    let c = sticker_rgb(face);
    [
        (c[0] * 255.0) as u8,
        (c[1] * 255.0) as u8,
        (c[2] * 255.0) as u8,
    ]
}

/// A 640x480 frame showing `face` of `cube`, centered, 240px across.
#[must_use]
pub fn face_frame(cube: &Facelets, face: Face) -> RgbImage {
    const W: u32 = 640;
    const H: u32 = 480;
    const SIDE: u32 = 240;
    const GAP: u32 = 10;
    let cell = (SIDE - 4 * GAP) / 3;
    let (ox, oy) = ((W - SIDE) / 2, (H - SIDE) / 2);
    RgbImage::from_fn(W, H, |x, y| {
        let inside = (ox..ox + SIDE).contains(&x) && (oy..oy + SIDE).contains(&y);
        if !inside {
            return image::Rgb([48, 52, 58]); // background
        }
        let (lx, ly) = (x - ox, y - oy);
        let col = (lx.saturating_sub(GAP)) / (cell + GAP);
        let row = (ly.saturating_sub(GAP)) / (cell + GAP);
        let in_x = lx >= GAP + col * (cell + GAP) && lx < GAP + col * (cell + GAP) + cell;
        let in_y = ly >= GAP + row * (cell + GAP) && ly < GAP + row * (cell + GAP) + cell;
        if col < 3 && row < 3 && in_x && in_y {
            let facelet = face.index() * 9 + (row * 3 + col) as usize;
            image::Rgb(face_rgb(cube.get(facelet)))
        } else {
            image::Rgb([14, 14, 16]) // cube body
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vision::pipeline::read_face_grid;

    #[test]
    fn the_production_reader_reads_a_rendered_face() {
        let cube = Facelets::SOLVED.apply("R".parse().unwrap());
        for face in Face::ALL {
            let read = read_face_grid(&face_frame(&cube, face))
                .unwrap_or_else(|| panic!("no face read for {}", face.to_char()));
            for (k, rgb) in read.iter().enumerate() {
                let expected = face_rgb(cube.get(face.index() * 9 + k));
                let close = rgb.iter().zip(expected).all(|(a, b)| a.abs_diff(b) <= 24);
                assert!(
                    close,
                    "face {} cell {k}: read {rgb:?}, expected {expected:?}",
                    face.to_char()
                );
            }
        }
    }
}
