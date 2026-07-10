# Design: Input/Solve UI Clarity

**Date:** 2026-07-10
**Status:** Proposed.

## Problem

On the opening screen (Input mode) the loudest button is **`Solve this`**, but it
does nothing until a complete, uniquely-solvable cube has been entered. A fresh
cube is blank (`0/48 painted`), so the primary call-to-action reads as broken:
there is nothing to solve yet. The wider button vocabulary (`New game`,
`Camera: OFF`, `Scan cube`) also does not clearly teach the flow — *give the app
your cube (paint it or scan it), then solve it, or shuffle a random one to play.*

## Goals

- Every visible button uses a clear, cube-native verb.
- The `Solve` action communicates its own readiness: visible so the goal is
  obvious, but visibly inert until the entered cube is solvable.
- Confirm and lock in the existing "solving shows only the 3D cube" behavior.

Non-goals: no change to painting (direct sticker/net manipulation stays), the
camera scan flow, the solver logic, or the 3D view.

## Label changes

| Location | Today | New label | Notes |
|---|---|---|---|
| top bar (`touch.rs`), Input + Solve | `New game` | **`Shuffle`** | Builds a random solvable cube; same word in both modes. |
| top bar (`touch.rs`), Input | `Solve this` | **`Solve`** | Always visible in Input mode; dimmed until ready (see below). |
| bottom bar (`camera_scan.rs`), Input | `Camera: OFF` / `Camera: ON` | **`Turn on camera`** / **`Turn off camera`** | Says what the tap does. |
| bottom bar (`camera_scan.rs`), Input | `Scan cube` | **`Scan`** | Starts the guided face-by-face capture. |
| `Edit`, `Beginner`, `Optimal`, `< Prev`, `Play / Pause`, `Next >`, camera `Capture / Retake`, `Restart`, `Cancel` | — | *unchanged* | Already clear. |

## `Solve` readiness state (shown-but-disabled)

The `Solve` button is shown in Input mode as today (hidden in Solve/Camera),
but its styling now reflects whether the entered cube is solvable:

- **Ready** — the painted/scanned state is `Completion::Unique`: accent
  background (green, matching the camera `Capture` green `srgb(0.15, 0.60, 0.30)`)
  and full-white text. This is the "your cube is complete — solve it" moment.
- **Not ready** — anything else (`NeedMore` / `Impossible`): dimmed background
  and grey text, reading as inactive.

Tapping while not ready is already inert: the button injects `Enter`, and
`paint::mode_control` only confirms into Solve mode when the state is
`Completion::Unique` (`try_confirm`). No new guard is required for correctness;
the styling is what changes.

A pure predicate drives the styling and is unit-tested:

```rust
// touch.rs
#[must_use]
pub fn solve_ready(completion: &Completion) -> bool {
    matches!(completion, Completion::Unique(_))
}
```

The styling system (Input mode only) reads `InputState::completion()`, computes
`solve_ready`, and sets the `Solve` button's `BackgroundColor` + child
`TextColor` to the ready or dimmed palette. It only writes on change (same
guard pattern the other toggle systems use).

## Solving shows only the 3D cube (lock in existing behavior)

Already implemented: `net::toggle_input_ui` hides both the net (`NetRoot`) and
the palette (`PaletteRoot`) whenever `mode == Solve`, so solving shows only the
3D cube. This design keeps that and guards it against regression by extracting
the per-mode visibility decision into pure, tested helpers:

```rust
// net.rs
#[must_use]
pub fn net_visible(mode: AppMode) -> bool { mode != AppMode::Solve }

#[must_use]
pub fn palette_visible(mode: AppMode) -> bool { mode == AppMode::Input }
```

`toggle_input_ui` calls these instead of inlining the comparisons. Tests assert
the net is hidden in Solve mode and the palette is visible only in Input mode.

## Files touched

- `crates/app/src/touch.rs` — `New game`→`Shuffle`, `Solve this`→`Solve`; add
  `solve_ready` predicate and the `Solve` styling system; register the system.
- `crates/app/src/camera_scan.rs` — `Camera: OFF/ON`→`Turn on/off camera`,
  `Scan cube`→`Scan`.
- `crates/app/src/net.rs` — extract `net_visible` / `palette_visible`; use them
  in `toggle_input_ui`.
- `crates/app/src/main.rs` — schedule the new `Solve`-styling system (Input
  mode).

## Testing

- `solve_ready`: true only for `Completion::Unique`; false for `NeedMore` and
  `Impossible`.
- `net_visible` / `palette_visible`: correct for each `AppMode`.
- No behavioral change to solving, scanning, or painting — existing tests stand.
