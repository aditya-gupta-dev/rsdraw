# rsdraw

A minimal screen annotation overlay for Linux/Wayland, built with Rust, GTK4, layer-shell, and Cairo. Draw directly over your desktop using hand-drawn shapes and Excalifont text. The canvas stays transparent and uncluttered: no toolbar, property sidebar, or persistent controls. Help and the color picker open only when requested.

Requires a Wayland compositor with `wlr-layer-shell` support, such as Hyprland or Sway. Drawings are kept in memory for the current daemon session.

## Build and run

System dependencies: a C compiler (for embedded QuickJS), GTK4 4.12+, gtk4-layer-shell, Cairo, Pango/PangoCairo, Fontconfig, and a Rust toolchain. The bundled font does not need to be installed system-wide.

```bash
cargo build --release -j 5
./target/release/rsdraw
```

Builds and verification should use at most five cores. `-j 5` caps Cargo jobs; on Linux, `taskset -c 0-4 cargo build --release -j 5` also limits compiler/linker threads to five logical CPUs (choose five permitted CPUs if your CPU affinity differs).

## Drawing

| Default shortcut | Tool |
| --- | --- |
| `P` or `7` | Freehand |
| `R` or `2` | Rectangle |
| `D` or `3` | Diamond |
| `O` or `4` | Ellipse |
| `A` or `5` | Arrow with an open head |
| `L` or `6` | Line |
| `T` or `8` | Text |
| `Shift+T` | Triangle (rsdraw extension) |
| `V` or `1` | Selection |
| `E` or `0` | Eraser |

Letters mean an unmodified key, not Shift+letter. Start drawing immediately with the freehand tool. Pick a shape shortcut and drag to draw it; tools stay active by default. Hold Shift for a square/circle or angle-constrained line; hold Alt to draw a shape from its center. Click with the text tool and type. Enter inserts a newline; Escape or Ctrl+Enter finishes. Double-click text to edit it.

`Ctrl+Z` undoes, `Ctrl+Shift+Z` / `Ctrl+Y` redoes, and `Ctrl+Delete` clears (undoable). `?` / `F1` opens help generated from your actual config. `S` opens the stroke color palette; `G` opens the fill color picker. `Ctrl+Q` exits. See [all controls](docs/CONTROLS.md).

Rendering uses the **same pinned RoughJS 4.6.4 generator** as the reference, with Excalidraw's shape options, round corners, arrowheads, and fill paths. Freehand uses the reference's constant-width laser geometry by default; variable-width strokes use its pinned perfect-freehand 1.2.0 engine. These implementations run inside the Rust binary through embedded QuickJS; Node, npm, and a browser are not required to run rsdraw. Cairo paints their original path operations. The bundled Excalifont uses the reference's 1.25 line height.

`theme = "dark"` applies Excalidraw's own dark-mode color transform, making default ink visible on dark desktops while retaining a transparent canvas. Set `theme = "light"` for light desktops. Color swatches and text use the same transform as shapes. This setting does not paint a background. Shape paths and colors are checked against fixtures generated directly from the reference checkout; final antialiasing and font rasterization are supplied by Cairo/Pango rather than a browser.

## Local configuration

On first run, rsdraw creates `$XDG_CONFIG_HOME/rsdraw/config.toml`, or `~/.config/rsdraw/config.toml` if XDG_CONFIG_HOME is unset. [config/default.toml](config/default.toml) is the complete editable default and the source used by the program.

```bash
rsdraw config-path
rsdraw check-config
rsdraw --config ./config/default.toml
# Alternatively: RSDRAW_CONFIG=/path/to/config.toml rsdraw
```

Configure shortcuts, stroke/fill color, width, roughness, and font there. Strings and lists of alternate bindings are supported; `[]` disables an action. Restart the daemon after editing. Existing local settings are preserved; see [configuration and upgrading](docs/CONFIGURATION.md) for older configs that use `F/C/X` tools or numeric color shortcuts.

## Commands

Run these from another terminal or compositor binding while the daemon is running:

```bash
rsdraw toggle                 # Drawing / desktop passthrough; drawings stay visible
rsdraw clear                  # Clear drawings (undoable)
rsdraw undo
rsdraw redo
rsdraw tool rectangle         # Also diamond, ellipse, triangle, arrow, line, freehand, text
rsdraw font Excalifont        # Multiword family names are supported
rsdraw font-size 24           # Independent of stroke width
rsdraw color '#e03131'        # Names or #RGB / #RGBA / #RRGGBB / #RRGGBBAA
rsdraw fill transparent
rsdraw width 2
rsdraw roughness 1            # 0 clean; 1 hand-drawn; 2 very rough (range 0–3)
rsdraw fill-style hachure     # hachure / cross_hatch / solid
rsdraw stroke-style solid    # solid / dashed / dotted
rsdraw theme dark            # dark / light: match the desktop behind the overlay
rsdraw roundness round       # round / sharp (round is the reference default)
rsdraw stroke-variability constant # constant / variable
rsdraw picker                # On-demand color picker
rsdraw help                  # Configured shortcuts and commands
rsdraw quit
```

Invalid commands or values report an error and leave the drawing settings unchanged. `--config` affects startup, help, and validation; IPC commands address the single running daemon.

## Hyprland bindings

```ini
bind = $mainMod, D, exec, rsdraw toggle
bind = $mainMod SHIFT, D, exec, rsdraw clear
bind = $mainMod SHIFT, Z, exec, rsdraw undo
```

The compositor handles global shortcuts while rsdraw passes input through. rsdraw's own shortcuts apply when the overlay has keyboard focus.

## Development

See [development and validation](docs/DEVELOPMENT.md) and [third-party notices](THIRD_PARTY_NOTICES.md). The local `excalidraw/` checkout is ignored and used only as a reference. No build step depends on it.
