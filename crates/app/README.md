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

### Scanning with a camera (optional)

Built with the `camera-native` feature, press `C` in Input mode to scan your
cube with a webcam: hold each face to the camera and it's captured automatically
(`Space` to force a capture, `Esc`/`Tab` to exit). The detected colors fill the
cube and drop you into the paint review above, so you can fix any misreads
before solving.

```sh
cargo run -p rubic --features camera-native
```

Feature flags: `camera` builds the pure computer-vision core (native + web);
`camera-native` adds the desktop webcam source. Live video preview and a web
(browser) camera source are not yet implemented — see
[`docs/specs/0002-camera-cube-input.md`](../../docs/specs/0002-camera-cube-input.md).

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

The release build is size-critical — the `.wasm` blob is shipped to the browser.
The workspace `[profile.release]` (`opt-level = "s"`, `lto`, `codegen-units = 1`,
`panic = "abort"`, `strip`) plus `wasm-opt -Oz` (via `data-wasm-opt` in
`index.html`) take the debug build from ~72 MB down to ~20 MB, which Vercel
serves as ~4 MB brotli. `index.html` also passes `--enable-bulk-memory` (and
related) to `wasm-opt`, since rustc emits bulk-memory ops the bundled wasm-opt
otherwise rejects.

## Deploy to Vercel

The app is hosted on Vercel at **[rubik.mingjerlee.com](https://rubik.mingjerlee.com)**.

Vercel's build image has no Rust/trunk toolchain, so we deploy **prebuilt**: the
release `.wasm` is built locally (or in CI) and only the static output is
uploaded. `vercel.json` (repo root) pins the build command and cache headers;
[`scripts/deploy-web.sh`](../../scripts/deploy-web.sh) wraps the two-step flow.

First time only — create/link the Vercel project (interactive):

```sh
vercel login
vercel link            # keep "code directory" as ./ ; answer N to "link existing"
```

Every deploy — stop any local `trunk serve` first (it writes the same `dist/`),
then:

```sh
./scripts/deploy-web.sh --prod      # runs `trunk build --release`, uploads static
```

Custom domain: add `rubik.mingjerlee.com` to the project (`vercel domains add
rubik.mingjerlee.com rubic`), then create a `CNAME` record `rubik ->
cname.vercel-dns.com` at the DNS host for `mingjerlee.com`. Vercel provisions TLS
automatically.

## License

Licensed under either of MIT or Apache-2.0 at your option.
