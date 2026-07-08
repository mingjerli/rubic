//! The facelet model: the 54 stickers of a cube in URFDLB order.
//!
//! Face order is `U, R, F, D, L, B`; within each face the nine stickers are
//! numbered `0..9` row-major as seen with that face toward the viewer and `U`
//! up. This is the human/IO boundary; moves are applied here (see
//! [`crate::engine`]).

use crate::color::Face;
use std::fmt;
use std::str::FromStr;

/// A full cube as 54 sticker labels in URFDLB order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Facelets(pub(crate) [Face; 54]);

const fn solved_array() -> [Face; 54] {
    let mut a = [Face::U; 54];
    let mut i = 0;
    while i < 54 {
        a[i] = Face::ALL[i / 9];
        i += 1;
    }
    a
}

impl Facelets {
    /// The solved cube: every sticker on face `X` is labelled `X`.
    pub const SOLVED: Facelets = Facelets(solved_array());

    /// The sticker label at facelet index `i` (`0..54`).
    ///
    /// # Panics
    /// Panics if `i >= 54`.
    #[must_use]
    pub fn get(&self, i: usize) -> Face {
        self.0[i]
    }
}

impl FromStr for Facelets {
    type Err = ParseFaceletsError;

    /// Parse a 54-character string of `U R F D L B` labels in URFDLB order.
    ///
    /// # Errors
    /// Returns [`ParseFaceletsError`] if the length is not 54 or a character is
    /// not a valid face label.
    fn from_str(s: &str) -> Result<Facelets, ParseFaceletsError> {
        let chars: Vec<char> = s.chars().collect();
        if chars.len() != 54 {
            return Err(ParseFaceletsError::WrongLength(chars.len()));
        }
        let mut arr = [Face::U; 54];
        for (slot, &ch) in arr.iter_mut().zip(chars.iter()) {
            *slot = Face::from_char(ch).ok_or(ParseFaceletsError::InvalidChar(ch))?;
        }
        Ok(Facelets(arr))
    }
}

impl fmt::Display for Facelets {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut s = String::with_capacity(54);
        for face in self.0 {
            s.push(face.to_char());
        }
        f.write_str(&s)
    }
}

/// Error returned when parsing a [`Facelets`] string fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseFaceletsError {
    /// The input did not contain exactly 54 characters.
    WrongLength(usize),
    /// A character was not one of `U R F D L B`.
    InvalidChar(char),
}

impl fmt::Display for ParseFaceletsError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseFaceletsError::WrongLength(n) => {
                write!(f, "expected 54 facelet characters, got {n}")
            }
            ParseFaceletsError::InvalidChar(c) => write!(f, "invalid facelet character {c:?}"),
        }
    }
}

impl std::error::Error for ParseFaceletsError {}
