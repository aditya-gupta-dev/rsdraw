//! Native execution of Excalidraw's constant-width laser outline and SVG
//! coordinate conversion. Algorithm adapted from packages/laser-pointer/src;
//! Copyright (c) 2023 Excalidraw, MIT (assets/vendor/laser-pointer-LICENSE.txt).
//! Keep the arithmetic order/cap loops in sync with the reference fixtures.
use std::f64::consts::PI;
type Point = [f64; 2];
fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
fn mul(p: Point, s: f64) -> Point {
    [p[0] * s, p[1] * s]
}
fn mag(p: Point) -> f64 {
    (p[0] * p[0] + p[1] * p[1]).sqrt()
}
fn norm(p: Point) -> Point {
    let d = mag(p);
    [p[0] / d, p[1] / d]
}
fn rot(p: Point, a: f64) -> Point {
    [
        a.cos() * p[0] - a.sin() * p[1],
        a.sin() * p[0] + a.cos() * p[1],
    ]
}
fn angle(center: Point, a: Point, b: Point) -> f64 {
    (b[1] - center[1]).atan2(b[0] - center[0]) - (a[1] - center[1]).atan2(a[0] - center[0])
}
fn norm_angle(a: f64) -> f64 {
    a.sin().atan2(a.cos())
}
fn arc(center: Point, size: f64, from: f64, to: f64) -> Vec<Point> {
    let mut points = Vec::new();
    let mut theta = from;
    while theta <= to {
        points.push(add(center, mul(rot([1., 0.], theta), size)));
        theta += PI / 16.;
    }
    points
}
/// Same fixed-pressure, 0.5-streamline, no-simplification mode used by the
/// reference. Tail stabilization only moves buffers; it doesn't change geometry.
pub fn outline(input: &[(f64, f64)], width: f64) -> Vec<Point> {
    let origin = input.first().copied().unwrap_or((0., 0.));
    let mut points = Vec::with_capacity(input.len());
    let mut previous_raw = None;
    for &(x, y) in input {
        let raw = [x - origin.0, y - origin.1];
        if previous_raw == Some(raw) {
            continue;
        }
        previous_raw = Some(raw);
        let point = points
            .last()
            .copied()
            .map_or(raw, |last| add(last, mul(sub(raw, last), 0.5)));
        points.push(point);
    }
    let size = width * 0.5 * 1.4;
    match points.len() {
        0 => return vec![],
        1 => {
            if size < 0.5 {
                return vec![];
            }
            let mut result = arc(points[0], size, 0., PI * 2.);
            result.push(add(points[0], [size, 0.]));
            return result;
        }
        2 => {
            if size < 0.5 {
                return vec![];
            }
            let c = points[0];
            let n = points[1];
            let a = angle(c, [c[0], c[1] - 100.], n);
            let mut result = arc(c, size, a, PI + a);
            result.extend(arc(n, size, PI + a, PI * 2. + a));
            if let Some(&first) = result.first() {
                result.push(first);
            }
            return result;
        }
        _ => {}
    }
    let mut forward = Vec::with_capacity(points.len());
    let mut backward = Vec::with_capacity(points.len());
    let mut previous_speed = 0.;
    for i in 1..points.len() - 1 {
        let (p, c, n) = (points[i - 1], points[i], points[i + 1]);
        let d = mag(sub(p, c));
        let speed = previous_speed + (d - previous_speed) * 0.2;
        let pc = norm(sub(p, c));
        let nc = norm(sub(n, c));
        let p1pc = rot(pc, PI / 2.);
        let p2pc = rot(pc, -PI / 2.);
        let p1nc = rot(nc, PI / 2.);
        let p2nc = rot(nc, -PI / 2.);
        let ft = add(p1pc, p2nc);
        let bt = add(p2pc, p1nc);
        let pa_pc = add(c, mul(if mag(ft) == 0. { pc } else { norm(ft) }, size));
        let pa_nc = add(c, mul(if mag(bt) == 0. { nc } else { norm(bt) }, size));
        let corner = norm_angle(angle(c, p, n));
        let threshold = (75. / 180.) * PI * if speed > 35. { 0.5 } else { 1. };
        if corner.abs() < threshold {
            let turn = norm_angle(PI - corner).abs();
            if turn == 0. {
                continue;
            }
            if corner < 0. {
                backward.extend([add(c, mul(p2pc, size)), pa_nc]);
                let mut theta = 0.;
                while theta <= turn {
                    forward.push(add(c, rot(mul(p1pc, size), theta)));
                    theta += turn / 4.;
                }
                let mut theta = turn;
                while theta >= 0. {
                    backward.push(add(c, rot(mul(p1pc, size), theta)));
                    theta -= turn / 4.;
                }
                backward.extend([pa_nc, add(c, mul(p1nc, size))]);
            } else {
                forward.extend([add(c, mul(p1pc, size)), pa_pc]);
                let mut theta = 0.;
                while theta <= turn {
                    backward.push(add(c, rot(mul(p1pc, -size), -theta)));
                    theta += turn / 4.;
                }
                let mut theta = turn;
                while theta >= 0. {
                    forward.push(add(c, rot(mul(p1pc, -size), -theta)));
                    theta -= turn / 4.;
                }
                forward.extend([pa_pc, add(c, mul(p2nc, size))]);
            }
        } else {
            forward.push(pa_pc);
            backward.push(pa_nc);
        }
        previous_speed = speed;
    }
    let first = points[0];
    let second = points[1];
    let ultimate = points[points.len() - 1];
    let penultimate = points[points.len() - 2];
    let pp_fs = rot(norm(sub(second, first)), -PI / 2.);
    let pp_pu = rot(norm(sub(penultimate, ultimate)), PI / 2.);
    let mut start = Vec::new();
    if size > 0.1 {
        let mut theta = 0.;
        while theta <= PI {
            start.push(add(first, rot(mul(pp_fs, size), -theta)));
            theta += PI / 16.;
        }
        start.reverse();
        start.insert(0, add(first, mul(pp_fs, -size)));
    } else {
        start.push(first);
    }
    let mut end = Vec::new();
    let mut theta = 0.;
    while theta <= PI * 3. {
        end.push(add(ultimate, rot(mul(pp_pu, -size), -theta)));
        theta += PI / 16.;
    }
    let first_cap = start[0];
    start.extend(forward);
    start.extend(end.into_iter().rev());
    start.extend(backward.into_iter().rev());
    start.push(first_cap);
    start
}
/// Preserve the reference's SVG regex truncation (not rounding). JS-compliant
/// shortest formatting matters at decimal boundaries such as 1.15 and exponents.
pub fn trim_coordinate(value: f64) -> f64 {
    if !value.is_finite() {
        return value;
    }
    let mut buffer = ryu_js::Buffer::new();
    let text = buffer.format(value);
    if let Some(dot) = text.find('.') {
        let decimal_end = text[dot + 1..]
            .bytes()
            .take_while(u8::is_ascii_digit)
            .count()
            + dot
            + 1;
        let end = (dot + 3).min(decimal_end);
        text[..end].parse().unwrap_or(value)
    } else {
        value
    }
}
