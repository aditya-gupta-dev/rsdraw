# Development

rsdraw is a Cargo workspace with two crates. Rendering is native Rust; no QuickJS, JavaScript runtime, Node, browser, or Excalidraw checkout is needed to build or run it.

## Layout

- `crates/rsdraw-core/`: independently buildable drawing library; Cairo/Pango dependencies, no GTK or Wayland dependencies.
- `crates/rsdraw-core/src/rough.rs`: seeded RoughJS 4.6.4 operations, corner geometry, arrowheads, curve subdivision, and hachure scan conversion.
- `crates/rsdraw-core/src/freehand.rs`: constant-width laser outlines and reference SVG decimal truncation.
- `crates/rsdraw-core/src/variable_freehand.rs`: native perfect-freehand 1.2.0 simulated-pressure strokes.
- `crates/rsdraw-core/src/rendering.rs`: typed requests, path data, validation, and direct freehand fill operations.
- `crates/rsdraw-core/src/canvas.rs`: elements, Cairo replay, reference colors, hit testing, and cached geometry.
- `crates/rsdraw-core/src/text.rs`: text normalization, fractional measurements, per-family line heights, and per-line alphabetic baselines.
- `crates/rsdraw-core/src/fonts.rs`, `assets/fonts/`: process-local Fontconfig registration and bundled Excalifont.
- `src/`: main `rsdraw` crate. GTK window/input (`app.rs`), scene edits/history (`model.rs`), keys, config, IPC, and asynchronous desktop capture (`screenshot.rs`, using `grim`).
- `examples/`: gallery and drawing benchmark importing `rsdraw_core`, without including private source files.
- `tests/fixtures/`, `scripts/`: independent reference data and optional development-only JavaScript oracle generators.
- `assets/vendor/`: upstream bundles used only by fixture generation, licenses, and provenance. None of the JavaScript is included in the application.

## Verification (five cores)

The checked-in `.cargo/config.toml` caps Cargo at five jobs. CPU affinity also limits internal compiler/linker and test threads. Choose five permitted CPUs if 0–4 are unavailable.

```bash
cargo fmt --all --check
taskset -c 0-4 cargo test --workspace -j 5 -- --test-threads=5
taskset -c 0-4 cargo clippy --workspace --all-targets -j 5 -- -D warnings
taskset -c 0-4 cargo build --release -j 5
taskset -c 0-4 cargo run -j 5 --example render_gallery -- /tmp/rsdraw-gallery.png
./target/release/rsdraw --config config/default.toml check-config
```

The core can be tested separately with `cargo test -p rsdraw-core -j 5`. A Wayland compositor is not required for its tests or examples. Append `--dark` to the gallery command to inspect dark-mode colors.

On a layer-shell compositor, run the native integration checks separately (GTK must run on one test thread):

```bash
taskset -c 0-4 cargo test -j 5 native_text_editor_and_empty_canvas -- --ignored --test-threads=1
taskset -c 0-4 cargo test -j 5 themed_window_keeps_clear_pixels_transparent -- --ignored --test-threads=1
# Requires grim; writes and removes a temporary screenshot.
taskset -c 0-4 cargo test -j 5 desktop_screenshot_hides_controls_and_saves_annotations -- --ignored --test-threads=1
```

These open temporary test windows. They cover editing, hidden controls, palettes, IPC validation, passthrough, and transparent pixels under an opaque GTK user theme. They do not change the running daemon. The capture test verifies that the saved PNG contains an annotation at the screen center while the F1 panel is hidden; the panel is restored afterward.

## Reference and scope

`excalidraw/` is a read-only, ignored reference checkout. Do not modify, stage, or commit it. The oracle records its exact commit. The supported rectangle, diamond, ellipse, two-point line/arrow, constant/variable freehand, and text tools follow its rendering algorithms. Triangle is an rsdraw extension. Other Excalidraw features (bound labels, elbow arrows, images, frames, scene-file interoperability) are not implemented.

The core preserves seeded operation order, including random numbers consumed by zero-amplitude offsets. Do not introduce fresh random numbers on redraw. Paths are shared across history clones and cached by geometry/seed/style. Color changes reuse cached paths. Freehand fills go directly to Cairo as closed quadratic paths converted to cubics, preserving the reference's SVG coordinate truncation.

The 314 geometry fixtures compare every operation and rendered option within 1e-9. Freehand tests compare outline points and Cairo pixels against independently generated reference paths, including duplicate samples, sharp corners, reversals, empty inputs, long traces, and multiple widths. These pixel checks compare paths through the same Cairo backend; they do not claim browser rasterization is identical. Invalid non-finite outlines are rejected before reaching Cairo.

Text uses the original Excalifont subsets, normalizes CR/CRLF and tabs, measures fractional advances, retains blank lines, and draws every line at Excalidraw's alphabetic baseline. Forty Chrome reference cases cover four font sizes, multiline text, whitespace, ligatures, accents, Greek, and Cyrillic. Width tolerance is 0.08 px for native/browser shaping precision; baseline and line-box height tolerance is 1e-9. Native antialiasing and fallback fonts can differ from the browser.

Regenerate shape/color fixtures from the actual clone:

```bash
taskset -c 0-4 env GOMAXPROCS=5 node scripts/generate_reference_fixtures.mjs
```

This optional development step extracts actual Excalidraw functions and uses pinned upstream bundles, independently of the Rust renderer. It downloads pinned esbuild through `npm exec`. No normal Cargo command needs Node or network access for these fixtures.

For browser text fixtures:

```bash
node scripts/generate_text_reference.mjs /tmp/rsdraw-text-reference.html
```

Open the generated HTML in Chrome. It loads the clone's WOFF2s with the reference unicode ranges and computes Canvas `measureText` widths and reference baselines. Once the title says `ready`, save `window.reference` as JSON to `tests/fixtures/text.json`. The JSON records the browser version and reference commit. No Rust output is used as an oracle.

## Performance

Measure fresh and cached native drawing on the same machine:

```bash
taskset -c 0-4 cargo run --release -j 5 --example benchmark_drawing
```

This is a headless Cairo benchmark, not a full compositor frame-rate measurement. Both constant and variable freehand now run in Rust. Input samples are preserved; there is no JS conversion or SVG parsing per pointer frame.
