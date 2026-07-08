# rubic

An interactive 3x3 Rubik's Cube solver built with the [Bevy](https://bevyengine.org/) engine.

Enter a cube's colors, let `rubic` check that the configuration is valid, then
watch a step-by-step animation guide you (or an optimal solver) to the solution.
Runs as a native desktop app and in the browser (WebAssembly).

## Status

Early development. See [`SPECIFICATION.md`](./SPECIFICATION.md) for the full
requirements and [`docs/`](./docs) for design specs.

## Workspace layout

| Crate | Path | Purpose |
|-------|------|---------|
| `rubic-core` | `crates/core` | Pure-Rust cube model, move engine, validation, solvers (no rendering) |
| `rubic` (app) | `crates/app` | Bevy visualization, input, animation, CLI *(coming)* |

## Features (planned)

- Minimal color input with live "enough / impossible" feedback
- Two solve modes: **optimal** and **human-friendly cheat-sheet method**
- Step-forward / step-back solution animation
- Free camera: move, zoom, rotate, realign
- Manual cube play, like a real cube
- Printable cheat sheet with theory
- Native + web (WASM), launchable via a Rust CLI

## Development

```sh
cargo test        # run the core test suite
cargo clippy      # lint
```

## License

Licensed under either of [MIT](./LICENSE-MIT) or
[Apache-2.0](./LICENSE-APACHE) at your option.
