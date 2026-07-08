//! Partial input completion: deciding when a partially-entered cube is
//! impossible, ambiguous, or fully determined.
//!
//! Implements the "minimal input" requirement: after each sticker the user
//! sets, we ask how many solvable cubes are consistent with what's known so
//! far — zero means impossible, one means we have enough, more than one means
//! we still need input. Centers are always known (they define the scheme).

use crate::color::Face;
use crate::facelets::Facelets;
use crate::state::{CubeError, CubeState, Tables, perm_is_odd, slot_tables};

/// A cube with some stickers known and others still unknown.
///
/// The six centers are always known (fixed to their face label); only the 48
/// non-center stickers are user input.
#[derive(Debug, Clone)]
pub struct PartialFacelets([Option<Face>; 54]);

impl Default for PartialFacelets {
    fn default() -> Self {
        Self::new()
    }
}

impl PartialFacelets {
    /// A cube with only the centers known.
    #[must_use]
    pub fn new() -> Self {
        let mut a = [None; 54];
        for (f, &face) in Face::ALL.iter().enumerate() {
            a[f * 9 + 4] = Some(face);
        }
        PartialFacelets(a)
    }

    /// Treat a full cube as fully-known partial input.
    #[must_use]
    pub fn from_facelets(f: &Facelets) -> Self {
        let mut a = [None; 54];
        for (i, slot) in a.iter_mut().enumerate() {
            *slot = Some(f.get(i));
        }
        PartialFacelets(a)
    }

    /// Set the sticker at `idx` to `color`, returning a new value.
    ///
    /// # Panics
    /// Panics if `idx >= 54`.
    #[must_use]
    pub fn set(&self, idx: usize, color: Face) -> Self {
        let mut a = self.0;
        a[idx] = Some(color);
        PartialFacelets(a)
    }

    /// Clear the sticker at `idx` (centers cannot be cleared), returning a new
    /// value.
    ///
    /// # Panics
    /// Panics if `idx >= 54`.
    #[must_use]
    pub fn clear(&self, idx: usize) -> Self {
        let mut a = self.0;
        if idx % 9 != 4 {
            a[idx] = None;
        }
        PartialFacelets(a)
    }

    /// The color known at `idx`, if any.
    ///
    /// # Panics
    /// Panics if `idx >= 54`.
    #[must_use]
    pub fn get(&self, idx: usize) -> Option<Face> {
        self.0[idx]
    }

    /// Number of non-center stickers the user has entered (`0..=48`).
    #[must_use]
    pub fn known_count(&self) -> u8 {
        let n = (0..54)
            .filter(|&i| i % 9 != 4 && self.0[i].is_some())
            .count();
        u8::try_from(n).unwrap_or(48)
    }

    /// Decide whether the current input is impossible, still ambiguous, or
    /// enough to determine the whole cube.
    ///
    /// Counts how many solvable cubes are consistent with the known stickers,
    /// stopping at two: zero → [`Completion::Impossible`], one →
    /// [`Completion::Unique`], two or more → [`Completion::NeedMore`].
    #[must_use]
    pub fn analyze(&self) -> Completion {
        let known = self.known_count();

        // Fully-known input: defer to the exact validator for precise errors.
        if self.0.iter().all(Option::is_some) {
            let mut arr = [Face::U; 54];
            for (slot, cell) in arr.iter_mut().zip(self.0.iter()) {
                // Branch guarantees every cell is `Some`; default is unreachable.
                *slot = cell.unwrap_or(Face::U);
            }
            return match Facelets(arr).validate() {
                Ok(state) => Completion::Unique(state),
                Err(e) => Completion::Impossible(e),
            };
        }

        if let Some(e) = self.cheap_violation() {
            return Completion::Impossible(e);
        }

        let solutions = self.completions(2);
        match solutions.len() {
            0 => Completion::Impossible(CubeError::Contradiction),
            1 => Completion::Unique(solutions[0]),
            _ => Completion::NeedMore { known },
        }
    }

    /// A cheap necessary check that catches the common early impossibility:
    /// a color used more than nine times.
    fn cheap_violation(&self) -> Option<CubeError> {
        for face in Face::ALL {
            let n = self.0.iter().filter(|c| **c == Some(face)).count();
            if n > 9 {
                return Some(CubeError::WrongColorCounts(face));
            }
        }
        None
    }

    /// Find up to `limit` distinct solvable cubes consistent with the knowns.
    fn completions(&self, limit: usize) -> Vec<CubeState> {
        let t = slot_tables();
        let corner_domain: Vec<Vec<(u8, u8)>> =
            (0..8).map(|s| corner_candidates(&self.0, t, s)).collect();
        let edge_domain: Vec<Vec<(u8, u8)>> =
            (0..12).map(|s| edge_candidates(&self.0, t, s)).collect();

        // Most-constrained slots first, so contradictions prune early.
        let mut corner_order: Vec<usize> = (0..8).collect();
        corner_order.sort_by_key(|&s| corner_domain[s].len());
        let mut edge_order: Vec<usize> = (0..12).collect();
        edge_order.sort_by_key(|&s| edge_domain[s].len());

        let mut search = Search {
            corner_domain,
            edge_domain,
            corner_order,
            edge_order,
            limit,
            results: Vec::new(),
            cp: [0; 8],
            co: [0; 8],
            ep: [0; 12],
            eo: [0; 12],
            used_c: [false; 8],
            used_e: [false; 12],
        };
        search.corners(0);
        search.results
    }
}

/// Candidate `(piece, twist)` pairs for a corner slot given the known stickers.
fn corner_candidates(known: &[Option<Face>; 54], tables: &Tables, slot: usize) -> Vec<(u8, u8)> {
    let facelets = tables.corner_facelets[slot];
    let here = [known[facelets[0]], known[facelets[1]], known[facelets[2]]];
    let mut out = Vec::new();
    for piece in 0..8u8 {
        let home = tables.corner_home[usize::from(piece)];
        for twist in 0..3u8 {
            let matches = (0..3).all(|pos| match here[pos] {
                None => true,
                Some(color) => home[(pos + 3 - usize::from(twist)) % 3] == color,
            });
            if matches {
                out.push((piece, twist));
            }
        }
    }
    out
}

/// Candidate `(piece, flip)` pairs for an edge slot given the known stickers.
fn edge_candidates(known: &[Option<Face>; 54], tables: &Tables, slot: usize) -> Vec<(u8, u8)> {
    let facelets = tables.edge_facelets[slot];
    let here = [known[facelets[0]], known[facelets[1]]];
    let mut out = Vec::new();
    for piece in 0..12u8 {
        let home = tables.edge_home[usize::from(piece)];
        for flip in 0..2u8 {
            let (first, second) = if flip == 0 {
                (home[0], home[1])
            } else {
                (home[1], home[0])
            };
            let matches = here[0].is_none_or(|c| c == first) && here[1].is_none_or(|c| c == second);
            if matches {
                out.push((piece, flip));
            }
        }
    }
    out
}

/// Backtracking search over corner then edge slots, respecting the known
/// stickers, piece uniqueness, and the three solvability constraints.
struct Search {
    corner_domain: Vec<Vec<(u8, u8)>>,
    edge_domain: Vec<Vec<(u8, u8)>>,
    corner_order: Vec<usize>,
    edge_order: Vec<usize>,
    limit: usize,
    results: Vec<CubeState>,
    cp: [u8; 8],
    co: [u8; 8],
    ep: [u8; 12],
    eo: [u8; 12],
    used_c: [bool; 8],
    used_e: [bool; 12],
}

impl Search {
    fn corners(&mut self, depth: usize) {
        if self.results.len() >= self.limit {
            return;
        }
        if depth == 8 {
            let co_sum: u32 = self.co.iter().map(|&x| u32::from(x)).sum();
            if co_sum % 3 != 0 {
                return;
            }
            let corner_parity = perm_is_odd(&self.cp);
            self.edges(0, corner_parity);
            return;
        }
        let slot = self.corner_order[depth];
        for (p, tw) in self.corner_domain[slot].clone() {
            if self.used_c[usize::from(p)] {
                continue;
            }
            self.used_c[usize::from(p)] = true;
            self.cp[slot] = p;
            self.co[slot] = tw;
            self.corners(depth + 1);
            self.used_c[usize::from(p)] = false;
            if self.results.len() >= self.limit {
                return;
            }
        }
    }

    fn edges(&mut self, depth: usize, corner_parity: bool) {
        if self.results.len() >= self.limit {
            return;
        }
        if depth == 12 {
            let eo_sum: u32 = self.eo.iter().map(|&x| u32::from(x)).sum();
            if eo_sum % 2 != 0 || perm_is_odd(&self.ep) != corner_parity {
                return;
            }
            self.results
                .push(CubeState::new_unchecked(self.cp, self.co, self.ep, self.eo));
            return;
        }
        let slot = self.edge_order[depth];
        for (p, fl) in self.edge_domain[slot].clone() {
            if self.used_e[usize::from(p)] {
                continue;
            }
            self.used_e[usize::from(p)] = true;
            self.ep[slot] = p;
            self.eo[slot] = fl;
            self.edges(depth + 1, corner_parity);
            self.used_e[usize::from(p)] = false;
            if self.results.len() >= self.limit {
                return;
            }
        }
    }
}

/// The result of analyzing partial input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Completion {
    /// No solvable cube is consistent with the known stickers.
    Impossible(CubeError),
    /// More than one solvable cube is still consistent; keep entering stickers.
    NeedMore {
        /// How many non-center stickers are known so far.
        known: u8,
    },
    /// Exactly one solvable cube is consistent: the input is now enough.
    Unique(CubeState),
}
