# Visual Cube Input

| | |
|--|--|
| **Status** | Implemented |
| **Date** | 2026-07-08 |
| **Design doc** | Implemented directly in `crates/app` (`paint.rs`, `net.rs`, `axis.rs`, `mode.rs`); no separate design doc. |

## Summary

A way for someone holding a physical scrambled cube to enter its configuration
by painting colors — on a 2D unfolded net or directly on the 3D cube — with live
feedback on whether the entered state is complete, impossible, or ready to solve.

## Motivation

The original app could only take a cube via `--scramble` (a move sequence) or
`--facelets` (54 raw letters). Both are useless for a real cube: to type a
scramble you must already know the moves — and the solution is just that
sequence reversed — while hand-typing 54 URFDLB letters in the correct order is
error-prone and unintuitive. There was no practical way to input the one thing
the app exists to solve: an actual scrambled cube in front of you.

A second problem compounded it: because the camera orbits freely, the user can't
tell which side is "front", so even the manual face-turn keys (`F`, `R`, …) are
ambiguous until something moves.

## Goals

- Enter a physical cube's colors without knowing any move notation.
- Two painting surfaces (2D net and 3D cube) backed by one shared state.
- Live validation: complete / impossible / ready, reusing `PartialFacelets`.
- Make cube orientation unambiguous so face-turn keys are predictable.

## Non-goals

- Camera- or photo-based color scanning.
- Auto-detecting the color scheme (centers are assumed to define it).
- Editing mid-solve; input happens before solving, with reset to re-enter.

## Functional requirements

1. An **Input mode** (distinct from Solve mode, toggled with `Tab`) in which
   painting is active and face-turn/solve keys are suppressed.
2. A **2D unfolded net** panel (labelled cross layout) whose cells are painted
   by clicking; centers are fixed.
3. **3D painting**: clicking a sticker on the rotatable cube paints it.
4. Both surfaces read and write a single shared `PartialFacelets`, staying
   live-synced.
5. A **6-color palette** selectable by click or number keys `1`–`6`.
6. **Live status** from `PartialFacelets::analyze`: `n/48 painted`,
   `impossible — <reason>`, or `ready`. Solving is only enabled when the state
   is uniquely determined; `Enter` confirms it into a solvable cube.
7. Controls to **clear** to unknown and to **reset** to solved.
8. An **orientation reference**: a face-colored axis triad (+X=R, −X=L, +Y=U,
   −Y=D, +Z=F, −Z=B) that turns with the cube, plus a move legend mapping each
   turn to its axis and direction (`Shift` = reverse, `2` = 180°).

## Non-functional requirements

1. Works in both native and WebAssembly builds.
2. 60 FPS; painting has no perceptible lag.
3. Pure input logic (net↔facelet mapping, paint/clear, axis mapping,
   completion) is unit-tested without a renderer.

## User experience

Launch with no arguments → Input mode, blank cube. Pick a color (swatch or
`1`–`6`), read the physical cube, and paint each sticker on the net or the 3D
cube. The HUD shows progress and validity; when the cube is uniquely determined,
press `Enter` (or `Tab`) to switch to Solve mode and solve it. `Tab` returns to
Input mode to edit. `--scramble` / `--facelets` still work as an optional seed.

## Clarifications & open questions

- **Q:** When is input "enough"?
  **A:** When `PartialFacelets::analyze` returns `Unique` — i.e. exactly one
  solvable cube is consistent with the painted stickers. Zero → impossible;
  more than one → keep painting. (See `overview.md`, clarification 1.)

## Acceptance criteria

- [x] Cube can be fully entered by painting, with no move notation.
- [x] Net and 3D surfaces share state and stay in sync.
- [x] Status reflects complete / impossible / ready live.
- [x] Axis triad turns with the cube; move legend present.
- [x] Native + wasm build, clippy-clean, pure logic unit-tested.
