# rubic documentation

Documentation for the `rubic` Rubik's Cube solver.

## Layout

| Folder | Holds | Answers |
|--------|-------|---------|
| [`specs/`](./specs) | Product specifications — requirements and behavior | *What* should be built, and why |
| [`design/`](./design) | Technical design docs — architecture and implementation plans | *How* it is built |

Specs describe desired behavior independent of implementation; design docs
record the technical decisions that realize them. A feature usually starts as a
spec, then gets one or more design docs before implementation.

## Specifications (`specs/`)

- [`overview.md`](./specs/overview.md) — the master product specification
  (goals, functional and non-functional requirements, clarifications).
- [`0001-visual-cube-input.md`](./specs/0001-visual-cube-input.md) — painting a
  physical cube's colors (2D net + 3D) with live validation, and the orientation
  axis reference.
- [`0002-camera-cube-input.md`](./specs/0002-camera-cube-input.md) — capturing a
  cube's colors with a camera (autonomous detection, native + web), feeding the
  paint-review flow. *Accepted; not yet implemented.*
- [`TEMPLATE.md`](./specs/TEMPLATE.md) — copy this to start a new feature spec.
- Future per-feature specs: `NNNN-<feature>.md`, where `NNNN` is the next
  zero-padded number (e.g. `0002-timer-and-stats.md`).

## Design docs (`design/`)

- [`2026-07-07-foundation-cube-core.md`](./design/2026-07-07-foundation-cube-core.md)
  — cube model, move engine, validation, and solvers (`rubic-core`).
- Future design docs: `YYYY-MM-DD-<topic>.md`.

## Adding documentation for a new feature

1. Write or extend a spec in `specs/` describing the desired behavior. For a
   self-contained feature, add `specs/NNNN-<feature>.md` and link it below; for
   a small change, extend `specs/overview.md`.
2. Add a design doc in `design/` (`YYYY-MM-DD-<topic>.md`) capturing the
   technical approach. Link its source requirements back to the relevant spec.
3. Add the new files to the lists above so this index stays complete.
