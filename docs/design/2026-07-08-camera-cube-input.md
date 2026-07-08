# Design: Camera Cube Input

**Date:** 2026-07-08
**Spec:** [`../specs/0002-camera-cube-input.md`](../specs/0002-camera-cube-input.md)
**Status:** Phase A + B delivered; Phase C (web camera) deferred.

## Overview

The feature reads a physical cube's colors from a camera and feeds the result
into the existing paint-review flow ([`0001`](../specs/0001-visual-cube-input.md)).
Everything platform-independent is a pure, unit-tested "vision core"; only the
frame *source* is platform-specific.

## Module layout (`crates/app/src`)

```
vision/                 # pure CV core (feature `camera`; no Bevy)
  color.rs      HSV chroma/brightness point + distance
  sample.rs     median 3x3 grid sampling of a face image
  detect.rs     locate a face (foreground bbox) and crop to a square
  classify.rs   assign 54 samples to faces relative to the 6 centers
  pipeline.rs   Scan: accumulate 6 faces -> classify; capture_from_frame
  capture.rs    CaptureFlow: guided 6-face state machine + auto-capture
  source.rs     CameraSource trait + ReplaySource (tests/headless)
  native.rs     nokhwa webcam source (feature `camera-native`, non-wasm)
camera_scan.rs          # Bevy wiring (feature `camera`)
```

## Data flow

```
frame (RgbImage)
  -> detect_face        (crop the face region to a square)
  -> sample_face        (9 median cell colors, row-major)
  -> CaptureFlow.on_frame  (stability gate; route to the target face's slot)
  -> [six faces] Scan.classify   (relative to centers -> Facelets + margin)
  -> handoff -> InputState.partial (PartialFacelets)   [switch to Input mode]
  -> user reviews / corrects (0001 paint UI) -> validate -> solve
```

## Key decisions

- **Relative color classification.** Each sticker is matched to the nearest of
  the six *center* colors in an HSV-derived point `(s·cos h, s·sin h, v)`. This
  white-balances implicitly, so a uniform lighting shift does not change the
  result (tested). Absolute color tables would not survive lighting changes.
- **Guided capture, fixed order.** `CaptureFlow` walks the user through the six
  faces (URFDLB order) and auto-captures a face once its reading is stable for
  `STABILITY_FRAMES` consecutive frames; each capture is routed to its own slot,
  so order/rotation of presentation is guided, not inferred by CV. Orientation
  mistakes are fixed in the review step.
- **Source seam.** `CameraSource::next_frame() -> Option<RgbImage>` is the only
  platform boundary. `ReplaySource` (frames from a queue) makes the whole loop
  testable headlessly; `NativeCamera` (nokhwa) is the desktop source; a web
  source (`getUserMedia`) is the deferred Phase C.
- **Feature split.** `camera` = pure CV core (adds only `image`; builds on wasm).
  `camera-native` = `camera` + `nokhwa` (desktop only). The base app and the
  wasm build never pull nokhwa.
- **`image`-version decoupling.** Native frames are rebuilt via raw bytes
  (`decoded.into_raw()` -> `RgbImage::from_raw`), so nokhwa's internal `image`
  version cannot conflict with ours.

## What is verified vs. deferred

- **Tested offline (no hardware):** color separation, grid sampling, detection,
  classification, the full render→detect→sample→classify recovery, the capture
  state machine, source-driven capture, and the scan→review hand-off.
- **Compile-verified only:** the nokhwa native source and the Bevy camera mode —
  live camera + display are required to exercise them, so they are validated
  on-device.
- **Deferred:** live video *preview* as a GPU texture (a text HUD shows target
  face + progress instead), and Phase C (web `getUserMedia` source).

## Testing strategy

Synthetic face images are generated programmatically (a 3×3 grid of a cube's
sticker colors on a plain background, plus noise / grid lines / lighting shifts),
so the pipeline is exercised deterministically without a camera. A fully-correct
synthetic scan must recover the exact cube and validate as `Unique`.
