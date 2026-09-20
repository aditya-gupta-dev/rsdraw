# rsdraw (Under Development)

Screen annotation tool exclusively for Linux. Built for Wayland (Hyprland / Sway / wlroots) using Rust + GTK4 + `wlr-layer-shell` + Cairo.

> **Note**: Linux only. Requires Wayland and a compositor implementing `wlr-layer-shell`.

## Build

```bash
cargo build --release
# Binary: target/release/rsdraw
```

## Usage

```bash
rsdraw              # Start overlay daemon
rsdraw help         # Show all keybindings and commands
rsdraw toggle       # Toggle draw / passthrough mode
rsdraw clear        # Clear all drawings
rsdraw undo         # Undo last stroke / shape / text
rsdraw tool <name>  # Switch tool (freehand/rectangle/circle/triangle/arrow/text)
rsdraw font <name>  # Set font family (e.g. monospace, "JetBrains Mono")
rsdraw color red    # Set color (red/green/blue/yellow/white/orange/cyan/pink/#hex)
rsdraw width 5      # Set brush width / text size (1–50)
rsdraw quit         # Exit daemon
```

## Tools

| Key | Tool | How to use |
|---|---|---|
| `f` | Freehand | Click & drag to draw freehand strokes |
| `r` | Rectangle | Click & drag from corner to corner |
| `c` | Circle | Click & drag bounding box for circle / ellipse |
| `t` | Triangle | Click & drag bounding box for isosceles triangle |
| `a` | Arrow | Click & drag start to tip with filled arrowhead |
| `x` | Text | Click on screen and type. Press `Enter` to commit, `Esc` to cancel, `Backspace` to delete |

## Configuration

Config lives at `~/.config/rsdraw/config.toml`:

```toml
[drawing]
default_color = "red"
default_width = 3.0
font_family   = "sans-serif" # e.g. "sans-serif", "monospace", "JetBrains Mono", "Fira Code"

[keys]
# Global
quit           = "Ctrl+x"
undo           = "Ctrl+z"
help           = "F1"

# Tools
tool_freehand  = "f"
tool_rectangle = "r"
tool_circle    = "c"
tool_triangle  = "t"
tool_arrow     = "a"
tool_text      = "x"

# Colors
color_red      = "1"
color_green    = "2"
color_blue     = "3"
color_yellow   = "4"
color_white    = "5"
color_orange   = "6"

# Brush / text size
brush_increase = "equal"
brush_decrease = "minus"
```

## Hyprland keybindings

Add to `~/.config/hypr/hyprland.conf`:

```ini
bind = $mainMod, D, exec, rsdraw toggle
bind = $mainMod SHIFT, D, exec, rsdraw clear
bind = $mainMod SHIFT, Z, exec, rsdraw undo
```

## Dependencies (Linux)

- GTK4 (`libgtk-4-dev`)
- gtk4-layer-shell (`libgtk4-layer-shell-dev`)
- Cairo (system library)
- Wayland compositor supporting `wlr-layer-shell` (Hyprland, Sway, etc.)
