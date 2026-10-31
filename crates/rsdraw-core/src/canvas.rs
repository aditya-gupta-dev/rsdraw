use crate::rendering::{self, RenderData};
use cairo::Context;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU64, Ordering};
use std::{cell::RefCell, rc::Rc};

pub type Point = (f64, f64);
pub type Color = (f64, f64, f64, f64);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tool {
    Selection,
    Hand,
    Rectangle,
    Diamond,
    Circle,
    Triangle,
    Arrow,
    Line,
    Freehand,
    Text,
    Eraser,
}
impl Tool {
    pub fn from_name(s: &str) -> Option<Self> {
        Some(match s.to_ascii_lowercase().as_str() {
            "selection" | "select" => Self::Selection,
            "hand" | "pan" => Self::Hand,
            "rectangle" | "rect" | "box" => Self::Rectangle,
            "diamond" => Self::Diamond,
            "circle" | "ellipse" | "oval" => Self::Circle,
            "triangle" | "tri" => Self::Triangle,
            "arrow" => Self::Arrow,
            "line" => Self::Line,
            "freehand" | "free" | "pen" | "draw" => Self::Freehand,
            "text" | "type" | "font" => Self::Text,
            "eraser" | "erase" => Self::Eraser,
            _ => return None,
        })
    }
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FillStyle {
    Hachure,
    CrossHatch,
    Solid,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StrokeStyle {
    Solid,
    Dashed,
    Dotted,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Theme {
    Light,
    Dark,
}
#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum StrokeVariability {
    Constant,
    Variable,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Style {
    pub color: Color,
    pub width: f64,
    pub roughness: f64,
    pub fill: Color,
    pub fill_style: FillStyle,
    pub stroke_style: StrokeStyle,
    pub font: String,
    pub font_size: f64,
    pub theme: Theme,
    pub rounded: bool,
    pub stroke_variability: StrokeVariability,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
impl Bounds {
    pub fn between(a: Point, b: Point) -> Self {
        Self {
            x: a.0.min(b.0),
            y: a.1.min(b.1),
            w: (a.0 - b.0).abs(),
            h: (a.1 - b.1).abs(),
        }
    }
    pub fn contains(self, p: Point, pad: f64) -> bool {
        p.0 >= self.x - pad
            && p.0 <= self.x + self.w + pad
            && p.1 >= self.y - pad
            && p.1 <= self.y + self.h + pad
    }
    pub fn union(self, b: Self) -> Self {
        Self::between(
            (self.x.min(b.x), self.y.min(b.y)),
            (
                (self.x + self.w).max(b.x + b.w),
                (self.y + self.h).max(b.y + b.h),
            ),
        )
    }
    pub fn encloses(self, b: Self) -> bool {
        self.contains((b.x, b.y), 0.) && self.contains((b.x + b.w, b.y + b.h), 0.)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Shape {
        tool: Tool,
        start: Point,
        end: Point,
    },
    Stroke(Vec<Point>),
    Text {
        pos: Point,
        text: String,
    },
}
#[derive(Debug, Clone)]
pub struct Element {
    pub geometry: Geometry,
    pub style: Style,
    pub seed: u64,
    cache: RefCell<Option<Rc<CachedPaths>>>,
}
#[derive(Debug)]
struct CachedPaths {
    geometry: Geometry,
    style: Style,
    seed: u64,
    paths: Rc<RenderData>,
}
impl CachedPaths {
    fn matches(&self, element: &Element) -> bool {
        let a = &self.style;
        let b = &element.style;
        self.seed == element.seed
            && self.geometry == element.geometry
            && a.width == b.width
            && a.roughness == b.roughness
            && (a.fill.3 > 0.) == (b.fill.3 > 0.)
            && a.fill_style == b.fill_style
            && a.stroke_style == b.stroke_style
            && a.rounded == b.rounded
            && a.stroke_variability == b.stroke_variability
    }
}
impl PartialEq for Element {
    fn eq(&self, other: &Self) -> bool {
        self.geometry == other.geometry && self.style == other.style && self.seed == other.seed
    }
}
static SEED: AtomicU64 = AtomicU64::new(1234567);
impl Element {
    pub fn new(geometry: Geometry, style: Style) -> Self {
        Self {
            geometry,
            style,
            seed: SEED.fetch_add(7919, Ordering::Relaxed),
            cache: RefCell::new(None),
        }
    }
    pub fn bounds(&self) -> Bounds {
        match &self.geometry {
            Geometry::Shape { start, end, .. } => Bounds::between(*start, *end),
            Geometry::Stroke(p) => p
                .iter()
                .fold(None, |b: Option<Bounds>, &p| {
                    Some(b.map_or(Bounds::between(p, p), |b| b.union(Bounds::between(p, p))))
                })
                .unwrap_or(Bounds::between((0., 0.), (0., 0.))),
            Geometry::Text { pos, text } => {
                let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1).unwrap();
                let cr = Context::new(&surface).unwrap();
                let (w, h) = crate::text::measure(&cr, text, &self.style);
                Bounds {
                    x: pos.0,
                    y: pos.1,
                    w,
                    h,
                }
            }
        }
    }
    pub fn translate(&mut self, dx: f64, dy: f64) {
        let mv = |p: &mut Point| {
            p.0 += dx;
            p.1 += dy;
        };
        match &mut self.geometry {
            Geometry::Shape { start, end, .. } => {
                mv(start);
                mv(end);
            }
            Geometry::Stroke(p) => p.iter_mut().for_each(mv),
            Geometry::Text { pos, .. } => mv(pos),
        }
    }
    pub fn resize(&mut self, from: Bounds, to: Bounds) {
        let sx = to.w / from.w.max(1.);
        let sy = to.h / from.h.max(1.);
        let map = |p: &mut Point| {
            p.0 = to.x + (p.0 - from.x) * sx;
            p.1 = to.y + (p.1 - from.y) * sy;
        };
        match &mut self.geometry {
            Geometry::Shape { start, end, .. } => {
                map(start);
                map(end);
            }
            Geometry::Stroke(p) => p.iter_mut().for_each(map),
            Geometry::Text { pos, .. } => {
                map(pos);
                self.style.font_size = (self.style.font_size * sx.min(sy)).clamp(6., 256.);
            }
        }
    }
    pub fn hit(&self, p: Point, tolerance: f64) -> bool {
        let b = self.bounds();
        let tol = tolerance + self.style.width / 2.;
        if !b.contains(p, tol) {
            return false;
        }
        match &self.geometry {
            Geometry::Text { .. } => true,
            Geometry::Stroke(_) => {
                let origin = self.render_origin();
                if let Ok(paths) = self.paths() {
                    let outline: Vec<Point> = paths
                        .outline
                        .iter()
                        .map(|p| (p[0] + origin.0, p[1] + origin.1))
                        .collect();
                    if inside_polygon(p, &outline) {
                        return true;
                    }
                    return (0..outline.len()).any(|i| {
                        distance_to_segment(p, outline[i], outline[(i + 1) % outline.len()])
                            <= tolerance
                    });
                }
                false
            }
            Geometry::Shape { tool, start, end } => {
                if matches!(tool, Tool::Arrow | Tool::Line) {
                    if distance_to_segment(p, *start, *end) <= tol {
                        return true;
                    }
                    return *tool == Tool::Arrow
                        && arrow_wings(*start, *end, self.style.width)
                            .iter()
                            .any(|&a| distance_to_segment(p, *end, a) <= tol);
                }
                if *tool == Tool::Circle {
                    let rx = b.w / 2.;
                    let ry = b.h / 2.;
                    if rx < 0.1 || ry < 0.1 {
                        return distance_to_segment(p, *start, *end) <= tol;
                    }
                    let r =
                        (((p.0 - b.x - rx) / rx).powi(2) + ((p.1 - b.y - ry) / ry).powi(2)).sqrt();
                    return if self.style.fill.3 > 0. {
                        r <= 1. + tol / rx.min(ry)
                    } else {
                        (r - 1.).abs() * rx.min(ry) <= tol
                    };
                }
                let pts = polygon(*tool, b);
                if self.style.fill.3 > 0. && inside_polygon(p, &pts) {
                    return true;
                }
                (0..pts.len())
                    .any(|i| distance_to_segment(p, pts[i], pts[(i + 1) % pts.len()]) <= tol)
            }
        }
    }
    pub fn render_request(&self) -> (Point, rendering::Request) {
        let s = &self.style;
        let (origin, tool, width, height, end, points) = match &self.geometry {
            Geometry::Shape { tool, start, end } => {
                let b = Bounds::between(*start, *end);
                let linear = matches!(tool, Tool::Line | Tool::Arrow);
                let origin = if linear { *start } else { (b.x, b.y) };
                let name = match tool {
                    Tool::Rectangle => "rectangle",
                    Tool::Diamond => "diamond",
                    Tool::Circle => "ellipse",
                    Tool::Triangle => "triangle",
                    Tool::Arrow => "arrow",
                    Tool::Line => "line",
                    _ => unreachable!(),
                };
                (
                    origin,
                    name,
                    b.w,
                    b.h,
                    (end.0 - start.0, end.1 - start.1),
                    Vec::<Point>::new(),
                )
            }
            Geometry::Stroke(points) => {
                let origin = points.first().copied().unwrap_or((0., 0.));
                (
                    origin,
                    "freehand",
                    0.,
                    0.,
                    (0., 0.),
                    points
                        .iter()
                        .map(|p| (p.0 - origin.0, p.1 - origin.1))
                        .collect(),
                )
            }
            Geometry::Text { .. } => unreachable!("text is rendered with Pango"),
        };
        let rounded = s.rounded && matches!(tool, "rectangle" | "diamond" | "line" | "arrow");
        (
            origin,
            rendering::Request {
                tool: tool.into(),
                width,
                height,
                end: [end.0, end.1],
                points,
                seed: (self.seed & 0x7fffffff).max(1) as u32,
                stroke_width: s.width,
                roughness: s.roughness,
                filled: s.fill.3 > 0.,
                fill_style: match s.fill_style {
                    FillStyle::Hachure => "hachure",
                    FillStyle::CrossHatch => "cross-hatch",
                    FillStyle::Solid => "solid",
                }
                .into(),
                stroke_style: match s.stroke_style {
                    StrokeStyle::Solid => "solid",
                    StrokeStyle::Dashed => "dashed",
                    StrokeStyle::Dotted => "dotted",
                }
                .into(),
                rounded,
                variability: match s.stroke_variability {
                    StrokeVariability::Constant => "constant",
                    StrokeVariability::Variable => "variable",
                }
                .into(),
            },
        )
    }
    fn render_origin(&self) -> Point {
        match &self.geometry {
            Geometry::Stroke(points) => points.first().copied().unwrap_or((0., 0.)),
            Geometry::Shape { tool, start, end } => {
                if matches!(tool, Tool::Arrow | Tool::Line) {
                    *start
                } else {
                    (start.0.min(end.0), start.1.min(end.1))
                }
            }
            Geometry::Text { pos, .. } => *pos,
        }
    }
    fn paths(&self) -> Result<Rc<RenderData>, String> {
        if let Some(cache) = &*self.cache.borrow() {
            if cache.matches(self) {
                return Ok(cache.paths.clone());
            }
        }
        let data = if let Geometry::Stroke(points) = &self.geometry {
            rendering::freehand(
                points,
                self.style.width,
                self.style.stroke_variability == StrokeVariability::Variable,
                self.seed,
            )?
        } else {
            rendering::generate_request(&self.render_request().1)?
        };
        let paths = Rc::new(data);
        *self.cache.borrow_mut() = Some(Rc::new(CachedPaths {
            geometry: self.geometry.clone(),
            style: self.style.clone(),
            seed: self.seed,
            paths: paths.clone(),
        }));
        Ok(paths)
    }
    pub fn draw(&self, cr: &Context) {
        let _ = cr.save();
        let s = &self.style;
        cr.set_line_cap(cairo::LineCap::Round);
        cr.set_line_join(cairo::LineJoin::Round);
        if let Geometry::Text { pos, text } = &self.geometry {
            source(cr, display_color(s.color, s.theme));
            crate::text::draw(cr, *pos, text, s);
        } else {
            let origin = self.render_origin();
            match self.paths() {
                Ok(paths) => {
                    cr.translate(origin.0, origin.1);
                    for drawable in &paths.drawables {
                        for set in &drawable.sets {
                            cr.new_path();
                            for operation in &set.ops {
                                let d = &operation.data;
                                match operation.op.as_str() {
                                    "move" => cr.move_to(d[0], d[1]),
                                    "lineTo" => cr.line_to(d[0], d[1]),
                                    "bcurveTo" => cr.curve_to(d[0], d[1], d[2], d[3], d[4], d[5]),
                                    _ => unreachable!("unsupported upstream path operation"),
                                }
                            }
                            cr.set_dash(&[], 0.);
                            if set.r#type == "fillPath" {
                                source(
                                    cr,
                                    display_color(
                                        if matches!(self.geometry, Geometry::Stroke(_)) {
                                            s.color
                                        } else {
                                            s.fill
                                        },
                                        s.theme,
                                    ),
                                );
                                cr.set_fill_rule(
                                    if matches!(
                                        drawable.shape.as_str(),
                                        "curve" | "polygon" | "path"
                                    ) {
                                        cairo::FillRule::EvenOdd
                                    } else {
                                        cairo::FillRule::Winding
                                    },
                                );
                                let _ = cr.fill();
                            } else {
                                if set.r#type == "fillSketch" {
                                    source(cr, display_color(s.fill, s.theme));
                                    cr.set_line_width(drawable.options.fill_weight);
                                } else {
                                    if drawable.options.stroke == "none" {
                                        cr.new_path();
                                        continue;
                                    }
                                    source(cr, display_color(s.color, s.theme));
                                    cr.set_line_width(drawable.options.stroke_width);
                                    cr.set_dash(&drawable.options.stroke_line_dash, 0.);
                                }
                                let _ = cr.stroke();
                            }
                        }
                    }
                }
                Err(error) => eprintln!("drawing failed: {error}"),
            }
        }
        let _ = cr.restore();
    }
}

pub fn source(cr: &Context, c: Color) {
    cr.set_source_rgba(c.0, c.1, c.2, c.3);
}
pub fn distance(a: Point, b: Point) -> f64 {
    (a.0 - b.0).hypot(a.1 - b.1)
}
pub fn distance_to_segment(p: Point, a: Point, b: Point) -> f64 {
    let dx = b.0 - a.0;
    let dy = b.1 - a.1;
    let len = dx * dx + dy * dy;
    let t = if len < 1e-8 {
        0.
    } else {
        ((p.0 - a.0) * dx + (p.1 - a.1) * dy) / len
    }
    .clamp(0., 1.);
    distance(p, (a.0 + t * dx, a.1 + t * dy))
}
fn polygon(tool: Tool, b: Bounds) -> Vec<Point> {
    match tool {
        Tool::Diamond => vec![
            (b.x + b.w / 2., b.y),
            (b.x + b.w, b.y + b.h / 2.),
            (b.x + b.w / 2., b.y + b.h),
            (b.x, b.y + b.h / 2.),
        ],
        Tool::Triangle => vec![
            (b.x + b.w / 2., b.y),
            (b.x + b.w, b.y + b.h),
            (b.x, b.y + b.h),
        ],
        _ => vec![
            (b.x, b.y),
            (b.x + b.w, b.y),
            (b.x + b.w, b.y + b.h),
            (b.x, b.y + b.h),
        ],
    }
}
fn inside_polygon(p: Point, pts: &[Point]) -> bool {
    let mut inside = false;
    for i in 0..pts.len() {
        let a = pts[i];
        let b = pts[(i + 1) % pts.len()];
        if (a.1 > p.1) != (b.1 > p.1) && p.0 < (b.0 - a.0) * (p.1 - a.1) / (b.1 - a.1) + a.0 {
            inside = !inside;
        }
    }
    inside
}
fn arrow_wings(a: Point, b: Point, width: f64) -> [Point; 2] {
    let len = distance(a, b);
    let head = 25_f64.min(len * 0.5);
    let _ = width;
    let angle = (b.1 - a.1).atan2(b.0 - a.0);
    let spread = 20. * std::f64::consts::PI / 180.;
    [
        (
            b.0 - head * (angle - spread).cos(),
            b.1 - head * (angle - spread).sin(),
        ),
        (
            b.0 - head * (angle + spread).cos(),
            b.1 - head * (angle + spread).sin(),
        ),
    ]
}
pub fn constrained_shape(
    tool: Tool,
    origin: Point,
    mut end: Point,
    shift: bool,
    center: bool,
) -> (Point, Point) {
    let mut start = origin;
    if shift {
        let dx = end.0 - origin.0;
        let dy = end.1 - origin.1;
        if matches!(tool, Tool::Line | Tool::Arrow) {
            let angle =
                (dy.atan2(dx) / (std::f64::consts::PI / 12.)).round() * std::f64::consts::PI / 12.;
            let len = dx.hypot(dy);
            end = (origin.0 + len * angle.cos(), origin.1 + len * angle.sin());
        } else {
            let size = dx.abs().max(dy.abs());
            end = (
                origin.0 + size * if dx < 0. { -1. } else { 1. },
                origin.1 + size * if dy < 0. { -1. } else { 1. },
            );
        }
    }
    if center {
        start = (origin.0 - (end.0 - origin.0), origin.1 - (end.1 - origin.1));
    }
    (start, end)
}

/// Excalidraw's exact CSS invert(93%) hue-rotate(180deg) transform.
/// Alpha is preserved; the transparent desktop is never painted.
pub fn display_color(color: Color, theme: Theme) -> Color {
    if theme == Theme::Light || color.3 == 0. {
        return color;
    }
    let invert = |v: f64| {
        let v = (v * 255.).round();
        (v * 0.07 + (255. - v) * 0.93).clamp(0., 255.).round() / 255.
    };
    let (r, g, b) = (invert(color.0), invert(color.1), invert(color.2));
    let a = std::f64::consts::PI;
    let c = a.cos();
    let sn = a.sin();
    let matrix = [
        0.213 + c * 0.787 - sn * 0.213,
        0.715 - c * 0.715 - sn * 0.715,
        0.072 - c * 0.072 + sn * 0.928,
        0.213 - c * 0.213 + sn * 0.143,
        0.715 + c * 0.285 + sn * 0.14,
        0.072 - c * 0.072 - sn * 0.283,
        0.213 - c * 0.213 - sn * 0.787,
        0.715 - c * 0.715 + sn * 0.715,
        0.072 + c * 0.928 + sn * 0.072,
    ];
    let channel = |i: usize| {
        ((r * matrix[i] + g * matrix[i + 1] + b * matrix[i + 2]).clamp(0., 1.) * 255.).round()
            / 255.
    };
    (channel(0), channel(3), channel(6), color.3)
}

pub fn parse_color(name: &str) -> Option<Color> {
    let name = name.trim().to_ascii_lowercase();
    if let Some(hex) = name.strip_prefix('#') {
        if !hex.is_ascii() {
            return None;
        }
        let body = match hex.len() {
            3 | 4 => hex.chars().flat_map(|c| [c, c]).collect::<String>(),
            6 | 8 => hex.to_owned(),
            _ => return None,
        };
        let val = u32::from_str_radix(&body, 16).ok()?;
        let rgba = if body.len() == 6 {
            (val << 8) | 255
        } else {
            val
        };
        return Some((
            ((rgba >> 24) & 255) as f64 / 255.,
            ((rgba >> 16) & 255) as f64 / 255.,
            ((rgba >> 8) & 255) as f64 / 255.,
            (rgba & 255) as f64 / 255.,
        ));
    }
    let hex = match name.as_str() {
        "transparent" => return Some((0., 0., 0., 0.)),
        "black" => "#1e1e1e",
        "white" => "#ffffff",
        "red" => "#e03131",
        "green" => "#2f9e44",
        "blue" => "#1971c2",
        "yellow" => "#f08c00",
        "orange" => "#e8590c",
        "pink" => "#c2255c",
        "purple" => "#6741d9",
        "cyan" => "#0c8599",
        "gray" | "grey" => "#868e96",
        "darkgray" | "darkgrey" => "#495057",
        "magenta" => "#ae3ec9",
        "indigo" => "#4263eb",
        "teal" | "mint" => "#099268",
        "brown" => "#a05a2c",
        _ => return None,
    };
    parse_color(hex)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    pub fn style() -> Style {
        Style {
            color: parse_color("black").unwrap(),
            width: 2.,
            roughness: 1.,
            fill: (0., 0., 0., 0.),
            fill_style: FillStyle::Hachure,
            stroke_style: StrokeStyle::Solid,
            font: "sans-serif".into(),
            font_size: 20.,
            theme: Theme::Light,
            rounded: true,
            stroke_variability: StrokeVariability::Constant,
        }
    }
    #[test]
    fn dark_colors_match_reference_and_preserve_alpha() {
        let fixtures: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/excalidraw.json")).unwrap();
        for fixture in fixtures["colors"].as_array().unwrap() {
            let rgb = &fixture["rgb"];
            let input = (
                rgb[0].as_f64().unwrap() / 255.,
                rgb[1].as_f64().unwrap() / 255.,
                rgb[2].as_f64().unwrap() / 255.,
                0.75,
            );
            let actual = display_color(input, Theme::Dark);
            let expected = &fixture["expected"];
            assert_eq!((actual.0 * 255.).round(), expected["r"].as_f64().unwrap());
            assert_eq!((actual.1 * 255.).round(), expected["g"].as_f64().unwrap());
            assert_eq!((actual.2 * 255.).round(), expected["b"].as_f64().unwrap());
            assert_eq!(actual.3, 0.75);
            assert_eq!(display_color(input, Theme::Light), input);
        }
        let ink = display_color(parse_color("black").unwrap(), Theme::Dark);
        assert!(
            ink.0 > 0.8 && ink.1 > 0.8 && ink.2 > 0.8,
            "dark desktop needs light ink"
        );
    }
    #[test]
    fn every_palette_color_reaches_opaque_pixels() {
        for theme in [Theme::Dark, Theme::Light] {
            for name in [
                "black", "white", "red", "green", "blue", "yellow", "orange", "purple", "cyan",
            ] {
                let mut style = style();
                style.color = parse_color(name).unwrap();
                style.theme = theme;
                let e = Element::new(
                    Geometry::Shape {
                        tool: Tool::Rectangle,
                        start: (20., 20.),
                        end: (120., 100.),
                    },
                    style.clone(),
                );
                let mut surface =
                    cairo::ImageSurface::create(cairo::Format::ARgb32, 160, 140).unwrap();
                {
                    let cr = Context::new(&surface).unwrap();
                    e.draw(&cr);
                }
                let expected = display_color(style.color, theme);
                let data = surface.data().unwrap();
                // Cairo ARGB32 is native endian; channels of fully covered ink must survive.
                assert!(
                    data.chunks_exact(4).any(|pixel| {
                        let argb = u32::from_ne_bytes(pixel.try_into().unwrap());
                        let alpha = argb >> 24;
                        alpha == 255
                            && ((argb >> 16) & 255) == (expected.0 * 255.).round() as u32
                            && ((argb >> 8) & 255) == (expected.1 * 255.).round() as u32
                            && (argb & 255) == (expected.2 * 255.).round() as u32
                    }),
                    "missing opaque {name} ink in {theme:?}"
                );
            }
        }
    }
    #[test]
    fn hex_input_never_panics() {
        for s in ["#💥", "#é0", "#xyz", "#12345", "bad"] {
            assert!(parse_color(s).is_none());
        }
        assert_eq!(parse_color("#abcd"), parse_color("#aabbccdd"));
    }
    #[test]
    fn hit_testing_uses_outlines() {
        let mut e = Element::new(
            Geometry::Shape {
                tool: Tool::Rectangle,
                start: (10., 10.),
                end: (110., 110.),
            },
            style(),
        );
        assert!(e.hit((10., 50.), 4.));
        assert!(!e.hit((60., 60.), 4.));
        e.style.fill = parse_color("red").unwrap();
        assert!(e.hit((60., 60.), 4.));
    }
    #[test]
    fn constraints_work_in_reverse_direction() {
        let (a, b) = constrained_shape(Tool::Rectangle, (100., 100.), (70., 50.), true, true);
        assert_eq!(a, (150., 150.));
        assert_eq!(b, (50., 50.));
    }
    #[test]
    fn cached_geometry_is_shared_and_ink_changes_do_not_regenerate_it() {
        let mut element = Element::new(
            Geometry::Stroke(vec![(0., 0.), (20., 20.), (30., 10.)]),
            style(),
        );
        let first = element.paths().unwrap();
        let copy = element.clone();
        assert!(Rc::ptr_eq(&first, &copy.paths().unwrap()));
        element.style.color = parse_color("blue").unwrap();
        element.style.theme = Theme::Dark;
        assert!(Rc::ptr_eq(&first, &element.paths().unwrap()));
        if let Geometry::Stroke(points) = &mut element.geometry {
            points.push((50., 20.));
        }
        assert!(!Rc::ptr_eq(&first, &element.paths().unwrap()));
        assert!(
            Rc::ptr_eq(&first, &copy.paths().unwrap()),
            "editing a clone cannot invalidate the original"
        );
    }
    #[test]
    fn render_is_deterministic_and_not_empty() {
        let e = Element::new(
            Geometry::Shape {
                tool: Tool::Rectangle,
                start: (10., 10.),
                end: (110., 110.),
            },
            style(),
        );
        let render = || {
            let mut s = cairo::ImageSurface::create(cairo::Format::ARgb32, 130, 130).unwrap();
            {
                let cr = Context::new(&s).unwrap();
                e.draw(&cr);
            }
            let data = s.data().unwrap().to_vec();
            data
        };
        let first = render();
        assert_eq!(first, render());
        assert!(first.iter().any(|&v| v != 0));
    }
}
