use cairo::Context;

#[derive(Clone, Copy, PartialEq)]
pub enum Tool {
    Freehand,
    Rectangle,
    Circle,
    Triangle,
    Arrow,
    Text,
}

impl Tool {
    pub fn name(&self) -> &'static str {
        match self {
            Self::Freehand => "freehand",
            Self::Rectangle => "rectangle",
            Self::Circle => "circle",
            Self::Triangle => "triangle",
            Self::Arrow => "arrow",
            Self::Text => "text",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s.to_lowercase().as_str() {
            "freehand" | "free" | "pen" | "draw" => Self::Freehand,
            "rectangle" | "rect" | "box" => Self::Rectangle,
            "circle" | "ellipse" | "oval" => Self::Circle,
            "triangle" | "tri" => Self::Triangle,
            "arrow" | "line" => Self::Arrow,
            "text" | "type" | "font" => Self::Text,
            _ => return None,
        })
    }
}

/// Anything drawn on screen — freehand stroke, geometric shape, or text.
#[derive(Clone)]
pub enum Element {
    Freehand(Stroke),
    Shape(Shape),
    Text(TextAnnotation),
}

impl Element {
    pub fn draw(&self, cr: &Context) {
        match self {
            Self::Freehand(s) => s.draw(cr),
            Self::Shape(s) => s.draw(cr),
            Self::Text(t) => t.draw(cr, false),
        }
    }
}

#[derive(Clone)]
pub struct Stroke {
    pub points: Vec<(f64, f64)>,
    pub color: (f64, f64, f64, f64),
    pub width: f64,
}

impl Stroke {
    pub fn new(color: (f64, f64, f64, f64), width: f64) -> Self {
        Self {
            points: Vec::with_capacity(64),
            color,
            width,
        }
    }

    pub fn add_point(&mut self, x: f64, y: f64) {
        if let Some(&(last_x, last_y)) = self.points.last() {
            let dx = x - last_x;
            let dy = y - last_y;
            if dx * dx + dy * dy < 4.0 {
                return;
            }
        }
        self.points.push((x, y));
    }

    pub fn draw(&self, cr: &Context) {
        if self.points.is_empty() {
            return;
        }

        cr.set_source_rgba(self.color.0, self.color.1, self.color.2, self.color.3);

        if self.points.len() == 1 {
            let (x, y) = self.points[0];
            let _ = cr.arc(x, y, self.width / 2.0, 0.0, std::f64::consts::TAU);
            let _ = cr.fill();
            return;
        }

        cr.set_line_width(self.width);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);

        cr.move_to(self.points[0].0, self.points[0].1);
        for &(x, y) in &self.points[1..] {
            cr.line_to(x, y);
        }
        let _ = cr.stroke();
    }
}

#[derive(Clone)]
pub struct Shape {
    pub tool: Tool,
    pub start: (f64, f64),
    pub end: (f64, f64),
    pub color: (f64, f64, f64, f64),
    pub width: f64,
}

impl Shape {
    pub fn draw(&self, cr: &Context) {
        cr.set_source_rgba(self.color.0, self.color.1, self.color.2, self.color.3);
        cr.set_line_width(self.width);
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);

        match self.tool {
            Tool::Rectangle => self.draw_rectangle(cr),
            Tool::Circle => self.draw_ellipse(cr),
            Tool::Triangle => self.draw_triangle(cr),
            Tool::Arrow => self.draw_arrow(cr),
            _ => {}
        }
    }

    fn draw_rectangle(&self, cr: &Context) {
        let x = self.start.0.min(self.end.0);
        let y = self.start.1.min(self.end.1);
        let w = (self.end.0 - self.start.0).abs();
        let h = (self.end.1 - self.start.1).abs();
        if w < 1.0 && h < 1.0 {
            return;
        }
        cr.rectangle(x, y, w, h);
        let _ = cr.stroke();
    }

    fn draw_ellipse(&self, cr: &Context) {
        let cx = (self.start.0 + self.end.0) / 2.0;
        let cy = (self.start.1 + self.end.1) / 2.0;
        let rx = (self.end.0 - self.start.0).abs() / 2.0;
        let ry = (self.end.1 - self.start.1).abs() / 2.0;
        if rx < 1.0 && ry < 1.0 {
            return;
        }

        let segments = 72;
        for i in 0..=segments {
            let t = i as f64 / segments as f64 * std::f64::consts::TAU;
            let px = cx + rx * t.cos();
            let py = cy + ry * t.sin();
            if i == 0 {
                cr.move_to(px, py);
            } else {
                cr.line_to(px, py);
            }
        }
        cr.close_path();
        let _ = cr.stroke();
    }

    fn draw_triangle(&self, cr: &Context) {
        let x = self.start.0.min(self.end.0);
        let y = self.start.1.min(self.end.1);
        let w = (self.end.0 - self.start.0).abs();
        let h = (self.end.1 - self.start.1).abs();
        if w < 1.0 && h < 1.0 {
            return;
        }

        cr.move_to(x + w / 2.0, y);
        cr.line_to(x, y + h);
        cr.line_to(x + w, y + h);
        cr.close_path();
        let _ = cr.stroke();
    }

    fn draw_arrow(&self, cr: &Context) {
        let (x1, y1) = self.start;
        let (x2, y2) = self.end;
        let dx = x2 - x1;
        let dy = y2 - y1;
        let len = (dx * dx + dy * dy).sqrt();
        if len < 2.0 {
            return;
        }

        cr.move_to(x1, y1);
        cr.line_to(x2, y2);
        let _ = cr.stroke();

        let head = (self.width * 4.0).clamp(12.0, 40.0);
        let angle = dy.atan2(dx);
        let spread = std::f64::consts::FRAC_PI_6;

        cr.move_to(x2, y2);
        cr.line_to(
            x2 - head * (angle - spread).cos(),
            y2 - head * (angle - spread).sin(),
        );
        cr.line_to(
            x2 - head * (angle + spread).cos(),
            y2 - head * (angle + spread).sin(),
        );
        cr.close_path();
        let _ = cr.fill();
    }
}

#[derive(Clone)]
pub struct TextAnnotation {
    pub pos: (f64, f64),
    pub text: String,
    pub color: (f64, f64, f64, f64),
    pub size: f64,
    pub font_family: String,
}

impl TextAnnotation {
    pub fn new(
        pos: (f64, f64),
        color: (f64, f64, f64, f64),
        size: f64,
        font_family: String,
    ) -> Self {
        Self {
            pos,
            text: String::new(),
            color,
            size,
            font_family,
        }
    }

    pub fn draw(&self, cr: &Context, is_active: bool) {
        cr.set_source_rgba(self.color.0, self.color.1, self.color.2, self.color.3);
        cr.select_font_face(
            &self.font_family,
            cairo::FontSlant::Normal,
            cairo::FontWeight::Bold,
        );
        cr.set_font_size(self.size);
        cr.move_to(self.pos.0, self.pos.1);

        let display_text = if is_active {
            format!("{}|", self.text)
        } else {
            self.text.clone()
        };

        let _ = cr.show_text(&display_text);
    }
}

pub fn draw_help_panel(
    cr: &Context,
    lines: &[(String, String)],
    canvas_w: i32,
    canvas_h: i32,
    color_name: &str,
    brush_width: f64,
    tool_name: &str,
) {
    let line_h = 22.0;
    let pad = 22.0;
    let col2_x = 150.0;
    let panel_w = 380.0;
    let panel_h = pad * 2.0 + (lines.len() as f64 + 3.5) * line_h;

    let x = (canvas_w as f64 - panel_w) / 2.0;
    let y = (canvas_h as f64 - panel_h) / 2.0;

    let r = 14.0;
    cr.new_sub_path();
    cr.arc(x + panel_w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + panel_w - r, y + panel_h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + panel_h - r, r, FRAC_PI_2, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * FRAC_PI_2);
    cr.close_path();
    cr.set_source_rgba(0.08, 0.08, 0.12, 0.90);
    let _ = cr.fill();

    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    cr.select_font_face(
        "monospace",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(15.0);
    let mut cy = y + pad + 15.0;
    cr.move_to(x + pad, cy);
    let _ = cr.show_text("rsdraw (Linux) — Keybindings");

    cr.select_font_face(
        "monospace",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(13.0);
    cy += line_h * 1.4;

    for (key, action) in lines {
        cr.set_source_rgba(0.35, 0.75, 1.0, 1.0);
        cr.move_to(x + pad, cy);
        let _ = cr.show_text(key);

        cr.set_source_rgba(0.78, 0.78, 0.78, 1.0);
        cr.move_to(x + pad + col2_x, cy);
        let _ = cr.show_text(action);

        cy += line_h;
    }

    cy += line_h * 0.4;
    cr.set_source_rgba(0.55, 0.55, 0.55, 1.0);
    cr.move_to(x + pad, cy);
    let _ = cr.show_text(&format!(
        "tool: {}   color: {}   width: {:.0}",
        tool_name, color_name, brush_width
    ));
}

pub const PALETTE: &[((f64, f64, f64, f64), &str)] = &[
    ((1.0, 0.23, 0.19, 1.0), "#ff3b30"),  // Red
    ((1.0, 0.58, 0.0, 1.0), "#ff9500"),   // Orange
    ((1.0, 0.80, 0.0, 1.0), "#ffcc00"),   // Yellow
    ((0.20, 0.78, 0.35, 1.0), "#34c759"), // Green
    ((0.0, 0.78, 0.75, 1.0), "#00c7be"),  // Mint
    ((0.19, 0.69, 0.78, 1.0), "#30b0c7"), // Teal
    ((0.20, 0.68, 0.90, 1.0), "#32ade6"), // Cyan
    ((0.0, 0.48, 1.0, 1.0), "#007aff"),   // Blue
    ((0.35, 0.34, 0.84, 1.0), "#5856d6"), // Indigo
    ((0.69, 0.32, 0.87, 1.0), "#af52de"), // Purple
    ((1.0, 0.18, 0.33, 1.0), "#ff2d55"),  // Pink
    ((0.64, 0.52, 0.37, 1.0), "#a2845e"), // Brown
    ((1.0, 1.0, 1.0, 1.0), "#ffffff"),    // White
    ((0.56, 0.56, 0.58, 1.0), "#8e8e93"), // Gray
    ((0.23, 0.23, 0.24, 1.0), "#3a3a3c"), // Dark Gray
    ((0.0, 0.0, 0.0, 1.0), "#000000"),    // Black
];

pub enum ColorPickerAction {
    SelectColor((f64, f64, f64, f64), String),
    Close,
    Inside,
}

pub fn draw_color_picker(
    cr: &Context,
    canvas_w: i32,
    canvas_h: i32,
    current_color: (f64, f64, f64, f64),
    current_color_name: &str,
) {
    let panel_w = 360.0;
    let panel_h = 210.0;
    let x = (canvas_w as f64 - panel_w) / 2.0;
    let y = (canvas_h as f64 - panel_h) / 2.0;
    let pad = 20.0;

    let r = 14.0;
    cr.new_sub_path();
    cr.arc(x + panel_w - r, y + r, r, -FRAC_PI_2, 0.0);
    cr.arc(x + panel_w - r, y + panel_h - r, r, 0.0, FRAC_PI_2);
    cr.arc(x + r, y + panel_h - r, r, FRAC_PI_2, PI);
    cr.arc(x + r, y + r, r, PI, 3.0 * FRAC_PI_2);
    cr.close_path();
    cr.set_source_rgba(0.08, 0.08, 0.12, 0.94);
    let _ = cr.fill();

    cr.set_source_rgba(1.0, 1.0, 1.0, 1.0);
    cr.select_font_face(
        "monospace",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Bold,
    );
    cr.set_font_size(14.0);
    cr.move_to(x + pad, y + pad + 12.0);
    let _ = cr.show_text("Color Palette [Esc/p to close]");

    let swatch_w = 36.0;
    let swatch_h = 26.0;
    let gap = 4.0;
    let swatches_y = y + pad + 24.0;

    for (i, &(rgba, _)) in PALETTE.iter().enumerate() {
        let col = (i % 8) as f64;
        let row = (i / 8) as f64;
        let sx = x + pad + col * (swatch_w + gap);
        let sy = swatches_y + row * (swatch_h + gap);

        cr.set_source_rgba(rgba.0, rgba.1, rgba.2, rgba.3);
        cr.rectangle(sx, sy, swatch_w, swatch_h);
        let _ = cr.fill();

        cr.set_source_rgba(0.4, 0.4, 0.4, 0.8);
        cr.set_line_width(1.0);
        cr.rectangle(sx, sy, swatch_w, swatch_h);
        let _ = cr.stroke();
    }

    let bar_y = swatches_y + 2.0 * (swatch_h + gap) + 8.0;
    let bar_w = 8.0 * swatch_w + 7.0 * gap;
    let bar_h = 18.0;
    let bar_x = x + pad;

    let pat = cairo::LinearGradient::new(bar_x, 0.0, bar_x + bar_w, 0.0);
    pat.add_color_stop_rgb(0.0 / 6.0, 1.0, 0.0, 0.0);
    pat.add_color_stop_rgb(1.0 / 6.0, 1.0, 1.0, 0.0);
    pat.add_color_stop_rgb(2.0 / 6.0, 0.0, 1.0, 0.0);
    pat.add_color_stop_rgb(3.0 / 6.0, 0.0, 1.0, 1.0);
    pat.add_color_stop_rgb(4.0 / 6.0, 0.0, 0.0, 1.0);
    pat.add_color_stop_rgb(5.0 / 6.0, 1.0, 0.0, 1.0);
    pat.add_color_stop_rgb(6.0 / 6.0, 1.0, 0.0, 0.0);
    let _ = cr.set_source(&pat);
    cr.rectangle(bar_x, bar_y, bar_w, bar_h);
    let _ = cr.fill();

    let prev_y = bar_y + bar_h + 10.0;
    let prev_w = 32.0;
    let prev_h = 24.0;
    cr.set_source_rgba(
        current_color.0,
        current_color.1,
        current_color.2,
        current_color.3,
    );
    cr.rectangle(bar_x, prev_y, prev_w, prev_h);
    let _ = cr.fill();
    cr.set_source_rgba(1.0, 1.0, 1.0, 0.8);
    cr.set_line_width(1.0);
    cr.rectangle(bar_x, prev_y, prev_w, prev_h);
    let _ = cr.stroke();

    cr.set_source_rgba(0.9, 0.9, 0.9, 1.0);
    cr.select_font_face(
        "monospace",
        cairo::FontSlant::Normal,
        cairo::FontWeight::Normal,
    );
    cr.set_font_size(13.0);
    cr.move_to(bar_x + prev_w + 12.0, prev_y + 16.0);
    let hex_display = format!(
        "Current: {} ({})",
        current_color_name,
        rgba_to_hex(current_color)
    );
    let _ = cr.show_text(&hex_display);
}

pub fn hit_test_color_picker(
    click_x: f64,
    click_y: f64,
    canvas_w: i32,
    canvas_h: i32,
) -> Option<ColorPickerAction> {
    let panel_w = 360.0;
    let panel_h = 210.0;
    let x = (canvas_w as f64 - panel_w) / 2.0;
    let y = (canvas_h as f64 - panel_h) / 2.0;
    let pad = 20.0;

    if click_x < x || click_x > x + panel_w || click_y < y || click_y > y + panel_h {
        return Some(ColorPickerAction::Close);
    }

    let swatch_w = 36.0;
    let swatch_h = 26.0;
    let gap = 4.0;
    let swatches_y = y + pad + 24.0;

    for (i, &(rgba, hex)) in PALETTE.iter().enumerate() {
        let col = (i % 8) as f64;
        let row = (i / 8) as f64;
        let sx = x + pad + col * (swatch_w + gap);
        let sy = swatches_y + row * (swatch_h + gap);

        if click_x >= sx && click_x <= sx + swatch_w && click_y >= sy && click_y <= sy + swatch_h {
            return Some(ColorPickerAction::SelectColor(rgba, hex.to_string()));
        }
    }

    let bar_y = swatches_y + 2.0 * (swatch_h + gap) + 8.0;
    let bar_w = 8.0 * swatch_w + 7.0 * gap;
    let bar_h = 18.0;
    let bar_x = x + pad;

    if click_x >= bar_x && click_x <= bar_x + bar_w && click_y >= bar_y && click_y <= bar_y + bar_h
    {
        let ratio = ((click_x - bar_x) / bar_w).clamp(0.0, 1.0);
        let hue = ratio * 360.0;
        let rgba = hsv_to_rgb(hue, 1.0, 1.0);
        let hex = rgba_to_hex(rgba);
        return Some(ColorPickerAction::SelectColor(rgba, hex));
    }

    Some(ColorPickerAction::Inside)
}

fn hsv_to_rgb(h: f64, s: f64, v: f64) -> (f64, f64, f64, f64) {
    let c = v * s;
    let x = c * (1.0 - ((h / 60.0) % 2.0 - 1.0).abs());
    let m = v - c;
    let (r1, g1, b1) = if h < 60.0 {
        (c, x, 0.0)
    } else if h < 120.0 {
        (x, c, 0.0)
    } else if h < 180.0 {
        (0.0, c, x)
    } else if h < 240.0 {
        (0.0, x, c)
    } else if h < 300.0 {
        (x, 0.0, c)
    } else {
        (c, 0.0, x)
    };
    (r1 + m, g1 + m, b1 + m, 1.0)
}

fn rgba_to_hex(c: (f64, f64, f64, f64)) -> String {
    let r = (c.0 * 255.0).round().clamp(0.0, 255.0) as u8;
    let g = (c.1 * 255.0).round().clamp(0.0, 255.0) as u8;
    let b = (c.2 * 255.0).round().clamp(0.0, 255.0) as u8;
    format!("#{:02x}{:02x}{:02x}", r, g, b)
}

use std::f64::consts::{FRAC_PI_2, PI};
