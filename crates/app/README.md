# rubic (app)

Interactive [Bevy](https://bevyengine.org/) GUI and CLI for solving a 3x3
Rubik's Cube, built on [`rubic-core`](../core).

## Run natively

```sh
cargo run -p rubic                       # solved cube
cargo run -p rubic -- --scramble "R U R' U2 F"
cargo run -p rubic -- --facelets "<54-char URFDLB string>"
```

### Controls

| Action | Keys |
|--------|------|
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
