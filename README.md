# rubic

An interactive 3x3 Rubik's Cube solver built with the [Bevy](https://bevyengine.org/) engine.

Enter a cube's colors (or a scramble), let `rubic` check that the configuration
is valid, then watch a step-by-step animation guide you — via a human-friendly
layer-by-layer method or a near-optimal two-phase solver — to the solution. Runs
as a native desktop app and in the browser (WebAssembly).

## Workspace layout

| Crate | Path | Purpose |
|-------|------|---------|
| `rubic-core` | `crates/core` | Pure-Rust cube model, move engine, validation, solvers (no rendering) |
| `rubic` (app) | `crates/app` | Bevy visualization, input, animation, and CLI |

## Features

- **Minimal input** with live "enough / impossible" feedback — enter stickers
  until the whole cube is mathematically determined; invalid states are rejected.
- **Two solve modes:** near-optimal (Kociemba two-phase) and a human-friendly
  layer-by-layer method that matches the cheat sheet.
- **Step-forward / step-back** solution animation with play/pause.
- **Free camera:** move, zoom, rotate, realign.
- **Manual cube play**, like a real cube.
- **Printable cheat sheet** (HTML or Markdown) with the theory behind each step.
- **Native + web (WASM)**, launchable via a Rust CLI.

## Quick start

```sh
cargo run -p rubic                          # launch the GUI (solved cube)
cargo run -p rubic -- --scramble "R U R' U2 F"
cargo run -p rubic -- cheatsheet -o guide.html   # printable cheat sheet
```

See [`crates/app/README.md`](./crates/app/README.md) for the full control list
and the WebAssembly build instructions.

## Using the library

`rubic-core` is a standalone, rendering-free crate:

```rust
use rubic_core::{Facelets, Sequence, solver::BeginnerSolver, Solver};

let cube = Facelets::SOLVED.apply_seq(&"R U R' U2 F".parse::<Sequence>().unwrap());
let state = cube.validate().expect("solvable");
let solution = BeginnerSolver.solve(&state).unwrap();
println!("{} moves", solution.move_count());
```

## Development

```sh
cargo test -p rubic-core                    # core suite
cargo test -p rubic-core --features optimal # + optimal solver
cargo test -p rubic                         # app logic
cargo clippy --workspace --all-targets
cargo fmt --all
```

Design specs live in [`docs/superpowers/specs/`](./docs/superpowers/specs);
requirements in [`SPECIFICATION.md`](./SPECIFICATION.md).

## Publishing

The workspace publishes to crates.io as two crates — publish `rubic-core` first,
then `rubic`:

```sh
cargo publish -p rubic-core
cargo publish -p rubic
```

## License

Licensed under either of [MIT](./LICENSE-MIT) or
[Apache-2.0](./LICENSE-APACHE) at your option.
