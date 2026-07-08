//! Cube moves and sequences in standard notation (`U R' F2` …).

use crate::color::Face;
use std::fmt;
use std::str::FromStr;

/// How far a face is turned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Amount {
    /// Quarter turn clockwise (as viewed from outside that face).
    Cw,
    /// Quarter turn counter-clockwise (the `'` suffix).
    Ccw,
    /// Half turn (the `2` suffix).
    Double,
}

/// A single face turn.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Move {
    /// The face being turned.
    pub face: Face,
    /// How far it turns.
    pub amount: Amount,
}

impl Move {
    /// The move that undoes this one.
    #[must_use]
    pub const fn inverse(self) -> Move {
        let amount = match self.amount {
            Amount::Cw => Amount::Ccw,
            Amount::Ccw => Amount::Cw,
            Amount::Double => Amount::Double,
        };
        Move {
            face: self.face,
            amount,
        }
    }
}

impl fmt::Display for Move {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let suffix = match self.amount {
            Amount::Cw => "",
            Amount::Ccw => "'",
            Amount::Double => "2",
        };
        write!(f, "{}{suffix}", self.face.to_char())
    }
}

impl FromStr for Move {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let invalid = || ParseMoveError::InvalidToken(s.to_string());
        let mut chars = s.chars();
        let face = chars.next().and_then(Face::from_char).ok_or_else(invalid)?;
        let amount = match chars.next() {
            None => Amount::Cw,
            Some('\'') => Amount::Ccw,
            Some('2') => Amount::Double,
            Some(_) => return Err(invalid()),
        };
        if chars.next().is_some() {
            return Err(invalid());
        }
        Ok(Move { face, amount })
    }
}

/// An ordered list of moves.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Sequence(pub Vec<Move>);

impl Sequence {
    /// The sequence that undoes this one (moves reversed and inverted).
    #[must_use]
    pub fn inverse(&self) -> Sequence {
        Sequence(self.0.iter().rev().map(|m| m.inverse()).collect())
    }
}

impl fmt::Display for Sequence {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let tokens: Vec<String> = self.0.iter().map(ToString::to_string).collect();
        f.write_str(&tokens.join(" "))
    }
}

impl FromStr for Sequence {
    type Err = ParseMoveError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let moves = s
            .split_whitespace()
            .map(str::parse)
            .collect::<Result<Vec<Move>, _>>()?;
        Ok(Sequence(moves))
    }
}

/// Error returned when parsing a [`Move`] or [`Sequence`] fails.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseMoveError {
    /// The token was not a valid move (e.g. bad face letter or suffix).
    InvalidToken(String),
}

impl fmt::Display for ParseMoveError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseMoveError::InvalidToken(t) => write!(f, "invalid move token {t:?}"),
        }
    }
}

impl std::error::Error for ParseMoveError {}
