use gtk4::prelude::*;
use gtk4::{Application, ApplicationWindow, DrawingArea, EventControllerKey, GestureDrag};
use gtk4_layer_shell::{Edge, KeyboardMode, Layer, LayerShell};
use std::cell::RefCell;
use std::rc::Rc;

use crate::canvas::{self, Element, Shape, Stroke, TextAnnotation, Tool};
use crate::config::{self, Config};
use crate::ipc;

struct ParsedBinding {
    key: gdk4::Key,
    mods: gdk4::ModifierType,
}

impl ParsedBinding {
    fn parse(s: &str) -> Option<Self> {
        let parts: Vec<&str> = s.split('+').collect();
        let mut mods = gdk4::ModifierType::empty();
        let mut key_name = "";

        for part in &parts {
            match part.trim().to_lowercase().as_str() {
                "ctrl" | "control" => mods |= gdk4::ModifierType::CONTROL_MASK,
                "shift" => mods |= gdk4::ModifierType::SHIFT_MASK,
                "alt" => mods |= gdk4::ModifierType::ALT_MASK,
                "super" => mods |= gdk4::ModifierType::SUPER_MASK,
                _ => key_name = part.trim(),
            }
        }

        let key = key_from_name(key_name)?;
        Some(Self { key, mods })
    }

    fn matches(&self, key: gdk4::Key, mods: gdk4::ModifierType) -> bool {
        let mask = gdk4::ModifierType::CONTROL_MASK
            | gdk4::ModifierType::SHIFT_MASK
            | gdk4::ModifierType::ALT_MASK
            | gdk4::ModifierType::SUPER_MASK;
        self.key == key && (mods & mask) == self.mods
    }
}

fn key_from_name(name: &str) -> Option<gdk4::Key> {
    use gdk4::Key;
    Some(match name.to_lowercase().as_str() {
        "a" => Key::a,
        "b" => Key::b,
        "c" => Key::c,
        "d" => Key::d,
        "e" => Key::e,
        "f" => Key::f,
        "g" => Key::g,
        "h" => Key::h,
        "i" => Key::i,
        "j" => Key::j,
        "k" => Key::k,
        "l" => Key::l,
        "m" => Key::m,
        "n" => Key::n,
        "o" => Key::o,
        "p" => Key::p,
        "q" => Key::q,
        "r" => Key::r,
        "s" => Key::s,
        "t" => Key::t,
        "u" => Key::u,
        "v" => Key::v,
        "w" => Key::w,
        "x" => Key::x,
        "y" => Key::y,
        "z" => Key::z,
        "0" => Key::_0,
        "1" => Key::_1,
        "2" => Key::_2,
        "3" => Key::_3,
        "4" => Key::_4,
        "5" => Key::_5,
        "6" => Key::_6,
        "7" => Key::_7,
        "8" => Key::_8,
        "9" => Key::_9,
        "f1" => Key::F1,
        "f2" => Key::F2,
        "f3" => Key::F3,
        "f4" => Key::F4,
        "f5" => Key::F5,
        "f6" => Key::F6,
        "f7" => Key::F7,
        "f8" => Key::F8,
        "f9" => Key::F9,
        "f10" => Key::F10,
        "f11" => Key::F11,
        "f12" => Key::F12,
        "escape" | "esc" => Key::Escape,
        "space" => Key::space,
        "tab" => Key::Tab,
        "return" | "enter" => Key::Return,
        "backspace" => Key::BackSpace,
        "delete" | "del" => Key::Delete,
        "equal" | "=" => Key::equal,
        "minus" | "-" => Key::minus,
        "plus" => Key::plus,
        "period" | "." => Key::period,
        "comma" | "," => Key::comma,
        "slash" | "/" => Key::slash,
        "backslash" | "\\" => Key::backslash,
        "bracketleft" | "[" => Key::bracketleft,
        "bracketright" | "]" => Key::bracketright,
        "semicolon" | ";" => Key::semicolon,
        "apostrophe" | "'" => Key::apostrophe,
        "grave" | "`" => Key::grave,
        _ => return None,
    })
}

struct Bindings {
    quit: ParsedBinding,
    undo: ParsedBinding,
    help: ParsedBinding,
    picker: ParsedBinding,
    color_red: ParsedBinding,
    color_green: ParsedBinding,
    color_blue: ParsedBinding,
    color_yellow: ParsedBinding,
    color_white: ParsedBinding,
    color_orange: ParsedBinding,
    brush_increase: ParsedBinding,
    brush_decrease: ParsedBinding,
    tool_freehand: ParsedBinding,
    tool_rectangle: ParsedBinding,
    tool_circle: ParsedBinding,
    tool_triangle: ParsedBinding,
    tool_arrow: ParsedBinding,
    tool_text: ParsedBinding,
}

impl Bindings {
    fn from_config(k: &config::KeysConfig) -> Self {
        let p = |s: &str, fallback: &str| -> ParsedBinding {
            ParsedBinding::parse(s).unwrap_or_else(|| {
                eprintln!("warning: invalid keybinding '{}', using '{}'", s, fallback);
                ParsedBinding::parse(fallback).unwrap()
            })
        };
        Self {
            quit: p(&k.quit, "Ctrl+x"),
            undo: p(&k.undo, "Ctrl+z"),
            help: p(&k.help, "F1"),
            picker: p(&k.picker, "p"),
            color_red: p(&k.color_red, "1"),
            color_green: p(&k.color_green, "2"),
            color_blue: p(&k.color_blue, "3"),
            color_yellow: p(&k.color_yellow, "4"),
            color_white: p(&k.color_white, "5"),
            color_orange: p(&k.color_orange, "6"),
            brush_increase: p(&k.brush_increase, "equal"),
            brush_decrease: p(&k.brush_decrease, "minus"),
            tool_freehand: p(&k.tool_freehand, "f"),
            tool_rectangle: p(&k.tool_rectangle, "r"),
            tool_circle: p(&k.tool_circle, "c"),
            tool_triangle: p(&k.tool_triangle, "t"),
            tool_arrow: p(&k.tool_arrow, "a"),
            tool_text: p(&k.tool_text, "x"),
        }
    }
}

struct AppState {
    elements: Vec<Element>,
    current_stroke: Option<Stroke>,
    current_shape: Option<Shape>,
    current_text: Option<TextAnnotation>,
    tool: Tool,
    drawing_active: bool,
    show_help: bool,
    show_color_picker: bool,
    color: (f64, f64, f64, f64),
    color_name: String,
    brush_width: f64,
    font_family: String,
    backing_surface: Option<cairo::ImageSurface>,
    surface_dirty: bool,
    surface_size: (i32, i32),
}

impl AppState {
    fn from_config(cfg: &Config) -> Self {
        let color = parse_color(&cfg.drawing.default_color);
        Self {
            elements: Vec::new(),
            current_stroke: None,
            current_shape: None,
            current_text: None,
            tool: Tool::Freehand,
            drawing_active: true,
            show_help: false,
            show_color_picker: false,
            color,
            color_name: cfg.drawing.default_color.clone(),
            brush_width: cfg.drawing.default_width.clamp(1.0, 50.0),
            font_family: cfg.drawing.font_family.clone(),
            backing_surface: None,
            surface_dirty: true,
            surface_size: (0, 0),
        }
    }

    fn set_color(&mut self, name: &str, rgba: (f64, f64, f64, f64)) {
        self.color = rgba;
        self.color_name = name.to_string();
        if let Some(ref mut text) = self.current_text {
            text.color = rgba;
        }
    }

    fn commit_text(&mut self) {
        if let Some(text) = self.current_text.take() {
            if !text.text.trim().is_empty() {
                let elem = Element::Text(text);
                self.bake_element(&elem);
                self.elements.push(elem);
            }
        }
    }

    fn rebake_surface(&mut self, w: i32, h: i32) {
        if w <= 0 || h <= 0 {
            return;
        }
        let surface = match cairo::ImageSurface::create(cairo::Format::ARgb32, w, h) {
            Ok(s) => s,
            Err(_) => return,
        };
        if let Ok(cr) = cairo::Context::new(&surface) {
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            let _ = cr.paint();
            cr.set_operator(cairo::Operator::Over);
            for elem in &self.elements {
                elem.draw(&cr);
            }
        }
        self.backing_surface = Some(surface);
        self.surface_size = (w, h);
        self.surface_dirty = false;
    }

    fn bake_element(&mut self, elem: &Element) {
        if let Some(ref surface) = self.backing_surface {
            if let Ok(cr) = cairo::Context::new(surface) {
                cr.set_operator(cairo::Operator::Over);
                elem.draw(&cr);
            }
        } else {
            self.surface_dirty = true;
        }
    }
}

pub fn run(config: Config) {
    let app = Application::builder()
        .application_id("io.github.rsdraw")
        .build();

    let config = Rc::new(config);
    let cfg = config.clone();
    app.connect_activate(move |app| build_ui(app, &cfg));
    app.run_with_args::<String>(&[]);
}

fn build_ui(app: &Application, config: &Config) {
    let window = ApplicationWindow::builder()
        .application(app)
        .title("rsdraw")
        .build();

    window.init_layer_shell();
    window.set_layer(Layer::Overlay);
    window.set_anchor(Edge::Top, true);
    window.set_anchor(Edge::Bottom, true);
    window.set_anchor(Edge::Left, true);
    window.set_anchor(Edge::Right, true);
    window.set_exclusive_zone(-1);
    window.set_keyboard_mode(KeyboardMode::OnDemand);
    window.set_decorated(false);

    let css = gtk4::CssProvider::new();
    css.load_from_string(
        "window, window.background, .background, headerbar, decoration { \
            background: none; \
            background-color: rgba(0,0,0,0); \
            border: none; \
            box-shadow: none; \
        }",
    );
    gtk4::style_context_add_provider_for_display(
        &gdk4::Display::default().expect("no display"),
        &css,
        gtk4::STYLE_PROVIDER_PRIORITY_USER,
    );

    let state = Rc::new(RefCell::new(AppState::from_config(config)));
    let bindings = Rc::new(Bindings::from_config(&config.keys));
    let help_lines = Rc::new(config::help_lines(config));

    let canvas = DrawingArea::new();
    canvas.set_hexpand(true);
    canvas.set_vexpand(true);
    canvas.set_cursor_from_name(Some("crosshair"));

    {
        let state = state.clone();
        let help_lines = help_lines.clone();
        canvas.set_draw_func(move |_widget, cr, w, h| {
            cr.set_operator(cairo::Operator::Source);
            cr.set_source_rgba(0.0, 0.0, 0.0, 0.0);
            let _ = cr.paint();

            cr.set_operator(cairo::Operator::Over);
            let mut s = state.borrow_mut();

            if s.backing_surface.is_none() || s.surface_size != (w, h) || s.surface_dirty {
                s.rebake_surface(w, h);
            }
            if let Some(ref surface) = s.backing_surface {
                let _ = cr.set_source_surface(surface, 0.0, 0.0);
                let _ = cr.paint();
            }

            if let Some(ref current) = s.current_stroke {
                current.draw(cr);
            }
            if let Some(ref current) = s.current_shape {
                current.draw(cr);
            }
            if let Some(ref current) = s.current_text {
                current.draw(cr, true);
            }

            if s.show_help {
                canvas::draw_help_panel(
                    cr,
                    &help_lines,
                    w,
                    h,
                    &s.color_name,
                    s.brush_width,
                    s.tool.name(),
                );
            }

            if s.show_color_picker {
                canvas::draw_color_picker(cr, w, h, s.color, &s.color_name);
            }
        });
    }

    let drag = GestureDrag::new();
    let drag_origin: Rc<RefCell<(f64, f64)>> = Rc::new(RefCell::new((0.0, 0.0)));

    {
        let state = state.clone();
        let origin = drag_origin.clone();
        let canvas_ref = canvas.clone();
        drag.connect_drag_begin(move |_g, x, y| {
            let mut s = state.borrow_mut();
            if !s.drawing_active {
                return;
            }

            if s.show_color_picker {
                let (w, h) = s.surface_size;
                match canvas::hit_test_color_picker(x, y, w, h) {
                    Some(canvas::ColorPickerAction::SelectColor(rgba, hex)) => {
                        s.set_color(&hex, rgba);
                        s.show_color_picker = false;
                        drop(s);
                        canvas_ref.queue_draw();
                        return;
                    }
                    Some(canvas::ColorPickerAction::Close) => {
                        s.show_color_picker = false;
                        drop(s);
                        canvas_ref.queue_draw();
                        return;
                    }
                    Some(canvas::ColorPickerAction::Inside) => {
                        return;
                    }
                    None => {}
                }
            }

            *origin.borrow_mut() = (x, y);
            match s.tool {
                Tool::Freehand => {
                    s.commit_text();
                    let mut stroke = Stroke::new(s.color, s.brush_width);
                    stroke.add_point(x, y);
                    s.current_stroke = Some(stroke);
                }
                Tool::Text => {
                    s.commit_text();
                    let font_size = (s.brush_width * 6.0).clamp(14.0, 72.0);
                    s.current_text = Some(TextAnnotation::new(
                        (x, y),
                        s.color,
                        font_size,
                        s.font_family.clone(),
                    ));
                    drop(s);
                    canvas_ref.queue_draw();
                }
                shape_tool => {
                    s.commit_text();
                    s.current_shape = Some(Shape {
                        tool: shape_tool,
                        start: (x, y),
                        end: (x, y),
                        color: s.color,
                        width: s.brush_width,
                    });
                }
            }
        });
    }
    {
        let state = state.clone();
        let origin = drag_origin.clone();
        let canvas_ref = canvas.clone();
        drag.connect_drag_update(move |_g, dx, dy| {
            let mut s = state.borrow_mut();
            if !s.drawing_active || s.show_color_picker {
                return;
            }
            let (sx, sy) = *origin.borrow();
            let cur_x = sx + dx;
            let cur_y = sy + dy;

            match s.tool {
                Tool::Freehand => {
                    if let Some(ref mut stroke) = s.current_stroke {
                        stroke.add_point(cur_x, cur_y);
                    }
                }
                Tool::Text => {}
                _ => {
                    if let Some(ref mut shape) = s.current_shape {
                        shape.end = (cur_x, cur_y);
                    }
                }
            }
            drop(s);
            canvas_ref.queue_draw();
        });
    }
    {
        let state = state.clone();
        let canvas_ref = canvas.clone();
        drag.connect_drag_end(move |_g, _dx, _dy| {
            let mut s = state.borrow_mut();
            if s.show_color_picker {
                return;
            }
            if let Some(stroke) = s.current_stroke.take() {
                if !stroke.points.is_empty() {
                    let elem = Element::Freehand(stroke);
                    s.bake_element(&elem);
                    s.elements.push(elem);
                }
            }
            if let Some(shape) = s.current_shape.take() {
                let dx = shape.end.0 - shape.start.0;
                let dy = shape.end.1 - shape.start.1;
                if dx.abs() > 2.0 || dy.abs() > 2.0 {
                    let elem = Element::Shape(shape);
                    s.bake_element(&elem);
                    s.elements.push(elem);
                }
            }
            drop(s);
            canvas_ref.queue_draw();
        });
    }
    canvas.add_controller(drag);

    let key_ctrl = EventControllerKey::new();
    {
        let state = state.clone();
        let bindings = bindings.clone();
        let canvas_ref = canvas.clone();
        key_ctrl.connect_key_pressed(move |_ctrl, key, _code, mods| {
            if bindings.quit.matches(key, mods) {
                let _ = std::fs::remove_file(ipc::socket_path());
                std::process::exit(0);
            }

            if key == gdk4::Key::Escape && state.borrow().show_color_picker {
                state.borrow_mut().show_color_picker = false;
                canvas_ref.queue_draw();
                return glib::Propagation::Stop;
            }

            if bindings.picker.matches(key, mods) {
                let mut s = state.borrow_mut();
                s.show_color_picker = !s.show_color_picker;
                drop(s);
                canvas_ref.queue_draw();
                return glib::Propagation::Stop;
            }

            {
                let mut s = state.borrow_mut();
                if s.current_text.is_some() {
                    use gdk4::Key;
                    match key {
                        Key::Return | Key::KP_Enter => {
                            s.commit_text();
                            drop(s);
                            canvas_ref.queue_draw();
                            return glib::Propagation::Stop;
                        }
                        Key::Escape => {
                            s.current_text = None;
                            drop(s);
                            canvas_ref.queue_draw();
                            return glib::Propagation::Stop;
                        }
                        Key::BackSpace => {
                            if let Some(ref mut text) = s.current_text {
                                text.text.pop();
                            }
                            drop(s);
                            canvas_ref.queue_draw();
                            return glib::Propagation::Stop;
                        }
                        _ => {
                            if let Some(ch) = key.to_unicode() {
                                if !ch.is_control() {
                                    if let Some(ref mut text) = s.current_text {
                                        text.text.push(ch);
                                    }
                                    drop(s);
                                    canvas_ref.queue_draw();
                                    return glib::Propagation::Stop;
                                }
                            }
                        }
                    }
                }
            }

            if bindings.help.matches(key, mods) {
                let mut s = state.borrow_mut();
                s.show_help = !s.show_help;
                drop(s);
                canvas_ref.queue_draw();
                return glib::Propagation::Stop;
            }

            let mut s = state.borrow_mut();
            if !s.drawing_active {
                return glib::Propagation::Proceed;
            }

            if bindings.undo.matches(key, mods) {
                if s.current_text.is_some() {
                    s.current_text = None;
                } else {
                    s.elements.pop();
                    s.surface_dirty = true;
                }
                drop(s);
                canvas_ref.queue_draw();
                return glib::Propagation::Stop;
            }

            if bindings.tool_freehand.matches(key, mods) {
                s.commit_text();
                s.tool = Tool::Freehand;
            } else if bindings.tool_rectangle.matches(key, mods) {
                s.commit_text();
                s.tool = Tool::Rectangle;
            } else if bindings.tool_circle.matches(key, mods) {
                s.commit_text();
                s.tool = Tool::Circle;
            } else if bindings.tool_triangle.matches(key, mods) {
                s.commit_text();
                s.tool = Tool::Triangle;
            } else if bindings.tool_arrow.matches(key, mods) {
                s.commit_text();
                s.tool = Tool::Arrow;
            } else if bindings.tool_text.matches(key, mods) {
                s.commit_text();
                s.tool = Tool::Text;
            } else if bindings.color_red.matches(key, mods) {
                let rgba = parse_color("red");
                s.set_color("red", rgba);
            } else if bindings.color_green.matches(key, mods) {
                let rgba = parse_color("green");
                s.set_color("green", rgba);
            } else if bindings.color_blue.matches(key, mods) {
                let rgba = parse_color("blue");
                s.set_color("blue", rgba);
            } else if bindings.color_yellow.matches(key, mods) {
                let rgba = parse_color("yellow");
                s.set_color("yellow", rgba);
            } else if bindings.color_white.matches(key, mods) {
                let rgba = parse_color("white");
                s.set_color("white", rgba);
            } else if bindings.color_orange.matches(key, mods) {
                let rgba = parse_color("orange");
                s.set_color("orange", rgba);
            } else if bindings.brush_increase.matches(key, mods) {
                s.brush_width = (s.brush_width + 1.0).min(50.0);
                let text_size = (s.brush_width * 6.0).clamp(14.0, 72.0);
                if let Some(ref mut text) = s.current_text {
                    text.size = text_size;
                }
            } else if bindings.brush_decrease.matches(key, mods) {
                s.brush_width = (s.brush_width - 1.0).max(1.0);
                let text_size = (s.brush_width * 6.0).clamp(14.0, 72.0);
                if let Some(ref mut text) = s.current_text {
                    text.size = text_size;
                }
            } else {
                return glib::Propagation::Proceed;
            }

            drop(s);
            canvas_ref.queue_draw();
            glib::Propagation::Stop
        });
    }
    window.add_controller(key_ctrl);

    window.set_child(Some(&canvas));

    let (sender, receiver) = std::sync::mpsc::channel::<String>();
    ipc::start_server(sender);

    {
        let state = state.clone();
        let canvas_ref = canvas.clone();
        let window_ref = window.clone();
        glib::timeout_add_local(std::time::Duration::from_millis(50), move || {
            while let Ok(cmd) = receiver.try_recv() {
                handle_ipc(&cmd, &state, &canvas_ref, &window_ref);
            }
            glib::ControlFlow::Continue
        });
    }

    window.present();
    set_input_mode(&window, true);
}

fn handle_ipc(
    cmd: &str,
    state: &Rc<RefCell<AppState>>,
    canvas: &DrawingArea,
    window: &ApplicationWindow,
) {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    match parts.first().copied() {
        Some("toggle") => {
            let active = {
                let mut s = state.borrow_mut();
                s.commit_text();
                s.drawing_active = !s.drawing_active;
                s.drawing_active
            };
            set_input_mode(window, active);
            canvas.set_cursor_from_name(Some(if active { "crosshair" } else { "default" }));
        }
        Some("clear") => {
            {
                let mut s = state.borrow_mut();
                s.elements.clear();
                s.current_stroke = None;
                s.current_shape = None;
                s.current_text = None;
                s.surface_dirty = true;
            }
            canvas.queue_draw();
        }
        Some("undo") => {
            let mut s = state.borrow_mut();
            if s.current_text.is_some() {
                s.current_text = None;
            } else {
                s.elements.pop();
                s.surface_dirty = true;
            }
            drop(s);
            canvas.queue_draw();
        }
        Some("quit") => {
            let _ = std::fs::remove_file(ipc::socket_path());
            std::process::exit(0);
        }
        Some("picker") => {
            let mut s = state.borrow_mut();
            s.show_color_picker = !s.show_color_picker;
            drop(s);
            canvas.queue_draw();
        }
        Some("tool") => {
            if let Some(name) = parts.get(1) {
                if let Some(tool) = Tool::from_name(name) {
                    let mut s = state.borrow_mut();
                    s.commit_text();
                    s.tool = tool;
                    drop(s);
                    canvas.queue_draw();
                }
            }
        }
        Some("color") => {
            if let Some(name) = parts.get(1) {
                let rgba = parse_color(name);
                state.borrow_mut().set_color(name, rgba);
                canvas.queue_draw();
            }
        }
        Some("font") => {
            if let Some(font) = parts.get(1) {
                let mut s = state.borrow_mut();
                s.font_family = font.to_string();
                let font_clone = s.font_family.clone();
                if let Some(ref mut text) = s.current_text {
                    text.font_family = font_clone;
                }
                drop(s);
                canvas.queue_draw();
            }
        }
        Some("width") => {
            if let Some(w) = parts.get(1).and_then(|v| v.parse::<f64>().ok()) {
                let mut s = state.borrow_mut();
                s.brush_width = w.clamp(1.0, 50.0);
                let text_size = (s.brush_width * 6.0).clamp(14.0, 72.0);
                if let Some(ref mut text) = s.current_text {
                    text.size = text_size;
                }
                drop(s);
                canvas.queue_draw();
            }
        }
        _ => {}
    }
}

fn set_input_mode(window: &ApplicationWindow, active: bool) {
    if let Some(surface) = window.surface() {
        if active {
            surface.set_input_region(None::<&cairo::Region>);
        } else {
            let empty = cairo::Region::create();
            surface.set_input_region(Some(&empty));
        }
    }
    if active {
        window.set_keyboard_mode(KeyboardMode::OnDemand);
    } else {
        window.set_keyboard_mode(KeyboardMode::None);
    }
}

fn parse_color(name: &str) -> (f64, f64, f64, f64) {
    let s = name.trim();
    if let Some(hex) = s.strip_prefix('#') {
        if let Some(rgba) = parse_hex_body(hex) {
            return rgba;
        }
    }
    match s.to_lowercase().as_str() {
        "red" => (1.0, 0.23, 0.19, 1.0),
        "green" => (0.20, 0.78, 0.35, 1.0),
        "blue" => (0.0, 0.48, 1.0, 1.0),
        "yellow" => (1.0, 0.80, 0.0, 1.0),
        "white" => (1.0, 1.0, 1.0, 1.0),
        "orange" => (1.0, 0.58, 0.0, 1.0),
        "cyan" => (0.20, 0.68, 0.90, 1.0),
        "magenta" => (1.0, 0.0, 1.0, 1.0),
        "pink" => (1.0, 0.18, 0.33, 1.0),
        "black" => (0.0, 0.0, 0.0, 1.0),
        "gray" | "grey" => (0.56, 0.56, 0.58, 1.0),
        "darkgray" | "darkgrey" => (0.23, 0.23, 0.24, 1.0),
        "purple" => (0.69, 0.32, 0.87, 1.0),
        "indigo" => (0.35, 0.34, 0.84, 1.0),
        "teal" => (0.19, 0.69, 0.78, 1.0),
        "mint" => (0.0, 0.78, 0.75, 1.0),
        "brown" => (0.64, 0.52, 0.37, 1.0),
        _ => (1.0, 0.23, 0.19, 1.0),
    }
}

fn parse_hex_body(hex: &str) -> Option<(f64, f64, f64, f64)> {
    match hex.len() {
        3 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            Some((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0))
        }
        4 => {
            let r = u8::from_str_radix(&hex[0..1], 16).ok()? * 17;
            let g = u8::from_str_radix(&hex[1..2], 16).ok()? * 17;
            let b = u8::from_str_radix(&hex[2..3], 16).ok()? * 17;
            let a = u8::from_str_radix(&hex[3..4], 16).ok()? * 17;
            Some((
                r as f64 / 255.0,
                g as f64 / 255.0,
                b as f64 / 255.0,
                a as f64 / 255.0,
            ))
        }
        6 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            Some((r as f64 / 255.0, g as f64 / 255.0, b as f64 / 255.0, 1.0))
        }
        8 => {
            let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
            let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
            let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
            let a = u8::from_str_radix(&hex[6..8], 16).ok()?;
            Some((
                r as f64 / 255.0,
                g as f64 / 255.0,
                b as f64 / 255.0,
                a as f64 / 255.0,
            ))
        }
        _ => None,
    }
}
