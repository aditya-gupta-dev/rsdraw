# Configuration

[config/default.toml](../config/default.toml) is the authoritative default, embedded in the binary. Missing config files are created automatically; existing ones are never overwritten. The daemon loads config once at startup.

Path precedence:

1. `rsdraw --config PATH` (before the command).
2. `RSDRAW_CONFIG` environment variable.
3. `$XDG_CONFIG_HOME/rsdraw/config.toml`.
4. `~/.config/rsdraw/config.toml`.

`rsdraw config-path` prints the chosen path without creating it. `rsdraw check-config` validates it without opening a window (and creates defaults if the file is missing). Invalid files report actionable errors and exit; the daemon does not silently substitute other keybindings.

## Drawing settings

```toml
[drawing]
default_color = "#1e1e1e"
default_width = 2.0
font_family = "Excalifont"
font_size = 20.0
roughness = 1.0
fill_color = "transparent"
fill_style = "solid"
stroke_style = "solid"
tool_lock = true
theme = "dark"
rounded = true
stroke_variability = "constant"
```

Width accepts 0.5–50; font size accepts 6–256; roughness accepts 0–3. NaN and infinity are rejected. Supported fill styles are `hachure`, `cross_hatch`, and `solid`; stroke styles are `solid`, `dashed`, and `dotted`.

Colors accept `#RGB`, `#RGBA`, `#RRGGBB`, `#RRGGBBAA`, or names: black, white, red, green, blue, yellow, orange, pink, purple, cyan, gray/grey, darkgray/darkgrey, magenta, indigo, teal, mint, brown, transparent. Fill transparency leaves the desktop visible. The default ink and named colors use Excalidraw's palette.

`theme = "dark"` is the default for transparent annotation over a dark desktop. It uses Excalidraw's exact `invert(93%) hue-rotate(180deg)` color transform for stroke, fill, and text; alpha remains unchanged. The canonical black ink becomes light ink rather than disappearing into the desktop. Use `theme = "light"` to display the canonical light-theme palette over a light desktop. The background remains transparent in both modes. The same palette values can therefore display differently between modes, as they do in Excalidraw. Use the **Dark mode** switch in the F1 menu, or `rsdraw theme dark` / `rsdraw theme light`, to change the whole scene and menu appearance. The switch stays synchronized with IPC changes. These are session changes; edit `drawing.theme` to set the next startup theme.

`rounded = true` matches the reference's default roundness: adaptive rectangle corners (25% of the smaller side, capped at 32px), proportional diamond corners, and curved line/arrow generation. Set false for sharp geometry, or use `rsdraw roundness sharp` / `rsdraw roundness round`.

`stroke_variability = "constant"` uses the reference's default laser-outline freehand geometry with size 1.4 and streamline 0.5. `"variable"` uses perfect-freehand with the reference's pressure simulation, easing, and tuning. Pen widths use the reference's schema-2 mapping (half the corresponding shape width); text size remains independent. Set this during the session with `rsdraw stroke-variability constant` or `variable`.

Excalifont is bundled and registered only for this process through Fontconfig, with Latin, extended Latin, Greek, and Cyrillic subsets. Other scripts use installed fallback fonts through Pango. The binary writes bundled font files to `$XDG_CACHE_HOME/rsdraw/fonts` (fallback `~/.cache/rsdraw/fonts`). Other installed font families are also supported. Fonts render at regular weight; text size is independent of brush width.

`tool_lock = true` keeps drawing with the chosen tool. Set it to false for shapes/text to return to selection after completion. `Q` toggles the setting during the session. The starting tool is freehand.

## Keybindings

```toml
[keys]
tool_freehand = ["p", "7"]
tool_triangle = "Shift+t"
undo = "Ctrl+z"
quit = "Ctrl+q"
color_red = "Alt+1"
help = ["?", "F1"]
save_screenshot = "Ctrl+s"
copy = [] # Disable this action
```

A string binds one shortcut; a list binds alternatives; an empty list disables the action. Omitted actions inherit defaults. Explicit user bindings take precedence over inherited shortcuts: assigning `1` to a color removes the inherited `1` selection alias but keeps `V` for selection. Conflicting bindings between two explicitly configured actions are errors. `finish_text` and `cancel` can share Escape because they operate in different contexts.

Modifiers: `Ctrl`/`Control`, `Shift`, `Alt`, `Super`. Keys accept GDK names (such as `F1`, `Escape`, `Return`, `BackSpace`, `Left`, `space`, `plus`) and the punctuation aliases in the default config. Use `plus` for the `+` key; the plus character separates modifiers. Capitalizing a letter does not add Shift: write `Shift+t` explicitly. Caps Lock does not interfere with letter matching.

All configurable action names are listed in [the default file](../config/default.toml). CLI and on-screen help show the actual effective bindings. Shape modifiers (Shift/Alt), Space+drag, middle drag, wheel behavior, and native text-editor shortcuts are pointer/editor conventions rather than action bindings.

## Screenshots

`Ctrl+S` saves a PNG of the desktop and annotations using `grim`. Install `grim` and use a compositor with its supported screen-capture protocol. Files go to the XDG Pictures directory under `Screenshots`, falling back to `~/Pictures/Screenshots`. The screenshot covers the overlay's monitor when its output name is available, otherwise all outputs. Capture errors are shown in the overlay. Existing files are preserved by choosing an unused filename.

The `[keys]` entry `save_screenshot` can be changed or disabled like any other action. Older configs inherit `Ctrl+S` automatically unless an explicit binding already uses it. Saving works during text editing and with F1 open.

## Upgrading an older config

Older configurations remain supported: custom font/color settings and shortcuts are retained. Legacy numeric color shortcuts override inherited numeric tool aliases. Previous defaults like `tool_freehand = "f"`, `tool_circle = "c"`, and `tool_text = "x"` keep working when explicitly present.

To adopt all new defaults, back up your file and replace it with [config/default.toml](../config/default.toml), then restart rsdraw. To adopt only the font and tool shortcuts, edit those entries or remove them so the defaults apply. In particular, a retained `font_family = "sans-serif"` continues to use that font; switch it to `Excalifont` for handwritten text.
