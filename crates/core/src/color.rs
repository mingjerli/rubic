//! Faces of the cube, which double as sticker color labels.
//!
//! On a 3x3 the centers are fixed, so each face's center color defines the
//! scheme. We label a sticker by the face whose center shares its color, using
//! the standard `U R F D L B` letters. In the solved state every sticker on
//! face `X` is labelled `X`.

/// A face of the cube, also used as a sticker color label.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Face {
    /// Up.
    U,
    /// Right.
    R,
    /// Front.
    F,
    /// Down.
    D,
    /// Left.
    L,
    /// Back.
    B,
}

impl Face {
    /// All six faces in the canonical URFDLB order used for facelet indexing.
    pub const ALL: [Face; 6] = [Face::U, Face::R, Face::F, Face::D, Face::L, Face::B];

    /// Position of this face in the canonical URFDLB order (`0..6`).
    #[must_use]
    pub const fn index(self) -> usize {
        match self {
            Face::U => 0,
            Face::R => 1,
            Face::F => 2,
            Face::D => 3,
            Face::L => 4,
            Face::B => 5,
        }
    }

    /// The single-letter label for this face.
    #[must_use]
    pub const fn to_char(self) -> char {
        match self {
            Face::U => 'U',
            Face::R => 'R',
            Face::F => 'F',
            Face::D => 'D',
            Face::L => 'L',
            Face::B => 'B',
        }
    }

    /// Parse a face from its single-letter label, if valid.
    #[must_use]
    pub const fn from_char(ch: char) -> Option<Face> {
        match ch {
            'U' => Some(Face::U),
            'R' => Some(Face::R),
            'F' => Some(Face::F),
            'D' => Some(Face::D),
            'L' => Some(Face::L),
            'B' => Some(Face::B),
            _ => None,
        }
    }
}
