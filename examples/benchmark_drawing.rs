//! Repeatable headless latency measurements of the actual native drawing path.
//! taskset -c 0-4 cargo run --release -j 5 --example benchmark_drawing
use canvas::*;
use rsdraw_core::canvas;
use std::time::Instant;
fn main() {
    let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1920, 1080).unwrap();
    let cr = cairo::Context::new(&surface).unwrap();
    let style = Style {
        color: parse_color("red").unwrap(),
        width: 2.,
        roughness: 1.,
        fill: (0., 0., 0., 0.),
        fill_style: FillStyle::Solid,
        stroke_style: StrokeStyle::Solid,
        font: "Excalifont".into(),
        font_size: 20.,
        theme: Theme::Dark,
        rounded: true,
        stroke_variability: StrokeVariability::Constant,
    };
    println!("tool,points,uncached_ms,cached_ms");
    for variability in [StrokeVariability::Constant, StrokeVariability::Variable] {
        for count in [32, 256, 1024, 4096] {
            let points = (0..count)
                .map(|i| {
                    let t = i as f64;
                    (t * 0.3 + 20., 300. + (t * 0.04).sin() * 100.)
                })
                .collect::<Vec<_>>();
            let mut style = style.clone();
            style.stroke_variability = variability;
            let begin = Instant::now();
            let samples = 5;
            let mut last = None;
            for _ in 0..samples {
                let e = Element::new(Geometry::Stroke(points.clone()), style.clone());
                e.draw(&cr);
                last = Some(e);
            }
            let uncached = begin.elapsed().as_secs_f64() * 1000. / samples as f64;
            let begin = Instant::now();
            for _ in 0..samples {
                last.as_ref().unwrap().draw(&cr);
            }
            let cached = begin.elapsed().as_secs_f64() * 1000. / samples as f64;
            println!("{variability:?},{count},{uncached:.3},{cached:.3}");
        }
    }
    let begin = Instant::now();
    for i in 0..100 {
        Element::new(
            Geometry::Shape {
                tool: Tool::Line,
                start: (20., 20.),
                end: (400. + i as f64, 300.),
            },
            style.clone(),
        )
        .draw(&cr);
    }
    println!("line,2,{:.3},-", begin.elapsed().as_secs_f64() * 10.);
}
