# rubic (app)

Interactive [Bevy](https://bevyengine.org/) GUI and CLI for solving a 3x3
Rubik's Cube, built on [`rubic-core`](../core).

## Run natively

```sh
cargo run -p rubic                       # solved cube
cargo run -p rubic -- --scramble "R U R' U2 F"
cargo run -p rubic -- --facelets "<54-char URFDLB string>"
```

### Entering a real cube

Launch with no arguments and you start in **Input mode** with a blank cube.
Read the colors off your physical cube and paint them in — you never need to
know any moves:

1. Pick a color from the palette (click a swatch or press `1`–`6`).
2. Click stickers to paint them, either on the **2D net** (top-right, always
   labelled) or directly on the **3D cube**. Both stay in sync. Centers are
   fixed.
3. The HUD shows live status: `n/48 painted`, `impossible – <reason>`, or
   `ready`. When the cube is uniquely determined, press `Enter` (or `Tab`) to
   solve it.

`Tab` switches back to Input mode any time to edit. The **axis triad** beside
the cube is colored per face (+X=R, +Y=U, +Z=F …) and turns with the cube, so
you always know which face each key rotates — see the on-screen move legend.

### Controls

| Action | Keys |
|--------|------|
| Switch Input / Solve mode | `Tab` |
| **Input:** pick color | palette swatch or `1`–`6` |
| **Input:** paint sticker | click a net cell or a 3D sticker |
| **Input:** clear / confirm | `Delete` / `Enter` |
| Orbit / pan / zoom camera | left-drag / right-drag / wheel |
| Reset view | `Home` or `0` |
| Turn a face (clockwise) | `U D L R F B` |
| Turn counter-clockwise | hold `Shift` + face key |
| Reset cube to solved | `Backspace` |
| Solve (beginner / optimal) | `1` / `2` |
| Play / pause solution | `Space` |
| Next / previous move | `Right` / `Left` (or `N` / `P`) |

## Cheat sheet

```sh
cargo run -p rubic -- cheatsheet            # printable HTML to stdout
cargo run -p rubic -- cheatsheet --markdown # Markdown
cargo run -p rubic -- cheatsheet -o guide.html
```

## Run in the browser (WebAssembly)

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk
cd crates/app
trunk serve            # dev server at http://localhost:8080
trunk build --release  # static site in crates/app/dist
```

## License

Licensed under either of MIT or Apache-2.0 at your option.
