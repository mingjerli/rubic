# Camera Cube Input

| | |
|--|--|
| **Status** | Phase A + B implemented; Phase C (web camera) deferred |
| **Date** | 2026-07-08 |
| **Design doc** | [`../design/2026-07-08-camera-cube-input.md`](../design/2026-07-08-camera-cube-input.md) |

## Summary

Capture a physical cube's configuration with a camera/webcam: the user shows
each face to the camera, the app autonomously detects the face and reads its nine
sticker colors, and the result auto-fills the cube state for review and solving.
Reduces input friction versus painting all 54 stickers by hand (feature
[`0001`](./0001-visual-cube-input.md)).

## Motivation

Even with the paint UI ([`0001`](./0001-visual-cube-input.md)), entering a cube
by hand is slow and error-prone — 48 clicks plus color selection. Pointing a
camera at the cube and turning it through its six faces is far faster and closer
to how people expect a modern solver to work.

## Goals

- The user can hold a real cube to a camera, rotate it through all six faces,
  and have the app read the colors — no manual sticker-by-sticker entry.
- Works on both native (desktop) and web (WASM) builds.
- The scan result flows into the existing cube state, so the user can review and
  fix any misread sticker before solving, using the paint UI from `0001`.
- The computer-vision core is testable without a live camera.

## Non-goals

- Solving from a single still photo of multiple faces, or from a video clip.
- Reconstructing hidden faces (all six must be shown).
- Auto-detecting a non-standard color scheme (centers define the scheme, as in
  `overview.md`).
- Guaranteeing correct reads under arbitrary lighting — misreads are expected
  and corrected via the review step, not eliminated.

## Design decisions (resolved clarifications)

These were settled during brainstorming and are fixed for this spec:

1. **Detection: autonomous, not a manual alignment grid.** The app finds the
   cube face itself rather than asking the user to line it up in an overlay.
   The tractable, shippable form of "autonomous" is: detect the face as the
   largest square-ish contour → perspective-warp it flat → slice into a 3×3
   grid → sample each cell. A soft on-screen guide box hints where to hold the
   cube but does not require precise alignment.
2. **Platforms: native and web, together.** One `CameraSource` trait abstracts
   the frame source; the vision/classification pipeline is shared. Native uses a
   Rust webcam crate (`nokhwa`); web uses the browser's `getUserMedia` with
   canvas pixel readback (`web-sys`).
3. **CV stack: pure Rust (`image` + `imageproc`), not OpenCV.** The web target
   makes OpenCV impractical (WASM build, size). Pure-Rust CV runs identically
   native and in-browser.
4. **Trust model: scan then confirm/correct.** The scan is a first pass that
   fills the shared `PartialFacelets`; the user reviews it on the 2D net / 3D
   cube and fixes misreads with the `0001` paint UI before solving. Live
   validation (`enough` / `impossible` / `ready`) applies throughout.
5. **Color classification is relative, not absolute.** Every sticker is
   classified against the six known center colors in a lighting-robust space
   (HSV), which white-balances implicitly. A confidence margin (gap between the
   best and second-best center) flags shaky reads.

## Functional requirements

1. A **Camera mode** (a third `AppMode` alongside Input and Solve), entered from
   the UI, showing a live camera preview.
2. **Autonomous face detection**: while a face is held to the camera, the app
   detects it (largest quadrilateral), warps it flat, and samples its nine cell
   colors — no manual grid alignment.
3. **Auto-capture per face**: when a detected face is stable and confident, its
   nine colors are captured without a keypress; a manual capture key is also
   available as a fallback.
4. **Per-face progress**: reuse the 2D net to show which of the six faces have
   been captured (e.g. a check-mark / fill per face), and which remain.
5. **Assemble + classify**: once faces are captured, classify all samples
   relative to the six centers and write the result into the shared
   `PartialFacelets` (the same state the paint UI edits).
6. **Review & correct**: after a scan, the user lands in the existing Input
   review — the 2D net and 3D cube show the detected colors and any sticker can
   be repainted. Live validation reports `n/48`, `impossible — <reason>`, or
   `ready`.
7. **Re-scan / clear**: the user can re-scan a face or clear all captured data
   and start over.

## Non-functional requirements

1. Runs on native and web (WASM) builds behind a `camera` cargo feature, so
   builds without a camera stay lean and continue to compile/pass CI.
2. Preview and detection keep the app responsive (target 60 FPS); heavy CV runs
   at a throttled cadence, not necessarily every frame.
3. The pure CV core (`vision`, `classify`) is unit-tested on synthetic/fixture
   images and sample sets, with no camera or GPU required.
4. No `unsafe`; clippy-clean; native + WASM both build in CI.

## User experience

From Input mode the user opens **Camera mode**. A live preview appears with a
soft guide box. The user holds the cube up and slowly rotates it to show each
face; as each face is recognized it is captured automatically and marked done on
the 2D net. When all six faces are captured, the app classifies the colors, fills
the cube, and returns to the Input review, where the detected cube is shown on
the net and the 3D cube. The user fixes any wrong stickers by painting, and once
the state is valid and complete, solves as usual. `--scramble` / `--facelets`
and hand painting (`0001`) remain available.

## Build phasing

Large feature; delivered in phases, each its own build cycle:

- **Phase A — pure CV core.** `vision` (frame → nine colors) and `classify`
  (samples → `PartialFacelets`), verified offline on generated fixture images
  and synthetic sample sets. No camera, no Bevy.
- **Phase B — native camera.** `nokhwa` source + Bevy Camera mode + live preview
  + capture flow + hand-off to the `0001` review.
- **Phase C — web camera.** `getUserMedia` source + WASM wiring; verify the WASM
  build.

## Testing & known limitations

- **Testable offline:** the CV pipeline is exercised with programmatically
  generated face images (flat 3×3 of known colors, plus perspective and noise)
  and synthetic 54-sample sets under simulated lighting shifts, asserting correct
  colors, correct face assignment, and low confidence on ambiguous input. A
  fully-correct synthetic scan must yield `Completion::Unique`.
- **Not testable in CI / this environment:** a live camera and the on-screen feel
  cannot be exercised without real hardware and a display. Real-world detection
  accuracy and color thresholds must be tuned on-device, and iteration there is
  expected. The manual-correction step (requirement 6) is what keeps the feature
  usable while thresholds are tuned.

## Acceptance criteria

- [x] The pure CV core is unit-tested (vision + classification) with no hardware.
- [x] A completed scan fills the shared `PartialFacelets` and lands in the
      review UI, where misreads can be repainted and validation is live.
      *(Hand-off unit-tested; live review reuses feature 0001.)*
- [x] Native and WASM builds compile with the `camera` feature; clippy-clean;
      no `unsafe`. *(Native camera source under `camera-native`.)*
- [~] Holding each face to the camera captures its nine colors; the capture
      state machine and native source are done, but live capture is
      compile-verified only — validated on-device.
- [ ] Camera mode shows a **live video preview**: deferred to on-hardware work
      (a text HUD shows target face + progress meanwhile).
- [ ] **Phase C:** web (`getUserMedia`) camera source — deferred.
