# Development

rsdraw is a native GTK/Cairo overlay. The binary embeds QuickJS and the pinned upstream rendering libraries. No Node/npm/browser runtime or reference checkout is required to build or run it. A C compiler is needed to build QuickJS.

## Layout

- `src/canvas.rs`: element geometry, replay of upstream path operations, reference dark-mode colors, Pango text, bounds/hit testing, and color parsing.
- `src/rendering.rs`, `assets/rendering/excalidraw.js`: embedded JS engine and Excalidraw option/shape bridge.
- `assets/vendor/`: pinned upstream rendering bundles, licenses, and provenance hashes.
- `tests/fixtures/excalidraw.json`, `scripts/generate_reference_fixtures.mjs`: independent reference-generated geometry/color cases.
- `src/model.rs`: scene edits, pointer gestures, selection, text lifecycle, pan/zoom, and bounded undo/redo history.
- `src/keys.rs`: action names, key parsing, modifier normalization, conflict checks, and contextual dispatch.
- `src/config.rs`: config loading, validation, default merging, and generated help.
- `config/default.toml`: complete default settings and keybindings; embedded directly in the binary.
- `src/fonts.rs`, `assets/fonts/`: bundled Excalifont and process-local Fontconfig registration.
- `src/app.rs`, `src/style.css`: cached transparent overlay, on-demand help/color palette, native text editor, and event wiring. No permanent toolbar or property panels.
- `src/ipc.rs`: local socket commands with execution responses, timeouts, and validation errors.
- `examples/render_gallery.rs`: headless rendering example using the same Cairo/Pango renderer.

## Verification (maximum five cores)

```bash
cargo fmt --check
# CPU affinity also limits internal compiler/linker and test threads.
taskset -c 0-4 cargo test -j 5 -- --test-threads=5
taskset -c 0-4 cargo clippy -j 5 --all-targets -- -D warnings
taskset -c 0-4 cargo build -j 5
taskset -c 0-4 cargo run -j 5 --example render_gallery -- /tmp/rsdraw-gallery.png
./target/debug/rsdraw --config config/default.toml check-config
```

Choose up to five CPUs available to the current process if 0–4 are unavailable. Preserve the local Cargo job limit when building. An additional native GTK smoke test can be run on Wayland with `taskset -c 0-4 cargo test -j 5 native_text_editor_and_empty_canvas -- --ignored --test-threads=1`. It opens a temporary overlay and checks the hidden controls, text editing, color picker, IPC validation, and passthrough.

Tests cover deterministic rendering, color input, shape constraints, key matching, config inheritance/errors, scene history, cancellation, erasing, text commits, and zoom coordinates. The render example requires font libraries but no Wayland display.

For an interactive smoke test on a layer-shell compositor, start with the checked-in config. Check the uncluttered transparent canvas, draw each tool in both directions, test Shift/Alt constraints, type multiline and Unicode text, finish/re-edit text, and undo/redo/clear. Open and close help/color palettes and test toggle passthrough. `Ctrl+Q` or `rsdraw quit` stops the daemon. Existing compositor shortcuts may intercept keys before GTK receives them.

Rendering is deterministic for an element's stored seed. Do not draw using fresh randomness on every frame. Geometry/style lives on each element, so changing defaults never changes older elements unless selected explicitly. Text uses Pango shaping/fallback and regular-weight Excalifont. The paths come from the actual upstream libraries and are compared to the reference code. Cairo/Pango supply native antialiasing and font rasterization; scene file interoperability is not implemented.

## Reference checkout

`excalidraw/` is a local, read-only reference and is ignored by Git. Do not modify, stage, or commit it. The bundled font subsets were decoded from the reference's WOFF2 files into TTF for native font loading; its license is retained in `assets/fonts/OFL.txt`. See [third-party notices](../THIRD_PARTY_NOTICES.md).

## Transparent overlay regression

The window, canvas container, and drawing area have scoped transparent styles applied above GTK user-theme priority. Clearing the Cairo surface alone does not clear a GTK window background painted underneath it. Keep help/picker panel backgrounds separate from these shell rules.

Run `taskset -c 0-4 cargo test -j 5 themed_window_keeps_clear_pixels_transparent -- --ignored --test-threads=1` on Wayland to verify rendered pixels against an opaque user theme. It checks that clear pixels have zero alpha while drawn pixels stay opaque, and briefly opens a separate test window without touching the running daemon.

## Rendering parity

RoughJS 4.6.4 and perfect-freehand 1.2.0 match the versions pinned by `packages/excalidraw/package.json`. The constant-width freehand generator is bundled from the reference's `packages/laser-pointer/src` with pinned esbuild. Excalidraw's roundness, roughness reduction for small shapes, preserveVertices, dash/fill settings, arrowhead geometry, freehand sizes/easing, and SVG outline rounding are used directly. Cached paths keep previews, redraws, and undo stable. The embedded JS compatibility shim supplies legacy `RegExp.$1` captures used by RoughJS's SVG parser.

`paths_match_the_actual_excalidraw_reference` compares every path operation and generator option against the checked-in oracle fixtures, with a 1e-9 numerical tolerance for JavaScript-engine floating-point differences. The fixtures are generated by extracting and running the actual functions from the local reference, separately from the application bridge. Color tests compare the reference's inversion/hue rotation and confirm fully opaque ink pixels for each palette color on both themes.

To regenerate fixtures with the read-only clone present:

```bash
taskset -c 0-4 env GOMAXPROCS=5 node scripts/generate_reference_fixtures.mjs
```

This optional development operation uses Node and downloads pinned esbuild through `npm exec`; ordinary builds/tests use only the vendored bundles and fixtures. The generator records the reference commit. To render a dark preview, append `--dark` to the gallery example's arguments. Keep upstream bundle bytes unmodified and update the provenance hashes when intentionally updating versions.
