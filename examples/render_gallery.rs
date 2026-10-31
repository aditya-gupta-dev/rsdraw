//! Render the actual Cairo/Pango engine without opening an overlay.
//! cargo run --example render_gallery -- /tmp/rsdraw-gallery.png
use canvas::*;
use rsdraw_core::{canvas, fonts};
fn main() {
    fonts::register().expect("register bundled fonts");
    let output = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "/tmp/rsdraw-gallery.png".into());
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1200, 760).unwrap();
    let cr = cairo::Context::new(&surface).unwrap();
    let dark = std::env::args().any(|arg| arg == "--dark");
    cr.set_source_rgb(
        if dark { 0.07 } else { 1. },
        if dark { 0.07 } else { 1. },
        if dark { 0.08 } else { 1. },
    );
    cr.paint().unwrap();
    let mut style = Style {
        color: parse_color("black").unwrap(),
        width: 2.,
        roughness: 1.,
        fill: parse_color("transparent").unwrap(),
        fill_style: FillStyle::Hachure,
        stroke_style: StrokeStyle::Solid,
        font: "Excalifont".into(),
        font_size: 36.,
        theme: if dark { Theme::Dark } else { Theme::Light },
        rounded: true,
        stroke_variability: StrokeVariability::Constant,
    };
    Element::new(
        Geometry::Text {
            pos: (40., 24.),
            text: "A little less perfect. A lot more human.".into(),
        },
        style.clone(),
    )
    .draw(&cr);
    style.font_size = 20.;
    for (i, (tool, label)) in [
        (Tool::Rectangle, "Rectangle"),
        (Tool::Diamond, "Diamond"),
        (Tool::Circle, "Ellipse"),
        (Tool::Triangle, "Triangle"),
    ]
    .into_iter()
    .enumerate()
    {
        let x = 50. + i as f64 * 290.;
        Element::new(
            Geometry::Shape {
                tool,
                start: (x, 130.),
                end: (x + 210., 270.),
            },
            style.clone(),
        )
        .draw(&cr);
        Element::new(
            Geometry::Text {
                pos: (x + 30., 290.),
                text: label.into(),
            },
            style.clone(),
        )
        .draw(&cr);
    }
    for (i, fill) in [FillStyle::Hachure, FillStyle::CrossHatch, FillStyle::Solid]
        .into_iter()
        .enumerate()
    {
        let x = 50. + i as f64 * 290.;
        let mut s = style.clone();
        s.fill = parse_color(["#a5d8ff", "#b2f2bb", "#ffc9c9"][i]).unwrap();
        s.fill_style = fill;
        Element::new(
            Geometry::Shape {
                tool: Tool::Rectangle,
                start: (x, 370.),
                end: (x + 210., 510.),
            },
            s,
        )
        .draw(&cr);
    }
    Element::new(
        Geometry::Shape {
            tool: Tool::Arrow,
            start: (940., 380.),
            end: (1130., 480.),
        },
        style.clone(),
    )
    .draw(&cr);
    let points = (0..70)
        .map(|i| {
            let t = i as f64 / 69.;
            (
                60. + t * 490.,
                610. + (t * std::f64::consts::TAU * 1.4).sin() * 25.,
            )
        })
        .collect();
    let mut pen = style.clone();
    pen.width = 3.;
    pen.color = parse_color("blue").unwrap();
    Element::new(Geometry::Stroke(points), pen).draw(&cr);
    Element::new(
        Geometry::Text {
            pos: (650., 570.),
            text: "Hand-drawn text\nLatin · Ελληνικά · Кириллица".into(),
        },
        style,
    )
    .draw(&cr);
    surface
        .write_to_png(&mut std::fs::File::create(&output).unwrap())
        .unwrap();
    println!("{output}");
}
