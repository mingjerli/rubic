//! Pure cube geometry for rendering and animation.
//!
//! `rubic-core` keeps its geometry table crate-private, so this module
//! re-derives the *same* documented facelet layout (URFDLB face order; each
//! face row-major as seen from outside with `U` up; coordinates x->Right,
//! y->Up, z->Front, each in `{-1,0,1}`). Correctness is pinned by unit tests
//! that assert the centers and layer membership, and cross-checked at runtime
//! because moves are still applied by the core engine.
//!
//! Called by `cube_render.rs` (to place cubies + stickers) and `animation.rs`
//! (to find a turning layer's cubies and the rotation axis/angle).

use rubic_core::{Amount, Face, Move};
use std::f32::consts::FRAC_PI_2;

/// Integer 3D vector with each component in `{-1,0,1}`.
pub type IVec = [i32; 3];

/// Position (cubie center) and outward normal of one facelet, matching the
/// documented `rubic-core` layout.
#[must_use]
fn facelet_pos_normal(face: Face, r: i32, c: i32) -> (IVec, IVec) {
    match face {
        Face::U => ([c - 1, 1, r - 1], [0, 1, 0]),
        Face::R => ([1, 1 - r, 1 - c], [1, 0, 0]),
        Face::F => ([c - 1, 1 - r, 1], [0, 0, 1]),
        Face::D => ([c - 1, -1, 1 - r], [0, -1, 0]),
        Face::L => ([-1, 1 - r, c - 1], [-1, 0, 0]),
        Face::B => ([1 - c, 1 - r, -1], [0, 0, -1]),
    }
}

/// Position and outward normal for facelet index `i` (`0..54`).
///
/// # Panics
/// Panics if `i >= 54`.
#[must_use]
pub fn facelet_geometry(i: usize) -> (IVec, IVec) {
    assert!(i < 54, "facelet index out of range");
    let face = Face::ALL[i / 9];
    let within = i % 9;
    let r = i32::try_from(within / 3).expect("0..3");
    let c = i32::try_from(within % 3).expect("0..3");
    facelet_pos_normal(face, r, c)
}

/// One renderable sticker: which facelet it is, which cubie it sits on, and
/// which way it faces.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct StickerSpec {
    /// Facelet index into [`rubic_core::Facelets`] (`0..54`).
    pub facelet: usize,
    /// Cubie center in `{-1,0,1}^3`.
    pub cubie: IVec,
    /// Outward normal direction of this sticker.
    pub normal: IVec,
}

/// All 54 sticker specs, in facelet-index order.
#[must_use]
pub fn all_stickers() -> Vec<StickerSpec> {
    (0..54)
        .map(|facelet| {
            let (cubie, normal) = facelet_geometry(facelet);
            StickerSpec {
                facelet,
                cubie,
                normal,
            }
        })
        .collect()
}

/// The 27 cubie centers in `{-1,0,1}^3`.
#[must_use]
pub fn all_cubies() -> Vec<IVec> {
    let mut out = Vec::with_capacity(27);
    for x in -1..=1 {
        for y in -1..=1 {
            for z in -1..=1 {
                out.push([x, y, z]);
            }
        }
    }
    out
}

/// `(axis index, value)` identifying the layer a face turn moves. Matches the
/// core engine's `layer` function.
#[must_use]
pub fn face_layer(face: Face) -> (usize, i32) {
    match face {
        Face::U => (1, 1),
        Face::D => (1, -1),
        Face::R => (0, 1),
        Face::L => (0, -1),
        Face::F => (2, 1),
        Face::B => (2, -1),
    }
}

/// Whether the cubie at `pos` lies in the layer turned by `face`.
#[must_use]
pub fn cubie_in_layer(pos: IVec, face: Face) -> bool {
    let (axis, value) = face_layer(face);
    pos[axis] == value
}

/// Outward normal of `face` as a unit float vector: the axis a visual turn
/// rotates about.
#[must_use]
pub fn face_axis(face: Face) -> [f32; 3] {
    match face {
        Face::U => [0.0, 1.0, 0.0],
        Face::D => [0.0, -1.0, 0.0],
        Face::R => [1.0, 0.0, 0.0],
        Face::L => [-1.0, 0.0, 0.0],
        Face::F => [0.0, 0.0, 1.0],
        Face::B => [0.0, 0.0, -1.0],
    }
}

/// Total signed rotation, in radians, for a move about its face's outward
/// normal.
///
/// A clockwise quarter turn (viewed from outside the face) is `-PI/2` about the
/// outward normal in this right-handed frame; this is exactly the rotation the
/// core engine applies to facelet positions, so the animation lands on the new
/// state.
#[must_use]
pub fn turn_angle(amount: Amount) -> f32 {
    match amount {
        Amount::Cw => -FRAC_PI_2,
        Amount::Ccw => FRAC_PI_2,
        Amount::Double => -std::f32::consts::PI,
    }
}

/// Convenience: the axis and total angle for animating `m`.
#[must_use]
pub fn move_rotation(m: Move) -> ([f32; 3], f32) {
    (face_axis(m.face), turn_angle(m.amount))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn centers_match_core_layout() {
        for (f, &face) in Face::ALL.iter().enumerate() {
            let (pos, nrm) = facelet_geometry(f * 9 + 4);
            let expected = face_axis(face).map(|v| v as i32);
            assert_eq!(nrm, expected, "center normal for {}", face.to_char());
            assert_eq!(pos, expected, "center pos for {}", face.to_char());
        }
    }

    #[test]
    fn u_center_is_up() {
        assert_eq!(facelet_geometry(4), ([0, 1, 0], [0, 1, 0]));
    }

    #[test]
    fn all_positions_are_ternary() {
        for i in 0..54 {
            let (pos, nrm) = facelet_geometry(i);
            for v in pos {
                assert!((-1..=1).contains(&v));
            }
            let mag: i32 = nrm.iter().map(|v| v.abs()).sum();
            assert_eq!(mag, 1, "normal must be a unit axis vector");
        }
    }

    #[test]
    fn each_facelet_has_unique_pos_normal() {
        let specs = all_stickers();
        for a in 0..specs.len() {
            for b in (a + 1)..specs.len() {
                assert!(
                    !(specs[a].cubie == specs[b].cubie && specs[a].normal == specs[b].normal),
                    "facelets {a} and {b} collide"
                );
            }
        }
    }

    #[test]
    fn nine_cubies_per_layer() {
        for face in Face::ALL {
            let count = all_cubies()
                .into_iter()
                .filter(|&p| cubie_in_layer(p, face))
                .count();
            assert_eq!(count, 9, "{}", face.to_char());
        }
    }

    #[test]
    fn there_are_27_cubies() {
        assert_eq!(all_cubies().len(), 27);
    }

    #[test]
    fn turn_angles_have_expected_signs() {
        assert!(turn_angle(Amount::Cw) < 0.0);
        assert!(turn_angle(Amount::Ccw) > 0.0);
        assert!((turn_angle(Amount::Double).abs() - std::f32::consts::PI).abs() < 1e-6);
        assert!((2.0 * turn_angle(Amount::Cw) - turn_angle(Amount::Double)).abs() < 1e-6);
    }

    #[test]
    fn interior_cubie_never_moves() {
        for face in Face::ALL {
            assert!(!cubie_in_layer([0, 0, 0], face));
        }
    }
}
