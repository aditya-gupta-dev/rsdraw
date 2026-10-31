//! perfect-freehand 1.2.0, specialized to Excalidraw's simulated-pressure options.
//! Copyright (c) 2021 Steve Ruiz, MIT; assets/vendor/perfect-freehand-LICENSE.txt.
use std::f64::consts::PI;
type Point = [f64; 2];
fn add(a: Point, b: Point) -> Point {
    [a[0] + b[0], a[1] + b[1]]
}
fn sub(a: Point, b: Point) -> Point {
    [a[0] - b[0], a[1] - b[1]]
}
fn mul(a: Point, b: f64) -> Point {
    [a[0] * b, a[1] * b]
}
fn perp(a: Point) -> Point {
    [a[1], -a[0]]
}
fn dot(a: Point, b: Point) -> f64 {
    a[0] * b[0] + a[1] * b[1]
}
fn norm(a: Point) -> Point {
    let d = a[0].hypot(a[1]);
    [a[0] / d, a[1] / d]
}
fn lerp(a: Point, b: Point, t: f64) -> Point {
    add(a, mul(sub(b, a), t))
}
fn rotate(p: Point, c: Point, r: f64) -> Point {
    let s = r.sin();
    let co = r.cos();
    let x = p[0] - c[0];
    let y = p[1] - c[1];
    [x * co - y * s + c[0], x * s + y * co + c[1]]
}
#[derive(Clone)]
struct StrokePoint {
    point: Point,
    pressure: f64,
    vector: Point,
    distance: f64,
    running_length: f64,
}
fn stroke_points(input: &[Point], size: f64) -> Vec<StrokePoint> {
    if input.is_empty() {
        return vec![];
    }
    let mut points = input.to_vec();
    if points.len() == 2 {
        let end = points[1];
        points.truncate(1);
        for i in 1..5 {
            points.push(lerp(points[0], end, i as f64 / 4.));
        }
    }
    if points.len() == 1 {
        points.push(add(points[0], [1., 1.]));
    }
    let mut result = vec![StrokePoint {
        point: points[0],
        pressure: 0.25,
        vector: [1., 1.],
        distance: 0.,
        running_length: 0.,
    }];
    let mut reached_size = false;
    let mut running_length = 0.;
    for i in 1..points.len() {
        let previous = result.last().unwrap();
        let point = if i == points.len() - 1 {
            points[i]
        } else {
            lerp(previous.point, points[i], 0.15 + (1. - 0.5) * 0.85)
        };
        if previous.point == point {
            continue;
        }
        let distance = (point[1] - previous.point[1]).hypot(point[0] - previous.point[0]);
        running_length += distance;
        if i < points.len() - 1 && !reached_size {
            if running_length < size {
                continue;
            }
            reached_size = true;
        }
        let vector = norm(sub(previous.point, point));
        result.push(StrokePoint {
            point,
            pressure: 0.5,
            vector,
            distance,
            running_length,
        });
    }
    result[0].vector = result.get(1).map_or([0., 0.], |p| p.vector);
    result
}
fn radius(size: f64, pressure: f64) -> f64 {
    size * ((0.5 - 0.6 * (0.5 - pressure)) * PI / 2.).sin()
}
fn pressure(previous: f64, distance: f64, size: f64) -> f64 {
    let speed = (distance / size).min(1.);
    let target = (1. - speed).min(1.);
    (previous + (target - previous) * (speed * 0.275)).min(1.)
}
pub fn outline(input: &[(f64, f64)], width: f64) -> Vec<Point> {
    let size = width * 0.5 * 4.25;
    if input.is_empty() || size <= 0. {
        return vec![];
    }
    let origin = input[0];
    let relative: Vec<_> = input
        .iter()
        .map(|p| [p.0 - origin.0, p.1 - origin.1])
        .collect();
    let points = stroke_points(&relative, size);
    let total = points.last().unwrap().running_length;
    let min_distance = (size * 0.5).powi(2);
    let mut left = Vec::new();
    let mut right = Vec::new();
    let mut previous_pressure = points[0].pressure;
    for p in points.iter().take(10) {
        previous_pressure =
            (previous_pressure + pressure(previous_pressure, p.distance, size)) / 2.;
    }
    let mut r = radius(size, points.last().unwrap().pressure);
    let mut first_radius = None;
    let mut previous_vector = points[0].vector;
    let mut previous_left = points[0].point;
    let mut previous_right = previous_left;
    let mut sharp = false;
    const FIXED_PI: f64 = PI + 0.0001;
    for (i, p) in points.iter().enumerate() {
        if i < points.len() - 1 && total - p.running_length < 3. {
            continue;
        }
        let current_pressure = pressure(previous_pressure, p.distance, size);
        r = radius(size, current_pressure);
        if first_radius.is_none() {
            first_radius = Some(r);
        }
        r = r.max(0.01);
        let next = points.get(i + 1).unwrap_or(p).vector;
        let next_dot = if i < points.len() - 1 {
            dot(p.vector, next)
        } else {
            1.
        };
        let previous_sharp = dot(p.vector, previous_vector) < 0. && !sharp;
        let next_sharp = next_dot < 0.;
        if previous_sharp || next_sharp {
            let offset = mul(perp(previous_vector), r);
            let mut t = 0.;
            while t <= 1. {
                previous_left = rotate(sub(p.point, offset), p.point, FIXED_PI * t);
                left.push(previous_left);
                previous_right = rotate(add(p.point, offset), p.point, FIXED_PI * -t);
                right.push(previous_right);
                t += 1. / 13.;
            }
            if next_sharp {
                sharp = true;
            }
            continue;
        }
        sharp = false;
        if i == points.len() - 1 {
            let offset = mul(perp(p.vector), r);
            left.push(sub(p.point, offset));
            right.push(add(p.point, offset));
            continue;
        }
        let offset = mul(perp(lerp(next, p.vector, next_dot)), r);
        let l = sub(p.point, offset);
        let d = sub(previous_left, l);
        if i <= 1 || dot(d, d) > min_distance {
            left.push(l);
            previous_left = l;
        }
        let r = add(p.point, offset);
        let d = sub(previous_right, r);
        if i <= 1 || dot(d, d) > min_distance {
            right.push(r);
            previous_right = r;
        }
        previous_pressure = current_pressure;
        previous_vector = p.vector;
    }
    let first = points[0].point;
    let last = if points.len() > 1 {
        points.last().unwrap().point
    } else {
        add(first, [1., 1.])
    };
    if points.len() == 1 {
        let p = add(
            first,
            mul(norm(perp(sub(first, last))), -first_radius.unwrap_or(r)),
        );
        let mut dot = Vec::new();
        let mut t = 1. / 13.;
        while t <= 1. {
            dot.push(rotate(p, first, FIXED_PI * 2. * t));
            t += 1. / 13.;
        }
        return dot;
    }
    let mut start_cap = Vec::new();
    let mut t = 1. / 13.;
    while t <= 1. {
        start_cap.push(rotate(right[0], first, FIXED_PI * t));
        t += 1. / 13.;
    }
    let direction = perp(mul(points.last().unwrap().vector, -1.));
    let p = add(last, mul(direction, r));
    let mut t = 1. / 29.;
    while t < 1. {
        left.push(rotate(p, last, FIXED_PI * 3. * t));
        t += 1. / 29.;
    }
    left.extend(right.into_iter().rev());
    left.extend(start_cap);
    left
}
