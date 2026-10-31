use crate::{
    canvas::*,
    config::Config,
    ipc,
    keys::{Action, Bindings},
    model::{Drag, Scene},
};
use gtk4::prelude::*;
use gtk4::{
    gdk, glib, Align, Application, ApplicationWindow, Box as GtkBox, Button, DrawingArea, Entry,
    EventControllerKey, EventControllerScroll, GestureClick, GestureDrag, Label, Orientation,
    Overlay, ScrolledWindow, TextView,
};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

struct State {
    scene: Scene,
    help: bool,
    active: bool,
    palette_target: Option<bool>,
}
#[derive(Clone)]
struct Ui {
    state: Rc<RefCell<State>>,
    canvas: DrawingArea,
    editor: TextView,
    editor_seed: Rc<Cell<Option<u64>>>,
    help: ScrolledWindow,
    palette: GtkBox,
    palette_title: Label,
    color_entry: Entry,
    color_swatches: Rc<RefCell<Vec<(Color, Label)>>>,
    window: ApplicationWindow,
}
impl Ui {
    fn refresh(&self) {
        let s = self.state.borrow();
        let scene = &s.scene;
        self.help.set_visible(s.active && s.help);
        self.palette
            .set_visible(s.active && s.palette_target.is_some());
        if self.palette.is_visible() {
            for (color, label) in self.color_swatches.borrow().iter() {
                let swatch = display_color(*color, scene.style.theme);
                let attrs = gtk4::pango::AttrList::new();
                attrs.insert(gtk4::pango::AttrColor::new_foreground(
                    (swatch.0 * 65535.) as u16,
                    (swatch.1 * 65535.) as u16,
                    (swatch.2 * 65535.) as u16,
                ));
                label.set_attributes(Some(&attrs));
            }
        }
        self.canvas
            .set_cursor_from_name(Some(if scene.space_down || scene.tool == Tool::Hand {
                "grab"
            } else if scene.tool == Tool::Text {
                "text"
            } else if scene.tool == Tool::Selection {
                "default"
            } else {
                "crosshair"
            }));
        if let Some(e) = &scene.editing {
            if let Geometry::Text { pos, text } = &e.geometry {
                if self.editor_seed.get() != Some(e.seed) {
                    self.editor_seed.set(Some(e.seed));
                    self.editor.buffer().set_text(text);
                    self.editor.set_visible(true);
                    self.editor.grab_focus();
                    let mut end = self.editor.buffer().end_iter();
                    self.editor.buffer().place_cursor(&end);
                    self.editor.scroll_to_iter(&mut end, 0., false, 0., 0.);
                }
                let p = scene.screen(*pos);
                self.editor.set_margin_start(p.0.max(0.) as i32);
                self.editor.set_margin_top(p.1.max(0.) as i32);
                let mut font = gtk4::pango::FontDescription::from_string(&e.style.font);
                font.set_absolute_size(e.style.font_size * scene.zoom * gtk4::pango::SCALE as f64);
                // TextView uses a tag so IME, selection and caret remain native.
                let buffer = self.editor.buffer();
                let table = buffer.tag_table();
                let tag = table.lookup("drawing-style").unwrap_or_else(|| {
                    let tag = gtk4::TextTag::new(Some("drawing-style"));
                    table.add(&tag);
                    tag
                });
                tag.set_font_desc(Some(&font));
                let ink = display_color(e.style.color, e.style.theme);
                tag.set_foreground_rgba(Some(&gdk::RGBA::new(
                    ink.0 as f32,
                    ink.1 as f32,
                    ink.2 as f32,
                    ink.3 as f32,
                )));
                let metrics = self.editor.pango_context().metrics(Some(&font), None);
                let natural_height =
                    (metrics.ascent() + metrics.descent()) as f64 / gtk4::pango::SCALE as f64;
                tag.set_line_height(
                    (e.style.font_size
                        * scene.zoom
                        * rsdraw_core::text::metrics(&e.style.font).line_height
                        / natural_height.max(1.)) as f32,
                );
                tag.set_pixels_below_lines(0);
                buffer.apply_tag(&tag, &buffer.start_iter(), &buffer.end_iter());
            }
        } else if self.editor.is_visible() {
            self.editor_seed.set(None);
            self.editor.set_visible(false);
            self.canvas.grab_focus();
        }
        drop(s);
        self.canvas.queue_draw();
    }
    fn action(&self, a: Action) {
        if a == Action::Quit {
            self.window.close();
            return;
        }
        let mut s = self.state.borrow_mut();
        if a == Action::Cancel && (s.palette_target.is_some() || s.help) {
            s.palette_target = None;
            s.help = false;
            drop(s);
            self.refresh();
            self.canvas.grab_focus();
            return;
        }
        if a == Action::Help {
            s.palette_target = None;
            s.help = !s.help;
            drop(s);
            self.refresh();
            return;
        }
        if matches!(a, Action::Picker | Action::FillPicker) {
            drop(s);
            self.choose_color(a == Action::FillPicker);
            return;
        }
        let scene = &mut s.scene;
        if scene.drag.is_some() && a != Action::Cancel {
            scene.cancel_drag();
        }
        use Action::*;
        let tool = match a {
            ToolSelection => Some(Tool::Selection),
            ToolHand => Some(Tool::Hand),
            ToolRectangle => Some(Tool::Rectangle),
            ToolDiamond => Some(Tool::Diamond),
            ToolCircle => Some(Tool::Circle),
            ToolArrow => Some(Tool::Arrow),
            ToolLine => Some(Tool::Line),
            ToolFreehand => Some(Tool::Freehand),
            ToolText => Some(Tool::Text),
            ToolEraser => Some(Tool::Eraser),
            ToolTriangle => Some(Tool::Triangle),
            _ => None,
        };
        if let Some(tool) = tool {
            scene.set_tool(tool);
        } else {
            match a {
                FinishText => scene.commit_text(),
                Cancel => {
                    scene.cancel_drag();
                    scene.selected.clear();
                    scene.tool = Tool::Selection;
                    s.help = false;
                }
                LockTool => scene.tool_lock = !scene.tool_lock,
                Undo => scene.undo(),
                Redo => scene.redo(),
                Delete => scene.delete(),
                Clear => scene.clear(),
                SelectAll => {
                    scene.selected = (0..scene.elements.len()).collect();
                    scene.tool = Tool::Selection;
                }
                Duplicate => scene.duplicate((10., 10.)),
                Copy => scene.copy(),
                Cut => {
                    scene.copy();
                    scene.delete();
                }
                Paste => {
                    if !scene.clipboard.is_empty() {
                        scene.paste();
                    } else {
                        drop(s);
                        self.paste_text();
                        return;
                    }
                }
                EditText => {
                    if scene.selected.len() == 1 {
                        let i = *scene.selected.first().unwrap();
                        if matches!(scene.elements[i].geometry, Geometry::Text { .. }) {
                            scene.begin_text((0., 0.), Some(i));
                        }
                    }
                }
                BrushIncrease => scene.restyle(|s| s.width = (s.width + 1.).min(50.)),
                BrushDecrease => scene.restyle(|s| s.width = (s.width - 1.).max(0.5)),
                FontIncrease => {
                    scene.restyle(|s| s.font_size = (s.font_size * 1.1).round().min(256.))
                }
                FontDecrease => {
                    scene.restyle(|s| s.font_size = (s.font_size / 1.1).round().max(6.))
                }
                ColorRed | ColorGreen | ColorBlue | ColorYellow | ColorWhite | ColorOrange => {
                    let name = match a {
                        ColorRed => "red",
                        ColorGreen => "green",
                        ColorBlue => "blue",
                        ColorYellow => "yellow",
                        ColorWhite => "white",
                        _ => "orange",
                    };
                    let c = parse_color(name).unwrap();
                    scene.restyle(|s| s.color = c);
                }
                ZoomIn | ZoomOut | ZoomReset => {
                    let p = (
                        self.canvas.width() as f64 / 2.,
                        self.canvas.height() as f64 / 2.,
                    );
                    let factor = if a == ZoomIn {
                        1.2
                    } else if a == ZoomOut {
                        1. / 1.2
                    } else {
                        1. / scene.zoom
                    };
                    scene.zoom_at(factor, p);
                }
                ZoomFit => scene.zoom_fit(self.canvas.width() as f64, self.canvas.height() as f64),
                NudgeLeft => scene.nudge(-1., 0.),
                NudgeRight => scene.nudge(1., 0.),
                NudgeUp => scene.nudge(0., -1.),
                NudgeDown => scene.nudge(0., 1.),
                NudgeLeftFast => scene.nudge(-10., 0.),
                NudgeRightFast => scene.nudge(10., 0.),
                NudgeUpFast => scene.nudge(0., -10.),
                NudgeDownFast => scene.nudge(0., 10.),
                BringForward | SendBackward | BringFront | SendBack => scene.reorder(a),
                _ => {}
            }
        }
        drop(s);
        self.refresh();
    }
    fn choose_color(&self, fill: bool) {
        let mut s = self.state.borrow_mut();
        s.scene.commit_text();
        s.palette_target = if s.palette_target == Some(fill) {
            None
        } else {
            Some(fill)
        };
        s.help = false;
        let color = if fill {
            s.scene.style.fill
        } else {
            s.scene.style.color
        };
        drop(s);
        self.palette_title
            .set_text(if fill { "Fill color" } else { "Stroke color" });
        self.color_entry.set_text(&format!(
            "#{:02x}{:02x}{:02x}{:02x}",
            (color.0 * 255.).round() as u8,
            (color.1 * 255.).round() as u8,
            (color.2 * 255.).round() as u8,
            (color.3 * 255.).round() as u8
        ));
        self.refresh();
        self.color_entry.grab_focus();
    }
    fn apply_color(&self, color: Color) {
        let mut s = self.state.borrow_mut();
        if let Some(fill) = s.palette_target.take() {
            s.scene.restyle(|style| {
                if fill {
                    style.fill = color;
                } else {
                    style.color = color;
                }
            });
        }
        drop(s);
        self.refresh();
        self.canvas.grab_focus();
    }
    fn paste_text(&self) {
        let ui = self.clone();
        self.window
            .clipboard()
            .read_text_async(None::<&gtk4::gio::Cancellable>, move |result| {
                if let Ok(Some(text)) = result {
                    let mut s = ui.state.borrow_mut();
                    if let Some(e) = &mut s.scene.editing {
                        if let Geometry::Text { text: current, .. } = &mut e.geometry {
                            current.push_str(&text);
                        }
                    } else {
                        let p = s.scene.world((
                            ui.canvas.width() as f64 / 2.,
                            ui.canvas.height() as f64 / 2.,
                        ));
                        s.scene.begin_text(p, None);
                        if let Some(Element {
                            geometry: Geometry::Text { text: current, .. },
                            ..
                        }) = &mut s.scene.editing
                        {
                            *current = text.to_string();
                        }
                        s.scene.commit_text();
                    }
                    drop(s);
                    ui.refresh();
                }
            });
    }
}

#[derive(Default)]
struct BackingCache {
    elements: Vec<Element>,
    edit_index: Option<usize>,
    pan: Point,
    zoom: f64,
    size: (i32, i32, i32),
    surface: Option<cairo::ImageSurface>,
}
impl BackingCache {
    fn paint(&mut self, cr: &cairo::Context, scene: &Scene, w: i32, h: i32, scale: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        if self.surface.is_none()
            || self.size != (w, h, scale)
            || self.elements != scene.elements
            || self.edit_index != scene.edit_index
            || self.pan != scene.pan
            || self.zoom != scene.zoom
        {
            let Ok(surface) =
                cairo::ImageSurface::create(cairo::Format::ARgb32, w * scale, h * scale)
            else {
                return;
            };
            surface.set_device_scale(scale as f64, scale as f64);
            if let Ok(context) = cairo::Context::new(&surface) {
                context.translate(scene.pan.0, scene.pan.1);
                context.scale(scene.zoom, scene.zoom);
                for (i, e) in scene.elements.iter().enumerate() {
                    if scene.edit_index != Some(i) {
                        e.draw(&context);
                    }
                }
            }
            self.elements.clone_from(&scene.elements);
            self.edit_index = scene.edit_index;
            self.pan = scene.pan;
            self.zoom = scene.zoom;
            self.size = (w, h, scale);
            self.surface = Some(surface);
        }
        if let Some(surface) = &self.surface {
            let _ = cr.set_source_surface(surface, 0., 0.);
            let _ = cr.paint();
        }
    }
}

pub fn run(config: Config) {
    if let Err(e) = crate::fonts::register() {
        eprintln!("Excalifont unavailable: {e}");
    }
    let app = Application::builder()
        .application_id("io.github.rsdraw")
        .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
        .build();
    app.connect_activate(move |app| {
        let _ = build_ui(app, &config);
    });
    app.run_with_args::<String>(&[]);
}
fn caption(parent: &GtkBox, text: &str) {
    let l = Label::new(Some(text));
    l.set_halign(Align::Start);
    l.add_css_class("caption");
    parent.append(&l);
}
fn install_style(window: &ApplicationWindow) {
    window.add_css_class("rsdraw-overlay");
    let css = gtk4::CssProvider::new();
    css.load_from_string(include_str!("style.css"));
    gtk4::style_context_add_provider_for_display(
        &gdk::Display::default().expect("no display"),
        &css,
        gtk4::STYLE_PROVIDER_PRIORITY_USER + 1,
    );
}

fn build_ui(app: &Application, config: &Config) -> Option<Ui> {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("rsdraw")
        .build();
    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    for edge in [Edge::Top, Edge::Bottom, Edge::Left, Edge::Right] {
        window.set_anchor(edge, true);
    }
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_decorated(false);
    install_style(&window);
    let state = Rc::new(RefCell::new(State {
        scene: Scene::new(config),
        help: false,
        active: true,
        palette_target: None,
    }));
    let bindings = Rc::new(Bindings::compile(&config.keys).expect("validated config"));
    let root = Overlay::new();
    root.add_css_class("rsdraw-canvas-root");
    let canvas = DrawingArea::new();
    canvas.add_css_class("rsdraw-canvas");
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    canvas.set_focusable(true);
    root.set_child(Some(&canvas));
    let editor = TextView::new();
    editor.set_visible(false);
    editor.set_halign(Align::Start);
    editor.set_valign(Align::Start);
    editor.set_width_request(240);
    editor.set_height_request(45);
    editor.add_css_class("drawing-editor");
    editor.set_wrap_mode(gtk4::WrapMode::None);
    editor.set_accepts_tab(true);
    editor.buffer().set_enable_undo(true);
    root.add_overlay(&editor);
    let help = ScrolledWindow::new();
    help.set_halign(Align::Center);
    help.set_valign(Align::Center);
    help.set_width_request(650);
    help.set_height_request(550);
    help.add_css_class("panel");
    help.set_visible(false);
    let help_content = GtkBox::new(Orientation::Vertical, 10);
    caption(&help_content, "Keyboard shortcuts");
    for (keys, label) in config.help_lines() {
        let row = GtkBox::new(Orientation::Horizontal, 12);
        let name = Label::new(Some(&label));
        name.set_hexpand(true);
        name.set_halign(Align::Start);
        row.append(&name);
        let key = Label::new(Some(&keys));
        key.add_css_class("shortcut");
        row.append(&key);
        help_content.append(&row);
    }
    caption(
        &help_content,
        "Shift: constrain / multi-select  ·  Alt: center / duplicate",
    );
    caption(
        &help_content,
        "Space or middle drag: pan  ·  Ctrl+wheel: zoom",
    );
    caption(
        &help_content,
        "Text: Enter for newline; Escape / Ctrl+Enter to finish",
    );
    help.set_child(Some(&help_content));
    root.add_overlay(&help);
    let palette = GtkBox::new(Orientation::Vertical, 12);
    palette.add_css_class("panel");
    palette.set_halign(Align::Center);
    palette.set_valign(Align::Center);
    palette.set_visible(false);
    let palette_title = Label::new(Some("Stroke color"));
    palette.append(&palette_title);
    let swatches = gtk4::Grid::new();
    swatches.set_row_spacing(6);
    swatches.set_column_spacing(6);
    palette.append(&swatches);
    let color_entry = Entry::new();
    color_entry.set_placeholder_text(Some("Color name or #RRGGBBAA; Enter to apply"));
    color_entry.set_width_chars(38);
    palette.append(&color_entry);
    caption(&palette, "Escape closes · use transparent for no fill");
    root.add_overlay(&palette);
    let ui = Ui {
        state: state.clone(),
        canvas: canvas.clone(),
        editor: editor.clone(),
        editor_seed: Rc::new(Cell::new(None)),
        help,
        palette,
        palette_title,
        color_entry: color_entry.clone(),
        color_swatches: Rc::new(RefCell::new(Vec::new())),
        window: window.clone(),
    };
    for (i, name) in [
        "black",
        "red",
        "green",
        "blue",
        "yellow",
        "orange",
        "purple",
        "pink",
        "white",
        "gray",
        "cyan",
        "transparent",
    ]
    .into_iter()
    .enumerate()
    {
        let color = parse_color(name).unwrap();
        let swatch = display_color(color, config.drawing.theme);
        let b = Button::with_label(if name == "transparent" { "∅" } else { "●" });
        b.set_tooltip_text(Some(name));
        b.set_focus_on_click(false);
        if let Some(label) = b.child().and_downcast::<Label>() {
            let attr = gtk4::pango::AttrList::new();
            attr.insert(gtk4::pango::AttrColor::new_foreground(
                (swatch.0 * 65535.) as u16,
                (swatch.1 * 65535.) as u16,
                (swatch.2 * 65535.) as u16,
            ));
            label.set_attributes(Some(&attr));
            ui.color_swatches.borrow_mut().push((color, label));
        }
        let ui = ui.clone();
        b.connect_clicked(move |_| ui.apply_color(color));
        swatches.attach(&b, (i % 6) as i32, (i / 6) as i32, 1, 1);
    }
    {
        let ui = ui.clone();
        color_entry.connect_activate(move |entry| {
            if let Some(color) = parse_color(&entry.text()) {
                entry.remove_css_class("error");
                ui.apply_color(color);
            } else {
                entry.add_css_class("error");
                entry.set_tooltip_text(Some("Use a color name or #RGB, #RGBA, #RRGGBB, #RRGGBBAA"));
            }
        });
    }
    {
        let state = state.clone();
        let cache = RefCell::new(BackingCache::default());
        canvas.set_draw_func(move |widget, cr, w, h| {
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_rgba(0., 0., 0., 0.);
            let _ = cr.paint();
            cr.set_operator(cairo::Operator::Over);
            let s = state.borrow();
            let scene = &s.scene;
            cache
                .borrow_mut()
                .paint(cr, scene, w, h, widget.scale_factor());
            let _ = cr.save();
            cr.translate(scene.pan.0, scene.pan.1);
            cr.scale(scene.zoom, scene.zoom);
            if let Some(e) = &scene.preview {
                e.draw(cr);
            }
            if let Some(b) = scene.selection_bounds() {
                cr.set_source_rgb(0.41, 0.33, 0.9);
                cr.set_line_width(1. / scene.zoom);
                cr.set_dash(&[4. / scene.zoom, 3. / scene.zoom], 0.);
                cr.rectangle(
                    b.x - 4. / scene.zoom,
                    b.y - 4. / scene.zoom,
                    b.w + 8. / scene.zoom,
                    b.h + 8. / scene.zoom,
                );
                let _ = cr.stroke();
                cr.set_dash(&[], 0.);
                let r = 4. / scene.zoom;
                cr.rectangle(b.x + b.w - r, b.y + b.h - r, 2. * r, 2. * r);
                cr.set_source_rgb(1., 1., 1.);
                let _ = cr.fill_preserve();
                cr.set_source_rgb(0.41, 0.33, 0.9);
                let _ = cr.stroke();
            }
            if let Some(Drag::Marquee { origin, end, .. }) = &scene.drag {
                let b = Bounds::between(*origin, *end);
                cr.rectangle(b.x, b.y, b.w, b.h);
                cr.set_source_rgba(0.41, 0.33, 0.9, 0.12);
                let _ = cr.fill_preserve();
                cr.set_source_rgba(0.41, 0.33, 0.9, 0.8);
                cr.set_line_width(1. / scene.zoom);
                let _ = cr.stroke();
            }
            let _ = cr.restore();
        });
    }
    {
        let ui = ui.clone();
        editor.buffer().connect_changed(move |buffer| {
            // refresh() can set the buffer while it holds an immutable state borrow.
            let updated = if let Ok(mut s) = ui.state.try_borrow_mut() {
                if let Some(Element {
                    geometry: Geometry::Text { text, .. },
                    ..
                }) = &mut s.scene.editing
                {
                    *text = buffer
                        .text(&buffer.start_iter(), &buffer.end_iter(), true)
                        .to_string();
                }
                true
            } else {
                false
            };
            if updated {
                ui.refresh();
            }
        });
    }
    let drag = GestureDrag::new();
    drag.set_button(0);
    let origin = Rc::new(RefCell::new((0., 0.)));
    {
        let ui = ui.clone();
        let origin = origin.clone();
        drag.connect_drag_begin(move |g, x, y| {
            if !matches!(g.current_button(), 1 | 2) {
                return;
            }
            ui.canvas.grab_focus();
            *origin.borrow_mut() = (x, y);
            let mods = g.current_event_state();
            ui.state.borrow_mut().scene.pointer_down(
                (x, y),
                mods.contains(gdk::ModifierType::SHIFT_MASK),
                mods.contains(gdk::ModifierType::ALT_MASK),
                g.current_button() == 2,
            );
            ui.refresh();
        });
    }
    {
        let ui = ui.clone();
        let origin = origin.clone();
        drag.connect_drag_update(move |g, dx, dy| {
            let (x, y) = *origin.borrow();
            let mods = g.current_event_state();
            ui.state.borrow_mut().scene.pointer_move(
                (x + dx, y + dy),
                mods.contains(gdk::ModifierType::SHIFT_MASK),
                mods.contains(gdk::ModifierType::ALT_MASK),
            );
            // Pointer motion only changes canvas geometry. Let GTK coalesce
            // redraws instead of restyling widgets/replacing the cursor per event.
            ui.canvas.queue_draw();
        });
    }
    {
        let ui = ui.clone();
        let origin = origin.clone();
        drag.connect_drag_end(move |g, dx, dy| {
            let (x, y) = *origin.borrow();
            let mods = g.current_event_state();
            ui.state.borrow_mut().scene.pointer_up(
                (x + dx, y + dy),
                mods.contains(gdk::ModifierType::SHIFT_MASK),
                mods.contains(gdk::ModifierType::ALT_MASK),
            );
            ui.refresh();
        });
    }
    {
        let ui = ui.clone();
        drag.connect_cancel(move |_, _| {
            ui.state.borrow_mut().scene.cancel_drag();
            ui.refresh();
        });
    }
    canvas.add_controller(drag);
    let click = GestureClick::new();
    click.set_button(1);
    {
        let ui = ui.clone();
        click.connect_pressed(move |g, n, x, y| {
            if n == 2 {
                let mut s = ui.state.borrow_mut();
                let p = s.scene.world((x, y));
                let hit = s.scene.hit(p);
                let text =
                    hit.filter(|&i| matches!(s.scene.elements[i].geometry, Geometry::Text { .. }));
                if text.is_some() || hit.is_none() {
                    s.scene.cancel_drag();
                    s.scene.begin_text(p, text);
                    g.set_state(gtk4::EventSequenceState::Claimed);
                }
                drop(s);
                ui.refresh();
            }
        });
    }
    canvas.add_controller(click);
    let key = EventControllerKey::new();
    key.set_propagation_phase(gtk4::PropagationPhase::Capture);
    {
        let ui = ui.clone();
        let bindings = bindings.clone();
        key.connect_key_pressed(move |_, key, _, mods| {
            if ui.state.borrow().palette_target.is_some() {
                if bindings.action(key, mods, false) == Some(Action::Cancel) {
                    ui.action(Action::Cancel);
                    return glib::Propagation::Stop;
                }
                return glib::Propagation::Proceed;
            }
            let editing = ui.state.borrow().scene.editing.is_some();
            if editing {
                if bindings.action(key, mods, true) == Some(Action::FinishText) {
                    ui.action(Action::FinishText);
                    return glib::Propagation::Stop;
                }
                return glib::Propagation::Proceed;
            }
            if key == gdk::Key::space
                && !mods.intersects(
                    gdk::ModifierType::CONTROL_MASK
                        | gdk::ModifierType::ALT_MASK
                        | gdk::ModifierType::SUPER_MASK,
                )
            {
                ui.state.borrow_mut().scene.space_down = true;
                ui.refresh();
                return glib::Propagation::Stop;
            }
            if let Some(a) = bindings.action(key, mods, false) {
                ui.action(a);
                glib::Propagation::Stop
            } else {
                glib::Propagation::Proceed
            }
        });
    }
    {
        let ui = ui.clone();
        key.connect_key_released(move |_, key, _, _| {
            if key == gdk::Key::space {
                ui.state.borrow_mut().scene.space_down = false;
                ui.refresh();
            }
        });
    }
    window.add_controller(key);
    let scroll = EventControllerScroll::new(gtk4::EventControllerScrollFlags::BOTH_AXES);
    {
        let ui = ui.clone();
        scroll.connect_scroll(move |ctrl, dx, dy| {
            let mut s = ui.state.borrow_mut();
            if ctrl
                .current_event_state()
                .contains(gdk::ModifierType::CONTROL_MASK)
            {
                let p = ctrl.current_event().and_then(|e| e.position()).unwrap_or((
                    ui.canvas.width() as f64 / 2.,
                    ui.canvas.height() as f64 / 2.,
                ));
                s.scene.zoom_at((-dy * 0.12).exp(), p);
            } else {
                s.scene.pan.0 -= dx * 30.;
                s.scene.pan.1 -= dy * 30.;
            }
            drop(s);
            ui.refresh();
            glib::Propagation::Stop
        });
    }
    canvas.add_controller(scroll);
    {
        let ui = ui.clone();
        window.connect_is_active_notify(move |w| {
            if !w.is_active() {
                let mut s = ui.state.borrow_mut();
                s.scene.space_down = false;
                s.scene.cancel_drag();
                drop(s);
                ui.refresh();
            }
        });
    }
    window.set_child(Some(&root));
    ui.refresh();
    window.present();
    canvas.grab_focus();
    let (sender, receiver) = std::sync::mpsc::channel::<ipc::Request>();
    if let Err(e) = ipc::start_server(sender) {
        eprintln!("{e}");
        window.close();
        return None;
    }
    app.connect_shutdown(|_| {
        let _ = std::fs::remove_file(ipc::socket_path());
    });
    {
        let ui = ui.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(30), move || {
            while let Ok(request) = receiver.try_recv() {
                let response = handle_ipc(&request.command, &ui);
                let _ = request.response.send(response);
            }
            if ui.window.is_visible() {
                glib::ControlFlow::Continue
            } else {
                glib::ControlFlow::Break
            }
        });
    }
    Some(ui)
}

fn handle_ipc(command: &str, ui: &Ui) -> String {
    let (name, arg) = command
        .split_once(char::is_whitespace)
        .unwrap_or((command, ""));
    let arg = arg.trim();
    let action = match name {
        "clear" => Some(Action::Clear),
        "undo" => Some(Action::Undo),
        "redo" => Some(Action::Redo),
        "picker" => Some(Action::Picker),
        "quit" => Some(Action::Quit),
        _ => None,
    };
    if let Some(a) = action {
        ui.action(a);
        return "OK".into();
    }
    if name == "toggle" {
        let active = !ui.state.borrow().active;
        {
            let mut s = ui.state.borrow_mut();
            s.scene.commit_text();
            s.scene.cancel_drag();
            s.scene.space_down = false;
            s.active = active;
        }
        if let Some(surface) = ui.window.surface() {
            if active {
                surface.set_input_region(None::<&cairo::Region>);
            } else {
                surface.set_input_region(Some(&cairo::Region::create()));
            }
        }
        ui.window.set_keyboard_mode(if active {
            KeyboardMode::OnDemand
        } else {
            KeyboardMode::None
        });
        ui.refresh();
        return "OK".into();
    }
    let mut s = ui.state.borrow_mut();
    let scene = &mut s.scene;
    match name {
        "tool" => {
            let Some(tool) = Tool::from_name(arg) else {
                return "ERROR unknown tool".into();
            };
            scene.set_tool(tool);
        }
        "color" | "fill" => {
            let Some(c) = parse_color(arg) else {
                return "ERROR invalid color".into();
            };
            scene.restyle(|s| {
                if name == "color" {
                    s.color = c;
                } else {
                    s.fill = c;
                }
            });
        }
        "font" => {
            if arg.is_empty() {
                return "ERROR missing font family".into();
            }
            scene.restyle(|s| s.font = arg.into());
        }
        "width" | "font-size" | "roughness" => {
            let (min, max) = match name {
                "width" => (0.5, 50.),
                "font-size" => (6., 256.),
                _ => (0., 3.),
            };
            let Ok(value) = arg.parse::<f64>() else {
                return "ERROR expected a number".into();
            };
            if !value.is_finite() || !(min..=max).contains(&value) {
                return format!("ERROR expected {min}..{max}");
            }
            scene.restyle(|s| match name {
                "width" => s.width = value,
                "font-size" => s.font_size = value,
                _ => s.roughness = value,
            });
        }
        "theme" => {
            let theme = match arg {
                "light" => Theme::Light,
                "dark" => Theme::Dark,
                _ => return "ERROR theme must be light or dark".into(),
            };
            scene.set_theme(theme);
        }
        "roundness" => {
            let rounded = match arg {
                "round" => true,
                "sharp" => false,
                _ => return "ERROR roundness must be round or sharp".into(),
            };
            scene.restyle(|style| style.rounded = rounded);
        }
        "stroke-variability" => {
            let variability = match arg {
                "constant" => StrokeVariability::Constant,
                "variable" => StrokeVariability::Variable,
                _ => return "ERROR stroke-variability must be constant or variable".into(),
            };
            scene.restyle(|style| style.stroke_variability = variability);
        }
        "fill-style" => {
            let style = match arg {
                "hachure" => FillStyle::Hachure,
                "cross_hatch" => FillStyle::CrossHatch,
                "solid" => FillStyle::Solid,
                _ => return "ERROR invalid fill style".into(),
            };
            scene.restyle(|s| s.fill_style = style);
        }
        "stroke-style" => {
            let style = match arg {
                "solid" => StrokeStyle::Solid,
                "dashed" => StrokeStyle::Dashed,
                "dotted" => StrokeStyle::Dotted,
                _ => return "ERROR invalid stroke style".into(),
            };
            scene.restyle(|s| s.stroke_style = style);
        }
        _ => return "ERROR unknown command; use rsdraw help".into(),
    }
    drop(s);
    ui.refresh();
    "OK".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires a Wayland display; briefly opens a transparency test window"]
    fn themed_window_keeps_clear_pixels_transparent() {
        gtk4::init().unwrap();
        let app = Application::builder()
            .application_id("io.github.rsdraw.transparencytest")
            .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk4::gio::Cancellable>).unwrap();
        // Reproduce a user theme, including opaque window and canvas rules.
        let theme = gtk4::CssProvider::new();
        theme.load_from_string(
            "window { background: #ff00ff; } drawingarea { background: #ff00ff; }",
        );
        let display = gdk::Display::default().unwrap();
        gtk4::style_context_add_provider_for_display(
            &display,
            &theme,
            gtk4::STYLE_PROVIDER_PRIORITY_USER,
        );
        let window = ApplicationWindow::builder()
            .application(&app)
            .default_width(128)
            .default_height(128)
            .decorated(false)
            .build();
        install_style(&window);
        let root = Overlay::new();
        root.add_css_class("rsdraw-canvas-root");
        let canvas = DrawingArea::new();
        canvas.add_css_class("rsdraw-canvas");
        canvas.set_hexpand(true);
        canvas.set_vexpand(true);
        canvas.set_draw_func(|_, cr, _, _| {
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_rgba(0., 0., 0., 0.);
            cr.paint().unwrap();
            cr.set_operator(cairo::Operator::Over);
            cr.set_source_rgba(1., 0., 0., 1.);
            cr.rectangle(40., 40., 20., 20.);
            cr.fill().unwrap();
        });
        root.set_child(Some(&canvas));
        window.set_child(Some(&root));
        window.present();
        let context = glib::MainContext::default();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(200);
        while std::time::Instant::now() < deadline {
            while context.pending() {
                context.iteration(false);
            }
            std::thread::sleep(std::time::Duration::from_millis(2));
        }
        let snapshot = gtk4::Snapshot::new();
        let paintable = gtk4::WidgetPaintable::new(Some(&window));
        paintable.snapshot(&snapshot, window.width() as f64, window.height() as f64);
        let node = snapshot.to_node().expect("window snapshot");
        let viewport =
            gtk4::graphene::Rect::new(0., 0., window.width() as f32, window.height() as f32);
        let texture = window
            .renderer()
            .unwrap()
            .render_texture(&node, Some(&viewport));
        let stride = texture.width() as usize * 4;
        let mut pixels = vec![0; stride * texture.height() as usize];
        texture.download(&mut pixels, stride);
        let background_alpha = pixels[10 * stride + 10 * 4 + 3];
        let ink_alpha = pixels[50 * stride + 50 * 4 + 3];
        window.close();
        gtk4::style_context_remove_provider_for_display(&display, &theme);
        assert_eq!(
            background_alpha, 0,
            "clear canvas must retain alpha=0 under user themes"
        );
        assert_eq!(ink_alpha, 255, "annotations must remain visible");
    }

    #[test]
    #[ignore = "requires a Wayland compositor with layer-shell; opens a temporary overlay"]
    fn native_text_editor_and_empty_canvas() {
        gtk4::init().unwrap();
        crate::fonts::register().unwrap();
        let app = Application::builder()
            .application_id("io.github.rsdraw.smoketest")
            .flags(gtk4::gio::ApplicationFlags::NON_UNIQUE)
            .build();
        app.register(None::<&gtk4::gio::Cancellable>).unwrap();
        let ui = build_ui(&app, &Config::default()).unwrap();
        assert!(!ui.editor.is_visible());
        assert!(!ui.help.is_visible());
        assert!(!ui.palette.is_visible());
        let overlay = ui.window.child().unwrap().downcast::<Overlay>().unwrap();
        assert_eq!(
            overlay.observe_children().n_items(),
            4,
            "canvas and hidden editor, help, color picker only"
        );
        ui.state.borrow_mut().scene.begin_text((300., 200.), None);
        ui.refresh();
        assert!(ui.editor.is_visible());
        ui.editor.buffer().set_text("p r s — Ελληνικά\nsecond line");
        assert!(
            matches!(&ui.state.borrow().scene.editing.as_ref().unwrap().geometry,Geometry::Text{text,..} if text=="p r s — Ελληνικά\nsecond line")
        );
        // A new text placement must not keep the previous editor's buffer.
        ui.state.borrow_mut().scene.begin_text((500., 400.), None);
        ui.refresh();
        assert_eq!(ui.editor.buffer().char_count(), 0);
        ui.editor.buffer().set_text("next annotation");
        ui.action(Action::FinishText);
        assert!(!ui.editor.is_visible());
        assert_eq!(ui.state.borrow().scene.elements.len(), 2);
        ui.action(Action::Undo);
        assert_eq!(ui.state.borrow().scene.elements.len(), 1);
        ui.action(Action::Redo);
        assert_eq!(ui.state.borrow().scene.elements.len(), 2);
        ui.choose_color(false);
        assert!(ui.palette.is_visible());
        ui.apply_color(parse_color("red").unwrap());
        assert!(!ui.palette.is_visible());
        assert_eq!(
            ui.state.borrow().scene.style.color,
            parse_color("red").unwrap()
        );
        assert_eq!(handle_ipc("font JetBrains Mono", &ui), "OK");
        assert_eq!(ui.state.borrow().scene.style.font, "JetBrains Mono");
        let width = ui.state.borrow().scene.style.width;
        assert!(handle_ipc("width NaN", &ui).starts_with("ERROR"));
        assert_eq!(ui.state.borrow().scene.style.width, width);
        assert_eq!(handle_ipc("toggle", &ui), "OK");
        assert!(!ui.state.borrow().active);
        assert_eq!(handle_ipc("toggle", &ui), "OK");
        assert!(ui.state.borrow().active);
        ui.window.close();
        while glib::MainContext::default().pending() {
            glib::MainContext::default().iteration(false);
        }
    }
}
