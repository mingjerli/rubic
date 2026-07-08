//! The cubie model (`CubeState`) and full-cube solvability checking.
//!
//! The cubie model describes the cube as a permutation and orientation of its
//! 8 corner and 12 edge pieces, which is the natural view for validity checks
//! and solvers. It is derived from [`Facelets`] by grouping the 54 stickers
//! into cubie slots using the engine's geometry table, so no piece/facelet
//! tables are hand-transcribed.

use crate::color::Face;
use crate::engine::facelet_geometry_table;
use crate::facelets::Facelets;
use std::sync::OnceLock;

/// A cube as the permutation + orientation of its pieces.
///
/// - `cp[s]` / `co[s]`: which corner piece occupies slot `s`, and its twist
///   (`0..3`).
/// - `ep[s]` / `eo[s]`: which edge piece occupies slot `s`, and its flip
///   (`0..2`).
///
/// Slot and piece indices are assigned deterministically from cube geometry;
/// the solved cube has identity permutations and zero orientations.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CubeState {
    cp: [u8; 8],
    co: [u8; 8],
    ep: [u8; 12],
    eo: [u8; 12],
}

impl CubeState {
    /// The solved cube.
    pub const SOLVED: CubeState = CubeState {
        cp: [0, 1, 2, 3, 4, 5, 6, 7],
        co: [0; 8],
        ep: [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11],
        eo: [0; 12],
    };

    /// Build a state from raw arrays without checking solvability.
    ///
    /// Useful for solvers and for constructing test fixtures (including
    /// deliberately unsolvable ones). Callers that need a guaranteed-solvable
    /// state should go through [`Facelets::validate`].
    #[must_use]
    pub fn new_unchecked(cp: [u8; 8], co: [u8; 8], ep: [u8; 12], eo: [u8; 12]) -> CubeState {
        CubeState { cp, co, ep, eo }
    }

    /// Whether this is the solved cube.
    #[must_use]
    pub fn is_solved(&self) -> bool {
        *self == CubeState::SOLVED
    }

    /// Check the three reachability constraints (corner twist sum, edge flip
    /// sum, permutation parity).
    ///
    /// # Errors
    /// Returns the specific [`CubeError`] for the first violated constraint.
    pub fn solvability(&self) -> Result<(), CubeError> {
        let co_sum: u32 = self.co.iter().map(|&x| u32::from(x)).sum();
        if co_sum % 3 != 0 {
            return Err(CubeError::CornerTwist);
        }
        let eo_sum: u32 = self.eo.iter().map(|&x| u32::from(x)).sum();
        if eo_sum % 2 != 0 {
            return Err(CubeError::EdgeFlip);
        }
        if perm_is_odd(&self.cp) != perm_is_odd(&self.ep) {
            return Err(CubeError::PermutationParity);
        }
        Ok(())
    }

    /// Paint this state back onto the 54 facelets.
    #[must_use]
    pub fn to_facelets(&self) -> Facelets {
        let t = tables();
        let mut arr = [Face::U; 54];
        for (f, &face) in Face::ALL.iter().enumerate() {
            arr[f * 9 + 4] = face;
        }
        for s in 0..8 {
            let piece = self.cp[s] as usize;
            let twist = self.co[s] as usize;
            let home = t.corner_home[piece];
            let facelets = t.corner_facelets[s];
            for j in 0..3 {
                arr[facelets[(twist + j) % 3]] = home[j];
            }
        }
        for s in 0..12 {
            let piece = self.ep[s] as usize;
            let flip = self.eo[s] as usize;
            let home = t.edge_home[piece];
            let facelets = t.edge_facelets[s];
            arr[facelets[flip]] = home[0];
            arr[facelets[1 - flip]] = home[1];
        }
        Facelets(arr)
    }
}

impl Facelets {
    /// Extract the cubie model, checking that every slot holds a physically
    /// valid, non-repeated piece.
    ///
    /// # Errors
    /// Returns a [`CubeError`] describing the first structural problem found.
    pub fn to_cube_state(&self) -> Result<CubeState, CubeError> {
        let t = tables();
        for (f, &face) in Face::ALL.iter().enumerate() {
            if self.get(f * 9 + 4) != face {
                return Err(CubeError::BadCenters);
            }
        }
        for face in Face::ALL {
            if (0..54).filter(|&i| self.get(i) == face).count() != 9 {
                return Err(CubeError::WrongColorCounts(face));
            }
        }

        let mut cp = [0u8; 8];
        let mut co = [0u8; 8];
        let mut used_corner = [false; 8];
        for s in 0..8 {
            let f = t.corner_facelets[s];
            let actual = [self.get(f[0]), self.get(f[1]), self.get(f[2])];
            let piece = match_corner(actual, t).ok_or(CubeError::InvalidCornerPiece)?;
            let home = t.corner_home[piece];
            let twist = (0..3)
                .find(|&k| actual[k] == home[0])
                .ok_or(CubeError::InvalidCornerPiece)?;
            for j in 0..3 {
                if actual[(twist + j) % 3] != home[j] {
                    return Err(CubeError::InvalidCornerPiece);
                }
            }
            if used_corner[piece] {
                return Err(CubeError::RepeatedCornerPiece);
            }
            used_corner[piece] = true;
            cp[s] = piece_index(piece);
            co[s] = piece_index(twist);
        }

        let mut ep = [0u8; 12];
        let mut eo = [0u8; 12];
        let mut used_edge = [false; 12];
        for s in 0..12 {
            let f = t.edge_facelets[s];
            let actual = [self.get(f[0]), self.get(f[1])];
            let piece = match_edge(actual, t).ok_or(CubeError::InvalidEdgePiece)?;
            let home = t.edge_home[piece];
            let flip = if actual == home {
                0
            } else if actual[0] == home[1] && actual[1] == home[0] {
                1
            } else {
                return Err(CubeError::InvalidEdgePiece);
            };
            if used_edge[piece] {
                return Err(CubeError::RepeatedEdgePiece);
            }
            used_edge[piece] = true;
            ep[s] = piece_index(piece);
            eo[s] = flip;
        }

        Ok(CubeState { cp, co, ep, eo })
    }

    /// Extract the cubie model and verify the cube is solvable.
    ///
    /// # Errors
    /// Returns a [`CubeError`] for the first structural or solvability problem.
    pub fn validate(&self) -> Result<CubeState, CubeError> {
        let state = self.to_cube_state()?;
        state.solvability()?;
        Ok(state)
    }
}

/// Convert a small slot/piece index (always `< 12`) to `u8`.
fn piece_index(i: usize) -> u8 {
    u8::try_from(i).expect("cubie index fits in u8")
}

/// True if the permutation has odd parity.
pub(crate) fn perm_is_odd(perm: &[u8]) -> bool {
    let n = perm.len();
    let mut seen = vec![false; n];
    let mut transpositions = 0usize;
    for start in 0..n {
        if seen[start] {
            continue;
        }
        let mut j = start;
        let mut len = 0usize;
        while !seen[j] {
            seen[j] = true;
            j = perm[j] as usize;
            len += 1;
        }
        transpositions += len - 1;
    }
    transpositions % 2 == 1
}

fn sorted3(c: [Face; 3]) -> [usize; 3] {
    let mut a = [c[0].index(), c[1].index(), c[2].index()];
    a.sort_unstable();
    a
}

fn sorted2(c: [Face; 2]) -> [usize; 2] {
    let mut a = [c[0].index(), c[1].index()];
    a.sort_unstable();
    a
}

fn match_corner(colors: [Face; 3], t: &Tables) -> Option<usize> {
    let key = sorted3(colors);
    (0..8).find(|&p| sorted3(t.corner_home[p]) == key)
}

fn match_edge(colors: [Face; 2], t: &Tables) -> Option<usize> {
    let key = sorted2(colors);
    (0..12).find(|&p| sorted2(t.edge_home[p]) == key)
}

/// Geometry-derived slot tables. Slot `s` and piece `s` coincide in the solved
/// cube, so `*_home[s]` is both slot `s`'s solved colors and piece `s`'s colors.
pub(crate) struct Tables {
    /// Corner slot facelets, ordered `[y-facelet, then geometric handed order]`.
    pub(crate) corner_facelets: [[usize; 3]; 8],
    /// Edge slot facelets, ordered `[primary, secondary]`.
    pub(crate) edge_facelets: [[usize; 2]; 12],
    pub(crate) corner_home: [[Face; 3]; 8],
    pub(crate) edge_home: [[Face; 2]; 12],
}

/// Crate-internal access to the geometry-derived slot tables.
pub(crate) fn slot_tables() -> &'static Tables {
    tables()
}

fn home_color(idx: usize) -> Face {
    Face::ALL[idx / 9]
}

fn tables() -> &'static Tables {
    static T: OnceLock<Tables> = OnceLock::new();
    T.get_or_init(build_tables)
}

#[allow(clippy::needless_range_loop)]
fn build_tables() -> Tables {
    let g = facelet_geometry_table();
    let mut corner_facelets = [[0usize; 3]; 8];
    let mut edge_facelets = [[0usize; 2]; 12];
    let mut corner_home = [[Face::U; 3]; 8];
    let mut edge_home = [[Face::U; 2]; 12];
    let mut ci = 0;
    let mut ei = 0;
    let mut seen = [false; 54];

    for i in 0..54 {
        if seen[i] {
            continue;
        }
        let pos = g[i].0;
        let zeros = pos.iter().filter(|&&v| v == 0).count();
        let group: Vec<usize> = (0..54).filter(|&j| g[j].0 == pos).collect();
        for &j in &group {
            seen[j] = true;
        }
        match zeros {
            2 => {} // center: nothing to record
            0 => {
                // Corner: facelets along x, y, z.
                let (mut fx, mut fy, mut fz) = (usize::MAX, usize::MAX, usize::MAX);
                for &j in &group {
                    let n = g[j].1;
                    if n[0] != 0 {
                        fx = j;
                    } else if n[1] != 0 {
                        fy = j;
                    } else {
                        fz = j;
                    }
                }
                // Handed cyclic order around the outward diagonal, starting at y.
                let handed_positive = pos[0] * pos[1] * pos[2] > 0;
                let ordered = if handed_positive {
                    [fy, fz, fx]
                } else {
                    [fy, fx, fz]
                };
                corner_facelets[ci] = ordered;
                corner_home[ci] = [
                    home_color(ordered[0]),
                    home_color(ordered[1]),
                    home_color(ordered[2]),
                ];
                ci += 1;
            }
            _ => {
                // Edge (one zero coordinate): two facelets.
                let (mut fx, mut fy, mut fz) = (usize::MAX, usize::MAX, usize::MAX);
                for &j in &group {
                    let n = g[j].1;
                    if n[0] != 0 {
                        fx = j;
                    } else if n[1] != 0 {
                        fy = j;
                    } else {
                        fz = j;
                    }
                }
                // Primary facelet: the U/D (y) facelet if present, else the
                // F/B (z) facelet. This is the standard edge-orientation
                // reference and makes the flip sum move-invariant.
                let (primary, secondary) = if fy == usize::MAX {
                    (fz, fx)
                } else {
                    (fy, if fz == usize::MAX { fx } else { fz })
                };
                edge_facelets[ei] = [primary, secondary];
                edge_home[ei] = [home_color(primary), home_color(secondary)];
                ei += 1;
            }
        }
    }

    Tables {
        corner_facelets,
        edge_facelets,
        corner_home,
        edge_home,
    }
}

/// Why a cube configuration is invalid or unsolvable.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CubeError {
    /// A face color does not appear exactly nine times.
    WrongColorCounts(Face),
    /// The six centers are not the six distinct colors.
    BadCenters,
    /// A corner slot's three colors are not a valid, correctly-oriented corner.
    InvalidCornerPiece,
    /// An edge slot's two colors are not a valid edge.
    InvalidEdgePiece,
    /// The same corner piece appears in two slots.
    RepeatedCornerPiece,
    /// The same edge piece appears in two slots.
    RepeatedEdgePiece,
    /// Corner twists do not sum to a multiple of three.
    CornerTwist,
    /// Edge flips do not sum to an even number.
    EdgeFlip,
    /// Corner and edge permutation parities disagree.
    PermutationParity,
    /// The known stickers cannot be completed to any solvable cube.
    ///
    /// Used for partial input where no single constraint is individually
    /// pinpointable but no valid completion exists.
    Contradiction,
}

impl std::fmt::Display for CubeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            CubeError::WrongColorCounts(c) => {
                return write!(f, "color {} does not appear exactly 9 times", c.to_char());
            }
            CubeError::BadCenters => "centers are not the six distinct colors",
            CubeError::InvalidCornerPiece => "a corner has an impossible color combination",
            CubeError::InvalidEdgePiece => "an edge has an impossible color combination",
            CubeError::RepeatedCornerPiece => "a corner piece appears twice",
            CubeError::RepeatedEdgePiece => "an edge piece appears twice",
            CubeError::CornerTwist => "a single corner is twisted (unsolvable)",
            CubeError::EdgeFlip => "a single edge is flipped (unsolvable)",
            CubeError::PermutationParity => "two pieces are swapped (unsolvable)",
            CubeError::Contradiction => "the known stickers cannot form a solvable cube",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for CubeError {}
