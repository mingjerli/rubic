# Foundation Design: `rubic-core` (cube model, validation, solvers)

**Date:** 2026-07-07
**Scope:** The pure-Rust core of the 3x3 solver. No rendering. Everything here
is unit-testable and consumed by the Bevy app, CLI, and cheat-sheet generator.
**Source requirements:** [`SPECIFICATION.md`](../../../SPECIFICATION.md)

## 1. Goals & non-goals

**In scope (this foundation):**
- Cube state model with two representations (facelet + cubie) and conversions.
- Move engine: all 18 face moves and their algebra (inverse, compose, double).
- Input completion + solvability: from a *partial* sticker input decide
  `Impossible` / `NeedMore` / `Unique(CubeState)`.
- Two solvers behind one `Solver` trait: `BeginnerSolver` (human, layer-by-layer)
  and `OptimalSolver` (short solution, ≤ 1s).

**Out of scope (later specs):** Bevy rendering, camera, input UI, animation,
cheat-sheet document generation, CLI, WASM packaging. The solvers here emit a
data structure (`Solution`) that those layers consume.

## 2. Colors, faces, and the color scheme

- `Face`: `U, D, L, R, F, B` (Up, Down, Left, Right, Front, Back).
- `Color`: six colors. On a 3x3 the **centers are fixed**, so the six center
  colors define the scheme. Default (Western): `U=White, D=Yellow, F=Green,
  B=Blue, R=Red, L=Orange`. The scheme is data, not hard-coded into logic — the
  center stickers determine which color maps to which face.
- Facelet indexing: each face is a 3x3 grid indexed `0..9` row-major as seen
  when that face is toward the viewer with `U` up. 54 stickers total, ordered
  `[U(9), R(9), F(9), D(9), L(9), B(9)]` (the "URFDLB" convention, matching most
  solver literature so an optimal-solver crate can be fed directly).

## 3. State representations

### 3.1 Facelet model (`Facelets`)
`[Color; 54]` in URFDLB order. Used for input, rendering, and (de)serialization
to a compact string (54 chars of `U R F D L B`). This is the human/IO boundary.

### 3.2 Cubie model (`CubeState`) — the canonical internal state
Standard permutation+orientation model:
- Corners: `cp: [u8; 8]` (which corner cubie sits in each of 8 slots),
  `co: [u8; 8]` (twist ∈ {0,1,2}).
- Edges: `ep: [u8; 12]` (which edge cubie in each of 12 slots),
  `eo: [u8; 12]` (flip ∈ {0,1}).

`CubeState::SOLVED` is identity permutations, zero orientations. All solvers and
validity checks operate on `CubeState`. Conversions `Facelets <-> CubeState`
live in one module and are covered by round-trip tests.

**Why two models:** facelets are natural for IO/rendering; the cubie model makes
moves, validity, and solving simple and fast. This mirrors established cube
libraries and keeps each module small and single-purpose.

## 4. Move engine

- `Move { face: Face, amount: Amount }` where `Amount ∈ {Cw, Ccw, Double}`
  (quarter clockwise, quarter counter-clockwise, half turn). 18 distinct moves.
- Parse/format standard notation: `U U' U2 R L2 ...`.
- Each of the 6 base clockwise quarter turns is defined once as a permutation on
  `(cp, co, ep, eo)`. `Ccw = Cw⁻¹`, `Double = Cw∘Cw`. `apply(&self, Move)` returns
  a **new** `CubeState` (immutable; never mutates in place).
- `Sequence(Vec<Move>)` with `inverse()` and `compose()`.

**Property tests (RED first):**
- `SOLVED.apply(m).apply(m.inverse()) == SOLVED` for all 18 moves.
- Any quarter turn applied 4× is identity; any move applied to notation and its
  inverse-notation cancels.
- "Sexy move" `(R U R' U')` has order 6 (identity after 6 repeats).
- A fixed scramble reaches a known facelet string (golden test).

## 5. Input completion + solvability

Implements clarification #1: *"if we can mathematically determine all 54 places,
input is enough; if the configuration is impossible, tell the user."*

### 5.1 Full-cube solvability (reachable-state check)
A complete `Facelets` is a valid, **solvable** cube iff, after conversion to the
cubie model:
1. Every corner piece and every edge piece appears exactly once (valid piece set,
   correct color triples/pairs — no impossible stickers).
2. Corner orientation sum ≡ 0 (mod 3).
3. Edge orientation sum ≡ 0 (mod 2).
4. Permutation parity: `sign(cp) == sign(ep)`.

Failing 1 → `Impossible(reason)`. Failing 2–4 → `Impossible(reason)` (physically
a valid cube can't be scrambled into that state).

### 5.2 Partial input → `Completion`
Input is a partial assignment: some of the 54 stickers set, others `Unknown`
(centers are always known — they define the scheme). After each sticker change
we compute:

```
enum Completion {
    Impossible(Reason),        // 0 valid completions
    NeedMore { known: u8 },    // >1 valid completions
    Unique(CubeState),         // exactly 1 → "enough"
}
```

**Algorithm (CSP with early-exit):** treat the 8 corner slots and 12 edge slots
as variables whose domains are the physical pieces (8 fixed corner pieces, 12
fixed edge pieces, each with its possible orientations). Known stickers prune
domains. Then:
1. Constraint-propagate (arc-consistency): a slot whose known stickers match
   exactly one piece+orientation is fixed; remove that piece from other domains.
2. Backtracking search over remaining slots, pruned by the §5.1 constraints,
   **stopping as soon as a 2nd full solution is found**.
3. Zero solutions → `Impossible`; exactly one → `Unique`; two+ → `NeedMore`.

Search space is tiny (≤20 pieces, heavily constrained), so this is fast enough
to run on every keystroke. This is the "research → algorithm" step the spec asked
for; it is fully specified above.

**Practical note (expectation-setting):** because centers are fixed, in practice
uniqueness is typically reached only when ~5 of 6 faces are entered (the last
face is then forced). The savings over full entry are modest but real, and the
`Impossible` feedback is immediate.

**Tests:** solved cube = `Unique`; solved minus last face = `Unique`; a cube with
one corner twisted = `Impossible` (co sum); swapped two edges = `Impossible`
(parity); ambiguous partial (2 faces) = `NeedMore`.

## 6. Solvers

One trait so the app/animation/cheat-sheet treat both uniformly and the user can
pick a mode:

```rust
pub trait Solver {
    fn solve(&self, cube: &CubeState) -> Result<Solution, SolveError>;
}

pub struct Solution { pub steps: Vec<Step> }
pub struct Step {
    pub moves: Vec<Move>,
    pub stage: Stage,   // WhiteCross, FirstLayer, ... , or Optimal
    pub note: String,   // human explanation; feeds the cheat sheet
}
```

`solve` requires an already-validated, solvable `CubeState` (returns
`SolveError::Unsolvable` otherwise).

### 6.1 `BeginnerSolver` (human layer-by-layer)
Classic beginner method as ordered stages, each emitting labelled `Step`s:
1. Bottom cross
2. Bottom corners (first layer)
3. Middle-layer edges
4. Top cross (edge orientation)
5. Top face (corner orientation)
6. Top corners permutation
7. Top edges permutation

Each stage is "match current pattern → apply the stage's known algorithm, repeat
until the stage invariant holds." Stage invariants are asserted in tests; the
`note`/`stage` fields become the cheat sheet's content. **Correctness test:**
1000 random solvable scrambles all reach `SOLVED`, and each stage's invariant
holds when that stage completes.

### 6.2 `OptimalSolver` (short solution, ≤ 1s)
Two-phase (Kociemba-style) near-optimal solver. **Decision:** prefer reusing a
maintained crate (evaluate `kociemba` and alternatives) rather than hand-rolling,
per "simplicity over over-engineering." The trait boundary makes the choice
swappable; if no suitable crate qualifies (licensing/quality/`no unsafe`), fall
back to a from-scratch Thistlethwaite solver in a later iteration. **Test:** 1000
random scrambles solve to `SOLVED` in < 1s each and within a sane move bound.

## 7. Module layout (many small files)

```
crates/core/src/
  lib.rs            // re-exports, crate docs
  color.rs          // Color, Face, scheme
  facelets.rs       // Facelets, string (de)serialization
  state.rs          // CubeState (cubie model), SOLVED
  convert.rs        // Facelets <-> CubeState
  moves.rs          // Move, Amount, notation parse/format
  engine.rs         // base move tables, apply, Sequence
  validate.rs       // §5 solvability + Completion CSP
  solver/
    mod.rs          // Solver trait, Solution, Step, Stage, SolveError
    beginner.rs     // §6.1
    optimal.rs      // §6.2
```

## 8. Error handling & practices

- No `unsafe` (workspace-forbidden). No panics on bad input — return
  `Result`/typed enums. `thiserror` for error types.
- Immutable state transitions (moves return new `CubeState`).
- TDD: every section above lists its RED tests; write them first.
- `#![warn(missing_docs)]`; every public item documented.

## 9. Build order (each = its own TDD cycle / task)

1. `color` + `facelets` + `state` + `convert` (round-trip tests).
2. `moves` + `engine` (move algebra property tests).  → Task #3
3. `validate` (solvability + completion).             → Task #4
4. `solver::beginner`.                                 → Task #5
5. `solver::optimal`.                                  → Task #6
