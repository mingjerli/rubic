//! Fast internal cubie model used by the beginner solver.
//!
//! Slots are numbered so that in the solved cube piece `i` sits in slot `i`.
//! Edge slots (0..12): UF UR UB UL DF DR DB DL FR FL BR BL.
//! Corner slots (0..8): URF UFL ULB UBR DFR DLF DBL DRB.
//!
//! Move permutation and orientation tables were derived empirically from the
//! (independently tested) facelet engine, so this model matches it exactly.

use crate::color::Face;
use crate::facelets::Facelets;
use crate::moves::{Amount, Move};

/// Edge slot indices.
pub mod e {
    pub const UF: usize = 0;
    pub const UR: usize = 1;
    pub const UB: usize = 2;
    pub const UL: usize = 3;
    pub const DF: usize = 4;
    pub const DR: usize = 5;
    pub const DB: usize = 6;
    pub const DL: usize = 7;
    pub const FR: usize = 8;
    pub const FL: usize = 9;
    pub const BR: usize = 10;
    pub const BL: usize = 11;
}

/// Corner slot indices.
pub mod c {
    pub const URF: usize = 0;
    pub const UFL: usize = 1;
    pub const ULB: usize = 2;
    pub const UBR: usize = 3;
    pub const DFR: usize = 4;
    pub const DLF: usize = 5;
    pub const DBL: usize = 6;
    pub const DRB: usize = 7;
}

/// Home colors of each edge piece `[primary, secondary]`.
const EDGE_COLORS: [[Face; 2]; 12] = [
    [Face::U, Face::F],
    [Face::U, Face::R],
    [Face::U, Face::B],
    [Face::U, Face::L],
    [Face::D, Face::F],
    [Face::D, Face::R],
    [Face::D, Face::B],
    [Face::D, Face::L],
    [Face::F, Face::R],
    [Face::F, Face::L],
    [Face::B, Face::R],
    [Face::B, Face::L],
];

/// Facelet indices of each edge slot `[primary, secondary]`.
const EDGE_FACELETS: [[usize; 2]; 12] = [
    [7, 19],
    [5, 10],
    [1, 46],
    [3, 37],
    [28, 25],
    [32, 16],
    [34, 52],
    [30, 43],
    [23, 12],
    [21, 41],
    [48, 14],
    [50, 39],
];

/// Home colors of each corner piece `[up/down, side, side]`.
const CORNER_COLORS: [[Face; 3]; 8] = [
    [Face::U, Face::R, Face::F],
    [Face::U, Face::F, Face::L],
    [Face::U, Face::L, Face::B],
    [Face::U, Face::B, Face::R],
    [Face::D, Face::F, Face::R],
    [Face::D, Face::L, Face::F],
    [Face::D, Face::B, Face::L],
    [Face::D, Face::R, Face::B],
];

/// Facelet indices of each corner slot `[up/down, side, side]`.
const CORNER_FACELETS: [[usize; 3]; 8] = [
    [8, 9, 20],
    [6, 18, 38],
    [0, 36, 47],
    [2, 45, 11],
    [29, 26, 15],
    [27, 44, 24],
    [33, 53, 42],
    [35, 17, 51],
];

/// One clockwise quarter-turn as edge/corner 4-cycles with orientation change.
struct BaseMove {
    edge_cycle: [usize; 4],
    edge_flip: u8,
    corner_cycle: [usize; 4],
    corner_delta: [u8; 4],
}

/// The six clockwise base moves, indexed by [`Face::index`].
const BASE: [BaseMove; 6] = [
    // U
    BaseMove {
        edge_cycle: [e::UF, e::UL, e::UB, e::UR],
        edge_flip: 0,
        corner_cycle: [c::URF, c::UFL, c::ULB, c::UBR],
        corner_delta: [0, 0, 0, 0],
    },
    // R
    BaseMove {
        edge_cycle: [e::UR, e::BR, e::DR, e::FR],
        edge_flip: 0,
        corner_cycle: [c::URF, c::UBR, c::DRB, c::DFR],
        corner_delta: [1, 2, 1, 2],
    },
    // F
    BaseMove {
        edge_cycle: [e::UF, e::FR, e::DF, e::FL],
        edge_flip: 1,
        corner_cycle: [c::URF, c::DFR, c::DLF, c::UFL],
        corner_delta: [2, 1, 2, 1],
    },
    // D
    BaseMove {
        edge_cycle: [e::DF, e::DR, e::DB, e::DL],
        edge_flip: 0,
        corner_cycle: [c::DFR, c::DRB, c::DBL, c::DLF],
        corner_delta: [0, 0, 0, 0],
    },
    // L
    BaseMove {
        edge_cycle: [e::UL, e::FL, e::DL, e::BL],
        edge_flip: 0,
        corner_cycle: [c::UFL, c::DLF, c::DBL, c::ULB],
        corner_delta: [2, 1, 2, 1],
    },
    // B
    BaseMove {
        edge_cycle: [e::UB, e::BL, e::DB, e::BR],
        edge_flip: 1,
        corner_cycle: [c::ULB, c::DBL, c::DRB, c::UBR],
        corner_delta: [2, 1, 2, 1],
    },
];

/// The cubie model: piece indices and orientations by slot.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cube {
    /// `cp[s]`: corner piece in slot `s`.
    pub cp: [u8; 8],
    /// `co[s]`: corner twist (`0..3`) in slot `s`.
    pub co: [u8; 8],
    /// `ep[s]`: edge piece in slot `s`.
    pub ep: [u8; 12],
    /// `eo[s]`: edge flip (`0..2`) in slot `s`.
    pub eo: [u8; 12],
}

impl Cube {
    /// The solved cube.
    pub const SOLVED: Cube = Cube {
        cp: [0, 1, 2, 3, 4, 5, 6, 7],
        co: [0; 8],
        ep: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        eo: [0; 12],
    };

    /// Build the model by reading the 54 stickers of `f`.
    #[must_use]
    pub fn from_facelets(f: &Facelets) -> Cube {
        let mut ep = [0u8; 12];
        let mut eo = [0u8; 12];
        for slot in 0..12 {
            let a = f.get(EDGE_FACELETS[slot][0]);
            let b = f.get(EDGE_FACELETS[slot][1]);
            let piece = find_edge_piece(a, b);
            ep[slot] = u8::try_from(piece).expect("edge piece < 12");
            eo[slot] = u8::from(a != EDGE_COLORS[piece][0]);
        }
        let mut cp = [0u8; 8];
        let mut co = [0u8; 8];
        for slot in 0..8 {
            let a = f.get(CORNER_FACELETS[slot][0]);
            let b = f.get(CORNER_FACELETS[slot][1]);
            let d = f.get(CORNER_FACELETS[slot][2]);
            let piece = find_corner_piece(a, b, d);
            cp[slot] = u8::try_from(piece).expect("corner piece < 8");
            let ud = CORNER_COLORS[piece][0];
            co[slot] = if a == ud {
                0
            } else if b == ud {
                1
            } else {
                2
            };
        }
        Cube { cp, co, ep, eo }
    }

    /// Whether this is the solved cube.
    #[must_use]
    pub fn is_solved(&self) -> bool {
        *self == Cube::SOLVED
    }

    /// Apply a move, returning a new cube (never mutates in place).
    #[must_use]
    pub fn apply(&self, m: Move) -> Cube {
        let quarters = match m.amount {
            Amount::Cw => 1,
            Amount::Double => 2,
            Amount::Ccw => 3,
        };
        let base = &BASE[m.face.index()];
        let mut cur = *self;
        for _ in 0..quarters {
            cur = cur.turn(base);
        }
        cur
    }

    /// One clockwise quarter turn described by `base`.
    fn turn(&self, base: &BaseMove) -> Cube {
        let mut next = *self;
        let ec = base.edge_cycle;
        for i in 0..4 {
            let from = ec[i];
            let to = ec[(i + 1) % 4];
            next.ep[to] = self.ep[from];
            next.eo[to] = self.eo[from] ^ base.edge_flip;
        }
        let cc = base.corner_cycle;
        for i in 0..4 {
            let from = cc[i];
            let to = cc[(i + 1) % 4];
            next.cp[to] = self.cp[from];
            next.co[to] = (self.co[from] + base.corner_delta[i]) % 3;
        }
        next
    }

    /// Slot currently holding edge piece `piece`.
    #[must_use]
    pub fn edge_slot(&self, piece: usize) -> usize {
        (0..12)
            .find(|&s| self.ep[s] as usize == piece)
            .expect("edge piece present")
    }

    /// Slot currently holding corner piece `piece`.
    #[must_use]
    pub fn corner_slot(&self, piece: usize) -> usize {
        (0..8)
            .find(|&s| self.cp[s] as usize == piece)
            .expect("corner piece present")
    }
}

fn find_edge_piece(a: Face, b: Face) -> usize {
    (0..12)
        .find(|&p| {
            let h = EDGE_COLORS[p];
            (h[0] == a && h[1] == b) || (h[0] == b && h[1] == a)
        })
        .expect("valid edge colors")
}

fn find_corner_piece(a: Face, b: Face, d: Face) -> usize {
    let mut key = [a.index(), b.index(), d.index()];
    key.sort_unstable();
    (0..8)
        .find(|&p| {
            let h = CORNER_COLORS[p];
            let mut k2 = [h[0].index(), h[1].index(), h[2].index()];
            k2.sort_unstable();
            k2 == key
        })
        .expect("valid corner colors")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::moves::Sequence;

    #[test]
    fn model_matches_engine() {
        // Deterministic xorshift; compare Cube apply vs engine apply.
        let mut st: u64 = 0x1234_5678_9abc_def0;
        let faces = ["U", "R", "F", "D", "L", "B"];
        let amts = ["", "'", "2"];
        for _ in 0..2000 {
            let mut f = Facelets::SOLVED;
            let mut cube = Cube::SOLVED;
            let len = 25;
            for _ in 0..len {
                st ^= st << 13;
                st ^= st >> 7;
                st ^= st << 17;
                let fi = (st % 6) as usize;
                let ai = ((st >> 8) % 3) as usize;
                let tok = format!("{}{}", faces[fi], amts[ai]);
                let seq: Sequence = tok.parse().unwrap();
                f = f.apply_seq(&seq);
                cube = cube.apply(seq.0[0]);
            }
            assert_eq!(cube, Cube::from_facelets(&f), "mismatch");
        }
    }
}
