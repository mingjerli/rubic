//! Beginner (layer-by-layer) solver.
//!
//! The cube is solved in the seven classic stages (see [`Stage`]). The first
//! two layers are placed one piece at a time with a bounded IDA* search
//! ([`search::ida_place`]) that keeps already-placed pieces home; the last
//! layer is solved with breadth-first search ([`search::bfs`]) over small sets
//! of standard algorithms, each of which preserves the first two layers.

use super::cube::{Cube, c, e};
use super::search::{bfs, ida_place};
use crate::moves::{Move, Sequence};
use crate::solver::{Solution, SolveError, Solver, Stage, Step};
use crate::state::CubeState;

/// Layer-by-layer solver producing human-readable, stage-labelled steps.
#[derive(Debug, Default, Clone, Copy)]
pub struct BeginnerSolver;

impl Solver for BeginnerSolver {
    fn solve(&self, cube: &CubeState) -> Result<Solution, SolveError> {
        if cube.solvability().is_err() {
            return Err(SolveError::Unsolvable);
        }
        let mut state = Cube::from_facelets(&cube.to_facelets());
        let mut steps = Vec::new();

        bottom_cross(&mut state, &mut steps);
        bottom_corners(&mut state, &mut steps);
        middle_edges(&mut state, &mut steps);
        last_layer(&mut state, &mut steps);

        debug_assert!(state.is_solved(), "beginner solver left cube unsolved");
        if !state.is_solved() {
            return Err(SolveError::Unsolvable);
        }
        Ok(Solution { steps })
    }
}

const CROSS_EDGES: [usize; 4] = [e::DF, e::DR, e::DB, e::DL];

/// Stage 1: the four D-layer edges (cross on the bottom face).
fn bottom_cross(state: &mut Cube, steps: &mut Vec<Step>) {
    let mut solved = Vec::new();
    let mut moves = Vec::new();
    for &piece in &CROSS_EDGES {
        solved.push(piece);
        place(state, &mut moves, &solved, &[]);
    }
    push(
        steps,
        moves,
        Stage::BottomCross,
        "Bottom cross: place the four bottom-layer edges.",
    );
}

/// Place `edges`/`corners` starting from the current `state`, appending the
/// moves used and advancing `state` in place.
fn place(state: &mut Cube, moves: &mut Vec<Move>, edges: &[usize], corners: &[usize]) {
    let ms = ida_place(state, edges, corners);
    for &m in &ms {
        *state = state.apply(m);
    }
    moves.extend(ms);
}

/// Stage 2: the four D-layer corners (first layer complete).
///
/// Each corner is placed with a small BFS over insertion triggers. Every
/// trigger preserves the cross and the other bottom corners, so only the
/// target slot and slots not yet filled contribute generators.
fn bottom_corners(state: &mut Cube, steps: &mut Vec<Step>) {
    let u = [seq("U"), seq("U2"), seq("U'")];
    let slots: [(usize, [Vec<Move>; 2]); 4] = [
        (c::DFR, [seq("R U R'"), seq("F' U' F")]),
        (c::DLF, [seq("F U F'"), seq("L' U' L")]),
        (c::DBL, [seq("L U L'"), seq("B' U' B")]),
        (c::DRB, [seq("B U B'"), seq("R' U' R")]),
    ];
    let mut moves = Vec::new();
    for i in 0..slots.len() {
        let piece = slots[i].0;
        let mut gens: Vec<Vec<Move>> = u.to_vec();
        for slot in &slots[i..] {
            gens.push(slot.1[0].clone());
            gens.push(slot.1[1].clone());
        }
        let ms = bfs(state, &gens, move |c| {
            c.cp[piece] as usize == piece && c.co[piece] == 0
        });
        apply(state, &ms);
        moves.extend(ms);
    }
    push(
        steps,
        moves,
        Stage::BottomCorners,
        "Bottom corners: complete the first layer.",
    );
}

/// Stage 3: the four E-slice edges (first two layers complete).
///
/// Each edge is placed with a small BFS over slot-insertion algorithms. Every
/// insertion preserves the first layer and the other E-slice edges, so a
/// placed edge is never disturbed: only the target slot and slots not yet
/// filled contribute insertion generators.
fn middle_edges(state: &mut Cube, steps: &mut Vec<Step>) {
    let u = [seq("U"), seq("U2"), seq("U'")];
    // (edge piece, two insertion algorithms) per E-slice slot, in fill order.
    let slots: [(usize, [Vec<Move>; 2]); 4] = [
        (
            e::FR,
            [seq("U R U' R' U' F' U F"), seq("U' F' U F U R U' R'")],
        ),
        (
            e::FL,
            [seq("U' L' U L U F U' F'"), seq("U F U' F' U' L' U L")],
        ),
        (
            e::BR,
            [seq("U' R' U R U B U' B'"), seq("U B U' B' U' R' U R")],
        ),
        (
            e::BL,
            [seq("U L U' L' U' B' U B"), seq("U' B' U B U L U' L'")],
        ),
    ];
    let mut moves = Vec::new();
    for i in 0..slots.len() {
        let piece = slots[i].0;
        let mut gens: Vec<Vec<Move>> = u.to_vec();
        for slot in &slots[i..] {
            gens.push(slot.1[0].clone());
            gens.push(slot.1[1].clone());
        }
        let ms = bfs(state, &gens, move |c| {
            c.ep[piece] as usize == piece && c.eo[piece] == 0
        });
        apply(state, &ms);
        moves.extend(ms);
    }
    push(
        steps,
        moves,
        Stage::MiddleEdges,
        "Middle edges: complete the first two layers.",
    );
}

/// Push a completed stage's moves as a [`Step`] (moves already applied).
///
/// Every stage is emitted even when it needs no moves (already solved), so a
/// beginner solution always has the same seven stages in the same order — the
/// flow matches the cheat sheet every time. An empty step contributes nothing
/// to the move list; it just keeps the stage numbering fixed.
fn push(steps: &mut Vec<Step>, moves: Vec<Move>, stage: Stage, note: &str) {
    steps.push(Step {
        moves,
        stage,
        note: note.to_string(),
    });
}

/// Stages 4-7: orient last-layer edges, orient corners, permute corners,
/// permute edges. Each search uses only last-layer-preserving algorithms.
fn last_layer(state: &mut Cube, steps: &mut Vec<Step>) {
    let u = seq("U");
    let u2 = seq("U2");
    let ui = seq("U'");

    // Stage 4: orient last-layer edges into a cross.
    let k = seq("F R U R' U' F'");
    let gens = vec![u.clone(), u2.clone(), ui.clone(), k];
    let ms = bfs(state, &gens, top_edges_oriented);
    apply(state, &ms);
    push(
        steps,
        ms,
        Stage::TopCross,
        "Top cross: orient the last-layer edges.",
    );

    // Stage 5: orient last-layer corners (top face one color).
    let sune = seq("R U R' U R U2 R'");
    let gens = vec![u.clone(), u2.clone(), ui.clone(), sune];
    let ms = bfs(state, &gens, top_corners_oriented);
    apply(state, &ms);
    push(
        steps,
        ms,
        Stage::TopFace,
        "Top face: orient the last-layer corners.",
    );

    // Stage 6: permute last-layer corners.
    let cc = seq("R' F R' B2 R F' R' B2 R2");
    let cc_inv = inverse(&cc);
    let gens = vec![u.clone(), u2.clone(), ui.clone(), cc, cc_inv];
    let ms = bfs(state, &gens, top_corners_placed);
    apply(state, &ms);
    push(
        steps,
        ms,
        Stage::TopCorners,
        "Top corners: permute the last-layer corners.",
    );

    // Stage 7: permute last-layer edges, finishing the solve.
    let ee = seq("R U' R U R U R U' R' U' R2");
    let ee_inv = inverse(&ee);
    let mut gens = Vec::new();
    for k in 0..4 {
        let pre = u_pow(k);
        let post = u_pow((4 - k) % 4);
        gens.push(conjugate(&pre, &ee, &post));
        gens.push(conjugate(&pre, &ee_inv, &post));
    }
    let ms = bfs(state, &gens, Cube::is_solved);
    apply(state, &ms);
    push(
        steps,
        ms,
        Stage::TopEdges,
        "Top edges: permute the last-layer edges.",
    );
}

fn apply(state: &mut Cube, moves: &[Move]) {
    for &m in moves {
        *state = state.apply(m);
    }
}

// --- last-layer goal predicates ---------------------------------------------

fn top_edges_oriented(cube: &Cube) -> bool {
    [e::UF, e::UR, e::UB, e::UL]
        .iter()
        .all(|&s| cube.eo[s] == 0)
}

fn top_corners_oriented(cube: &Cube) -> bool {
    [c::URF, c::UFL, c::ULB, c::UBR]
        .iter()
        .all(|&s| cube.co[s] == 0)
}

fn top_corners_placed(cube: &Cube) -> bool {
    [c::URF, c::UFL, c::ULB, c::UBR]
        .iter()
        .all(|&s| cube.cp[s] as usize == s)
}

// --- small move helpers -----------------------------------------------------

/// Parse a move sequence written in standard notation.
fn seq(s: &str) -> Vec<Move> {
    s.parse::<Sequence>().expect("valid algorithm").0
}

/// The inverse of a move list.
fn inverse(moves: &[Move]) -> Vec<Move> {
    moves.iter().rev().map(|m| m.inverse()).collect()
}

/// `k` clockwise U turns as a minimal move list (`0..4`).
fn u_pow(k: usize) -> Vec<Move> {
    match k {
        1 => seq("U"),
        2 => seq("U2"),
        3 => seq("U'"),
        _ => Vec::new(),
    }
}

/// `pre` then `body` then `post`, concatenated.
fn conjugate(pre: &[Move], body: &[Move], post: &[Move]) -> Vec<Move> {
    let mut out = Vec::with_capacity(pre.len() + body.len() + post.len());
    out.extend_from_slice(pre);
    out.extend_from_slice(body);
    out.extend_from_slice(post);
    out
}
