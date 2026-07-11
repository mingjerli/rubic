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

---

## Follow-on: explicit setup methods + Start over

### Problem

Even with the clearer labels, the opening screen shows the paint surface, the
palette, `Shuffle`, a dimmed `Solve`, and `Turn on camera` all at once. A new
user does not know they can paint a cube by hand, nor what "Turn on camera" is
for. And once painting or scanning, there is no obvious way to reset — the user
gets stuck mid-configuration.

### Model: an input *stage* within Input mode

Add an `InputStage` resource (`mode.rs`); it only matters while
`AppMode == Input`:

```rust
#[derive(Resource, Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum InputStage {
    #[default]
    ChooseMethod, // the method picker (open screen)
    Editing,      // manual painting / post-scan review
}
```

`AppMode` is unchanged (`Input | Camera | Solve`) — additive and low-risk.

### Behavior by state

| State | 3D cube | 2D net + palette | Buttons |
|---|---|---|---|
| Input · ChooseMethod (open) | solved **preview** | hidden | `Shuffle` · `Manual` · `Camera` (each with a hint line) |
| Input · Editing | paintable | shown | `Solve` (dim→green) · `Start over` |
| Camera | hidden | net fills live, palette hidden | `< Prev` · `Capture/Retake` · `Next >` · `Restart` · `Start over` |
| Solve | only view | hidden | `Shuffle` · `Edit` · `Beginner` · `Optimal` · playback |

Transitions:

- `Shuffle` (`G`) — scramble → **Solve** (any stage).
- `Manual` (`M`) — → **Editing**, clearing the cube to blank to paint.
- `Camera` (`C`) — one tap: open the webcam and enter the guided **Camera**
  scan; on completion → **Editing** (review the filled net).
- `Solve` (`Enter`, when green) — → **Solve**.
- `Start over` (`Esc`) — → **ChooseMethod**, reseeding the solved preview.
  Available in Editing and (replacing `Cancel`) in Camera.
- Solve-mode `Edit` (`Tab`) — → **Editing**.

### Decisions

- **Solved preview.** In `ChooseMethod` the input partial is seeded to
  `Facelets::SOLVED`, so the 3D cube reads as a real cube (not grey "unknown"
  stickers). `Manual` clears it to blank; `Start over` reseeds it.
- **Method hints.** `Shuffle` "random cube", `Manual` "paint by hand",
  `Camera` "scan with webcam" — a small grey sub-label under the main label.
  Buttons with a hint use a two-line (column) child layout; others stay flat.
- **Camera availability.** The `Camera` button and its `C` handler exist only
  under `cfg(feature = "camera")`; a native no-camera build shows just
  `Shuffle` · `Manual`.
- **One-tap camera.** `enter_camera_scan` opens the device if needed, then
  starts the scan. The old bottom-bar `Turn on camera` / `Scan` toggle and the
  `update_camera_toggle_label` system are removed; the bottom bar is
  camera-mode-only now.
- **CLI seeding.** `--scramble` / `--facelets` start in **Editing** (a cube was
  provided), skipping the picker.
- **Status line.** In `ChooseMethod` the HUD reads "choose a setup method"
  instead of the painted-count status.

### Files touched (this follow-on)

- `mode.rs` — `InputStage` resource + `editing_input` run condition.
- `touch.rs` — `Manual` / `StartOver` / `Camera` controls, hint sub-labels,
  per-stage visibility (Camera gated by feature).
- `camera_scan.rs` — one-tap open+scan, `Cancel`→`Start over`, set `Editing` on
  completion, drop the `Camera`/`Scan` toggle + label system.
- `net.rs` — `net_visible` / `palette_visible` take `InputStage`.
- `paint.rs` — stage transitions in `mode_control` (Manual, Start over, confirm
  only in Editing, `Edit` back to Editing).
- `ui.rs` — `ChooseMethod` status text.
- `main.rs` — init `InputStage`, seed the preview, gate paint systems on
  `editing_input`, drop the removed camera-label system.

### Testing (this follow-on)

- `net_visible` / `palette_visible`: net hidden in `ChooseMethod` and Solve,
  shown in Editing and Camera; palette shown only in Editing.
- Stage transition helpers (Manual → Editing+blank, Start over →
  ChooseMethod+solved) unit-tested where they are pure.
- Existing solve/scan/paint tests stand.

---

## Follow-on: phone (narrow) layout — stop the overlaps

### Problem

On a phone the 3D cube renders centered in the full window, so its top face
pokes into the 2D net stacked above it (manual editing); during a camera scan
the net, the 3D cube, and the instruction HUD all pile on top of each other.

### Fixes

- **Manual on a phone — cube below the net.** The cube sits at the world
  origin; `responsive_layout` raises the camera *focus* (`CUBE_DOWN_SHIFT`) only
  when `narrow && Input && Editing`, dropping the cube into the empty space
  under the net. Re-applied only on a layout change (keyed on
  `(narrow, shift_down)`) so it never fights the user's pinch-zoom / orbit.
- **Hide the 3D cube during a scan** (`cube_render::toggle_cube_visibility`,
  every width) — it is redundant with the net + preview and was the main
  overlap source. With the cube gone, `draw_axes` also skips Camera mode so the
  orientation triad doesn't float alone, and the net has room to sit at the top
  without colliding with anything.
- **Keep the net visible during a scan** as the progress view — it fills in
  face-by-face as each is captured. Entering a scan from the method picker
  clears the solved preview (`PartialFacelets::new()`) so the net starts blank
  and the filling reads as genuine progress.

### Testing

- `net_visible`: net shown during a camera scan (the progress view), hidden in
  `ChooseMethod` and Solve. Verified in a browser at desktop and phone widths:
  manual (cube clears the net), camera (cube + axes hidden, net fills in as
  faces are captured, controls below) — no overlaps.
