//! Search primitives for the beginner solver.
//!
//! Two engines share the [`Cube`] model:
//! - [`ida_place`]: IDA* that places a chosen set of pieces at home while
//!   keeping every already-placed piece home. Used for the first two layers.
//! - [`bfs`]: breadth-first search over a small set of last-layer generator
//!   sequences (each of which preserves the first two layers).

use super::cube::Cube;
use crate::color::Face;
use crate::moves::{Amount, Move};
use std::collections::HashMap;
use std::collections::VecDeque;
use std::sync::OnceLock;

/// The 18 face turns, grouped by face for pruning.
fn all_moves() -> &'static [Move; 18] {
    static M: OnceLock<[Move; 18]> = OnceLock::new();
    M.get_or_init(|| {
        let mut v = Vec::with_capacity(18);
        for face in Face::ALL {
            for amount in [Amount::Cw, Amount::Double, Amount::Ccw] {
                v.push(Move { face, amount });
            }
        }
        v.try_into().expect("18 moves")
    })
}

/// `edge_move[m][slot] = (dest slot, flip delta)` for the m-th move in
/// [`all_moves`], applied to a single edge (independent of other pieces).
fn edge_move_table() -> &'static [[(usize, u8); 12]; 18] {
    static T: OnceLock<[[(usize, u8); 12]; 18]> = OnceLock::new();
    T.get_or_init(|| {
        let mut table = [[(0usize, 0u8); 12]; 18];
        for (mi, &m) in all_moves().iter().enumerate() {
            let c = Cube::SOLVED.apply(m);
            for (src, cell) in table[mi].iter_mut().enumerate() {
                let dest = (0..12).find(|&d| c.ep[d] as usize == src).expect("edge");
                *cell = (dest, c.eo[dest]);
            }
        }
        table
    })
}

/// `corner_move[m][slot] = (dest slot, twist delta)`.
fn corner_move_table() -> &'static [[(usize, u8); 8]; 18] {
    static T: OnceLock<[[(usize, u8); 8]; 18]> = OnceLock::new();
    T.get_or_init(|| {
        let mut table = [[(0usize, 0u8); 8]; 18];
        for (mi, &m) in all_moves().iter().enumerate() {
            let c = Cube::SOLVED.apply(m);
            for (src, cell) in table[mi].iter_mut().enumerate() {
                let dest = (0..8).find(|&d| c.cp[d] as usize == src).expect("corner");
                *cell = (dest, c.co[dest]);
            }
        }
        table
    })
}

/// `edge_dist[home][slot][ori]`: quarter-count distance for an edge whose home
/// is `home`, currently at `(slot, ori)`, to reach `(home, 0)`.
fn edge_dist() -> &'static [[[u8; 2]; 12]; 12] {
    static T: OnceLock<[[[u8; 2]; 12]; 12]> = OnceLock::new();
    T.get_or_init(|| {
        let moves = edge_move_table();
        let mut out = [[[u8::MAX; 2]; 12]; 12];
        for home in 0..12 {
            let dist = &mut out[home];
            dist[home][0] = 0;
            let mut queue = VecDeque::new();
            queue.push_back((home, 0usize));
            while let Some((slot, ori)) = queue.pop_front() {
                let d = dist[slot][ori];
                for mv in moves {
                    let (ns, flip) = mv[slot];
                    let no = ori ^ flip as usize;
                    if dist[ns][no] == u8::MAX {
                        dist[ns][no] = d + 1;
                        queue.push_back((ns, no));
                    }
                }
            }
        }
        out
    })
}

/// `corner_dist[home][slot][ori]`.
fn corner_dist() -> &'static [[[u8; 3]; 8]; 8] {
    static T: OnceLock<[[[u8; 3]; 8]; 8]> = OnceLock::new();
    T.get_or_init(|| {
        let moves = corner_move_table();
        let mut out = [[[u8::MAX; 3]; 8]; 8];
        for home in 0..8 {
            let dist = &mut out[home];
            dist[home][0] = 0;
            let mut queue = VecDeque::new();
            queue.push_back((home, 0usize));
            while let Some((slot, ori)) = queue.pop_front() {
                let d = dist[slot][ori];
                for mv in moves {
                    let (ns, delta) = mv[slot];
                    let no = (ori + delta as usize) % 3;
                    if dist[ns][no] == u8::MAX {
                        dist[ns][no] = d + 1;
                        queue.push_back((ns, no));
                    }
                }
            }
        }
        out
    })
}

fn heuristic(cube: &Cube, edges: &[usize], corners: &[usize]) -> u8 {
    let ed = edge_dist();
    let cd = corner_dist();
    let mut h = 0u8;
    for &piece in edges {
        let slot = cube.edge_slot(piece);
        h = h.max(ed[piece][slot][cube.eo[slot] as usize]);
    }
    for &piece in corners {
        let slot = cube.corner_slot(piece);
        h = h.max(cd[piece][slot][cube.co[slot] as usize]);
    }
    h
}

fn placed(cube: &Cube, edges: &[usize], corners: &[usize]) -> bool {
    edges
        .iter()
        .all(|&p| cube.ep[p] as usize == p && cube.eo[p] == 0)
        && corners
            .iter()
            .all(|&p| cube.cp[p] as usize == p && cube.co[p] == 0)
}

fn opposite(face: Face) -> Face {
    Face::ALL[(face.index() + 3) % 6]
}

/// Whether trying `m` right after `last` is redundant (same face, or the
/// higher-indexed half of a commuting opposite pair).
fn prune(last: Option<Face>, m: Move) -> bool {
    match last {
        None => false,
        Some(l) => l == m.face || (opposite(l) == m.face && m.face.index() > l.index()),
    }
}

enum Ida {
    Found,
    Bound(u8),
}

fn search(
    cube: &Cube,
    g: u8,
    bound: u8,
    path: &mut Vec<Move>,
    edges: &[usize],
    corners: &[usize],
    last: Option<Face>,
) -> Ida {
    let f = g + heuristic(cube, edges, corners);
    if f > bound {
        return Ida::Bound(f);
    }
    if placed(cube, edges, corners) {
        return Ida::Found;
    }
    let mut min = u8::MAX;
    for &m in all_moves() {
        if prune(last, m) {
            continue;
        }
        let next = cube.apply(m);
        path.push(m);
        match search(&next, g + 1, bound, path, edges, corners, Some(m.face)) {
            Ida::Found => return Ida::Found,
            Ida::Bound(b) => min = min.min(b),
        }
        path.pop();
    }
    Ida::Bound(min)
}

/// Find a move list that brings every piece in `edges`/`corners` home while
/// keeping any already-home listed piece home. Returns the moves (possibly
/// empty). The goal is always reachable for a solvable cube.
#[must_use]
pub fn ida_place(start: &Cube, edges: &[usize], corners: &[usize]) -> Vec<Move> {
    if placed(start, edges, corners) {
        return Vec::new();
    }
    let mut bound = heuristic(start, edges, corners);
    loop {
        let mut path = Vec::new();
        match search(start, 0, bound, &mut path, edges, corners, None) {
            Ida::Found => return path,
            Ida::Bound(b) => {
                if b == u8::MAX {
                    return path; // unreachable in practice for a valid cube
                }
                bound = b;
            }
        }
    }
}

/// Breadth-first search over `generators` (each a whole move sequence) from
/// `start` until `goal` holds. Returns the concatenated moves. Every generator
/// must preserve the first two layers so BFS explores only last-layer states.
#[must_use]
pub fn bfs(start: &Cube, generators: &[Vec<Move>], goal: impl Fn(&Cube) -> bool) -> Vec<Move> {
    if goal(start) {
        return Vec::new();
    }
    let mut prev: HashMap<Cube, (Cube, usize)> = HashMap::new();
    let mut queue: VecDeque<Cube> = VecDeque::new();
    queue.push_back(*start);
    prev.insert(*start, (*start, usize::MAX));
    while let Some(cube) = queue.pop_front() {
        for (gi, moves) in generators.iter().enumerate() {
            let mut next = cube;
            for &m in moves {
                next = next.apply(m);
            }
            if prev.contains_key(&next) {
                continue;
            }
            prev.insert(next, (cube, gi));
            if goal(&next) {
                return reconstruct(start, &next, &prev, generators);
            }
            queue.push_back(next);
        }
    }
    Vec::new()
}

fn reconstruct(
    start: &Cube,
    end: &Cube,
    prev: &HashMap<Cube, (Cube, usize)>,
    generators: &[Vec<Move>],
) -> Vec<Move> {
    let mut gens = Vec::new();
    let mut cur = *end;
    while &cur != start {
        let (parent, gi) = prev[&cur];
        gens.push(gi);
        cur = parent;
    }
    gens.reverse();
    let mut out = Vec::new();
    for gi in gens {
        out.extend_from_slice(&generators[gi]);
    }
    out
}
