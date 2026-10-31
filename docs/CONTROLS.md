# Controls

The default drawing surface has no permanent UI. Use shortcuts, local config, or IPC commands. Help (`?` / `F1`) and stroke/fill color palettes (`S` / `G`) are available on demand. Keybinding descriptions in CLI and on-screen help come from the effective local config.

## Tools

| Action | Default |
| --- | --- |
| Selection | `V`, `1` |
| Hand / pan | `H` |
| Rectangle | `R`, `2` |
| Diamond | `D`, `3` |
| Ellipse | `O`, `4` |
| Arrow | `A`, `5` |
| Line | `L`, `6` |
| Freehand | `P`, `7` |
| Text | `T`, `8` |
| Eraser | `E`, `0` |
| Triangle | `Shift+T` |
| Keep tool active | `Q` |

Press letter shortcuts without Shift unless shown. Tools stay active by default, matching the minimal overlay workflow. `Q` toggles this: when unlocked, completed shapes/text return to selection. Freehand and eraser remain active.

## Pointer and text input

- Drag to draw a shape, line, arrow, or freehand stroke.
- Hold Shift to constrain rectangles/ellipses/triangles/diamonds to equal dimensions or snap line/arrow angles to 15° increments.
- Hold Alt while drawing a shape to draw from its center. Shift and Alt can be combined.
- Select with `V`: click an outline (or the inside of a filled shape/text), Shift+click to add/remove, or drag a selection rectangle around entire elements.
- Drag a selection to move it. Shift constrains movement to one axis; Alt+drag duplicates. Drag the lower-right selection handle to resize; Shift preserves the selection's aspect ratio. Text scales uniformly.
- Drag with the eraser to remove touched elements. One drag is one undoable change.
- `H`, Space+drag, or middle-button drag pans. Wheel scroll pans; Ctrl+wheel zooms around the pointer.
- `T`, then click, starts native multiline text input. Enter/Shift+Enter inserts a newline. Escape/Ctrl+Enter commits. Click elsewhere to finish. Double-click existing text, or select it and press Enter, to edit.
- While editing, native cursor movement, selection, clipboard shortcuts, Unicode, and input methods belong to the text editor. Tool/color shortcuts cannot intercept typed letters.

## Editing

| Action | Default |
| --- | --- |
| Undo | `Ctrl+Z` |
| Redo | `Ctrl+Shift+Z`, `Ctrl+Y` |
| Delete selection | `Delete`, `Backspace` |
| Clear canvas | `Ctrl+Delete` |
| Save desktop and annotations as a screenshot | `Ctrl+S` |
| Select all | `Ctrl+A` |
| Duplicate | `Ctrl+D` |
| Copy / cut / paste elements in this session | `Ctrl+C`, `Ctrl+X`, `Ctrl+V` |
| Edit selected text | `Enter` |
| Cancel drag / deselect / selection tool | `Escape` |
| Move selection | Arrow keys; Shift moves 10 pixels |
| Bring forward / send backward | `Ctrl+]`, `Ctrl+[` |
| Bring to front / send to back | `Ctrl+Shift+]`, `Ctrl+Shift+[` |

Undo/redo records complete edits, including eraser drags, resizing, styling, and clearing. History retains up to 100 scene changes. Cancelling an in-progress drag restores the scene. Copy/paste of shapes uses an internal session clipboard; when that is empty, Ctrl+V pastes system clipboard text as an annotation. During text editing, clipboard operations use the system clipboard normally.

## Appearance and navigation

| Action | Default |
| --- | --- |
| Stroke color palette | `S` |
| Fill color palette | `G` |
| Red / green / blue / yellow / white / orange | `Alt+1` through `Alt+6` |
| Stroke width decrease / increase | `[`, `]` |
| Font size decrease / increase | `Ctrl+Shift+,`, `Ctrl+Shift+.` |
| Zoom in / out / reset | `Ctrl++` or `Ctrl+=`, `Ctrl+-`, `Ctrl+0` |
| Zoom to fit | `Shift+1` |
| Help | `?`, `F1` |
| Quit | `Ctrl+Q` |

Choose `theme = "dark"` (default) for dark desktops, or `theme = "light"` for light ones. These use Excalidraw's actual color palettes/transformation, without painting a canvas background. The **Dark mode** switch in F1 changes the drawing colors and menu panels together. You can also use `rsdraw theme dark` or `rsdraw theme light`. Session changes do not rewrite `drawing.theme` in the config.

Color and size commands affect the active defaults and selected elements. Stroke width and font size are independent. Set roughness, font family, fills, and stroke styles in the [config](CONFIGURATION.md) or through the [IPC commands](../README.md#commands).

The **Sloppiness** slider at the top of the `F1` / `?` menu adjusts roughness from `0` (clean) to `3` (very rough), in steps of `0.1`. Changes apply immediately to drawing defaults and selected shapes; text and freehand appearance are unaffected. Drag the slider or use arrow keys, then close the menu with `Escape` or `F1`. Changes to selected elements can be undone after closing the menu. The slider follows the current session setting, including updates made with `rsdraw roughness`; it does not rewrite your config.

`Ctrl+S` saves a PNG to your Pictures/Screenshots folder, with a timestamp and no overwriting. It includes the desktop beneath the annotations, hides menus and selection handles, and commits active text first. The shortcut also works from F1 and the text editor. Saving requires `grim`; an on-screen message reports the saved path or error.

The drawing shortcuts follow Excalidraw for the tools implemented here. Triangle and quick color shortcuts are rsdraw extensions. This overlay focuses on basic annotations, and does not provide the web editor's image/frame tools, collaboration, scene files, libraries, grouping, rotation, or connector binding.
