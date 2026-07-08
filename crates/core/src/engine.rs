//! Move application: turning faces permutes the 54 facelets.
//!
//! Rather than hand-transcribing permutation tables, we derive them from cube
//! geometry: every facelet has a 3D position (each coordinate in `{-1,0,1}`)
//! and an outward normal, and a face turn rotates the affected layer by 90°.
//! This is correct by construction and validated by the move-algebra tests
//! (quarter-turn order, inverses, and the order of `R U R' U'` and `R U`).

use crate::color::Face;
use crate::facelets::Facelets;
use crate::moves::{Amount, Move, Sequence};
use std::sync::OnceLock;

type Vec3 = [i32; 3];

/// Position and outward normal of facelet `(face, row, col)` (row/col in `0..3`).
///
/// Coordinates use x→Right, y→Up, z→Front. Each face is numbered row-major as
/// seen from outside with `U` up (URFDLB convention).
fn facelet_geometry(face: Face, r: i32, c: i32) -> (Vec3, Vec3) {
    match face {
        Face::U => ([c - 1, 1, r - 1], [0, 1, 0]),
        Face::R => ([1, 1 - r, 1 - c], [1, 0, 0]),
        Face::F => ([c - 1, 1 - r, 1], [0, 0, 1]),
        Face::D => ([c - 1, -1, 1 - r], [0, -1, 0]),
        Face::L => ([-1, 1 - r, c - 1], [-1, 0, 0]),
        Face::B => ([1 - c, 1 - r, -1], [0, 0, -1]),
    }
}

/// Geometry table for all 54 facelets: `(position, outward normal)` indexed by
/// facelet number. Used by the validation module to group facelets into cubie
/// slots. Coordinates use x→Right, y→Up, z→Front, each in `{-1,0,1}`.
pub(crate) fn facelet_geometry_table() -> &'static [([i32; 3], [i32; 3]); 54] {
    geometry()
}

/// Geometry table for all 54 facelets, indexed by facelet number.
fn geometry() -> &'static [(Vec3, Vec3); 54] {
    static G: OnceLock<[(Vec3, Vec3); 54]> = OnceLock::new();
    G.get_or_init(|| {
        let mut g = [([0, 0, 0], [0, 0, 0]); 54];
        for (fi, &face) in Face::ALL.iter().enumerate() {
            for r in 0..3i32 {
                for c in 0..3i32 {
                    let idx = fi * 9 + usize::try_from(r * 3 + c).expect("0..9");
                    g[idx] = facelet_geometry(face, r, c);
                }
            }
        }
        g
    })
}

/// Rotate a vector 90° for a clockwise turn of `face` (viewed from outside).
fn rotate_cw(face: Face, v: Vec3) -> Vec3 {
    let [x, y, z] = v;
    match face {
        Face::U => [-z, y, x],
        Face::D => [z, y, -x],
        Face::R => [x, z, -y],
        Face::L => [x, -z, y],
        Face::F => [y, -x, z],
        Face::B => [-y, x, z],
    }
}

/// `(axis index, value)` identifying the turning layer for `face`.
fn layer(face: Face) -> (usize, i32) {
    match face {
        Face::U => (1, 1),
        Face::D => (1, -1),
        Face::R => (0, 1),
        Face::L => (0, -1),
        Face::F => (2, 1),
        Face::B => (2, -1),
    }
}

/// Clockwise quarter-turn permutation: the sticker at index `i` moves to
/// index `perm[i]`. Facelets outside the turning layer map to themselves.
fn base_perm(face: Face) -> [usize; 54] {
    let g = geometry();
    let (ax, val) = layer(face);
    let mut perm = [0usize; 54];
    for (i, slot) in perm.iter_mut().enumerate() {
        let (pos, nrm) = g[i];
        *slot = if pos[ax] == val {
            let np = rotate_cw(face, pos);
            let nn = rotate_cw(face, nrm);
            (0..54)
                .find(|&k| g[k].0 == np && g[k].1 == nn)
                .expect("rotated facelet exists in a closed cube")
        } else {
            i
        };
    }
    perm
}

/// Cached clockwise quarter-turn permutations for all six faces (URFDLB order).
fn perms() -> &'static [[usize; 54]; 6] {
    static P: OnceLock<[[usize; 54]; 6]> = OnceLock::new();
    P.get_or_init(|| {
        let mut p = [[0usize; 54]; 6];
        for (slot, &face) in p.iter_mut().zip(Face::ALL.iter()) {
            *slot = base_perm(face);
        }
        p
    })
}

impl Facelets {
    /// Apply a single move, returning a new cube (never mutates in place).
    #[must_use]
    pub fn apply(&self, m: Move) -> Facelets {
        let quarters = match m.amount {
            Amount::Cw => 1,
            Amount::Double => 2,
            Amount::Ccw => 3,
        };
        let perm = &perms()[m.face.index()];
        let mut cur = self.0;
        for _ in 0..quarters {
            let mut next = cur;
            for (i, &face) in cur.iter().enumerate() {
                next[perm[i]] = face;
            }
            cur = next;
        }
        Facelets(cur)
    }

    /// Apply a whole sequence of moves in order.
    #[must_use]
    pub fn apply_seq(&self, seq: &Sequence) -> Facelets {
        let mut cube = *self;
        for &m in &seq.0 {
            cube = cube.apply(m);
        }
        cube
    }
}
