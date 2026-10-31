//! Native port of the RoughJS 4.6.4 operations used by Excalidraw.
//! Copyright (c) 2017 Preet Shihn, MIT; see assets/vendor/roughjs-LICENSE.txt.
//! Seed consumption and arithmetic order are part of the rendering contract.
use crate::rendering::{Drawable, Op, OpSet, Options, Request};
use std::f64::consts::PI;
type Point = [f64; 2];

#[derive(Clone)]
struct Rough {
    seed: u32,
    random: u32,
    roughness: f64,
    preserve: bool,
    single: bool,
    width: f64,
    gap: f64,
    dash: Vec<f64>,
    fill_weight: f64,
}
impl Rough {
    fn new(e: &Request) -> Self {
        let max = e.width.max(e.height);
        let min = e.width.min(e.height);
        let linear = matches!(e.tool.as_str(), "line" | "arrow");
        let roughness =
            if (min >= 20. && max >= 50.) || (min >= 15. && e.rounded) || (linear && max >= 50.) {
                e.roughness
            } else {
                (e.roughness / if max < 10. { 3. } else { 2. }).min(2.5)
            };
        Self {
            seed: e.seed,
            random: e.seed,
            roughness,
            preserve: (!linear && e.rounded) || e.roughness < 2.,
            single: e.stroke_style != "solid",
            width: e.stroke_width + if e.stroke_style == "solid" { 0. } else { 0.5 },
            gap: e.stroke_width * 4.,
            fill_weight: e.stroke_width / 2.,
            dash: match e.stroke_style.as_str() {
                "dashed" => vec![8., 8. + e.stroke_width],
                "dotted" => vec![1.5, 6. + e.stroke_width],
                _ => vec![],
            },
        }
    }
    fn next(&mut self) -> f64 {
        self.random = self.random.wrapping_mul(48271);
        (self.random & 0x7fffffff) as f64 / 2147483648.
    }
    fn range(&mut self, min: f64, max: f64) -> f64 {
        self.roughness * (self.next() * (max - min) + min)
    }
    fn offset(&mut self, value: f64) -> f64 {
        self.range(-value, value)
    }
    fn gain_offset(&mut self, value: f64, gain: f64) -> f64 {
        self.roughness * gain * (self.next() * (value + value) - value)
    }
    fn line(&mut self, a: Point, b: Point, overlay: bool) -> Vec<Op> {
        let length_sq = (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2);
        let length = length_sq.sqrt();
        let gain = if length < 200. {
            1.
        } else if length > 500. {
            0.4
        } else {
            -0.0016668 * length + 1.233334
        };
        let offset = if 400. > length_sq { length / 10. } else { 2. };
        let divergence = 0.2 + self.next() * 0.2;
        let mx = self.gain_offset(2. * (b[1] - a[1]) / 200., gain);
        let my = self.gain_offset(2. * (a[0] - b[0]) / 200., gain);
        let jitter = if overlay { offset / 2. } else { offset };
        let start = if self.preserve {
            a
        } else {
            [
                a[0] + self.gain_offset(jitter, gain),
                a[1] + self.gain_offset(jitter, gain),
            ]
        };
        let data = vec![
            mx + a[0] + (b[0] - a[0]) * divergence + self.gain_offset(jitter, gain),
            my + a[1] + (b[1] - a[1]) * divergence + self.gain_offset(jitter, gain),
            mx + a[0] + 2. * (b[0] - a[0]) * divergence + self.gain_offset(jitter, gain),
            my + a[1] + 2. * (b[1] - a[1]) * divergence + self.gain_offset(jitter, gain),
            b[0] + if self.preserve {
                0.
            } else {
                self.gain_offset(jitter, gain)
            },
            b[1] + if self.preserve {
                0.
            } else {
                self.gain_offset(jitter, gain)
            },
        ];
        vec![op("move", start.to_vec()), op("bcurveTo", data)]
    }
    fn double_line(&mut self, a: Point, b: Point, filling: bool) -> Vec<Op> {
        let mut ops = self.line(a, b, false);
        if filling || !self.single {
            ops.extend(self.line(a, b, true));
        }
        ops
    }
    fn linear(&mut self, points: &[Point], close: bool) -> Vec<Op> {
        let mut ops = Vec::new();
        for pair in points.windows(2) {
            ops.extend(self.double_line(pair[0], pair[1], false));
        }
        if close && points.len() > 2 {
            ops.extend(self.double_line(*points.last().unwrap(), points[0], false));
        }
        ops
    }
    fn bezier(&mut self, current: Point, p: [f64; 6]) -> Vec<Op> {
        let mut ops = Vec::new();
        for i in 0..if self.single { 1 } else { 2 } {
            let jitter = if i == 0 { 2. } else { 2.3 };
            let start = if i == 0 || self.preserve {
                current
            } else {
                [current[0] + self.offset(2.), current[1] + self.offset(2.)]
            };
            let end = if self.preserve {
                [p[4], p[5]]
            } else {
                [p[4] + self.offset(jitter), p[5] + self.offset(jitter)]
            };
            ops.push(op("move", start.to_vec()));
            ops.push(op(
                "bcurveTo",
                vec![
                    p[0] + self.offset(jitter),
                    p[1] + self.offset(jitter),
                    p[2] + self.offset(jitter),
                    p[3] + self.offset(jitter),
                    end[0],
                    end[1],
                ],
            ));
        }
        ops
    }
    fn path(&mut self, segments: &[Op]) -> Vec<Op> {
        let mut current = [0., 0.];
        let mut ops = Vec::new();
        for segment in segments {
            let d = &segment.data;
            match segment.op.as_str() {
                "move" => current = [d[0], d[1]],
                "lineTo" => {
                    ops.extend(self.double_line(current, [d[0], d[1]], false));
                    current = [d[0], d[1]];
                }
                "bcurveTo" => {
                    ops.extend(self.bezier(current, d.as_slice().try_into().unwrap()));
                    current = [d[4], d[5]];
                }
                _ => unreachable!(),
            }
        }
        ops
    }
    fn curve_with_offset(&mut self, points: &[Point], offset: f64) -> Vec<Op> {
        let mut ps = Vec::new();
        let p = points[0];
        ps.push([p[0] + self.offset(offset), p[1] + self.offset(offset)]);
        ps.push([p[0] + self.offset(offset), p[1] + self.offset(offset)]);
        for (i, p) in points.iter().enumerate().skip(1) {
            ps.push([p[0] + self.offset(offset), p[1] + self.offset(offset)]);
            if i == points.len() - 1 {
                ps.push([p[0] + self.offset(offset), p[1] + self.offset(offset)]);
            }
        }
        curve(&ps)
    }
    fn curved_line(&mut self, points: &[Point]) -> Vec<Op> {
        let mut ops = self.curve_with_offset(points, 1. + self.roughness * 0.2);
        if !self.single {
            let mut second = self.clone();
            second.random = self.seed.wrapping_add(1);
            ops.extend(second.curve_with_offset(points, 1.5 * (1. + self.roughness * 0.22)));
        }
        ops
    }
    fn ellipse_points(
        &mut self,
        increment: f64,
        center: Point,
        r: Point,
        offset: f64,
        overlap: f64,
    ) -> (Vec<Point>, Vec<Point>) {
        let [cx, cy] = center;
        let [rx, ry] = r;
        let mut all = Vec::new();
        let mut core = Vec::new();
        if self.roughness == 0. {
            let increment = increment / 4.;
            all.push([cx + rx * (-increment).cos(), cy + ry * (-increment).sin()]);
            let mut angle: f64 = 0.;
            while angle <= PI * 2. {
                let p = [cx + rx * angle.cos(), cy + ry * angle.sin()];
                core.push(p);
                all.push(p);
                angle += increment;
            }
            all.push([cx + rx, cy]);
            all.push([cx + rx * increment.cos(), cy + ry * increment.sin()]);
        } else {
            let rad = self.offset(0.5) - PI / 2.;
            all.push([
                self.offset(offset) + cx + 0.9 * rx * (rad - increment).cos(),
                self.offset(offset) + cy + 0.9 * ry * (rad - increment).sin(),
            ]);
            let end = PI * 2. + rad - 0.01;
            let mut angle = rad;
            while angle < end {
                let p = [
                    self.offset(offset) + cx + rx * angle.cos(),
                    self.offset(offset) + cy + ry * angle.sin(),
                ];
                core.push(p);
                all.push(p);
                angle += increment;
            }
            all.push([
                self.offset(offset) + cx + rx * (rad + PI * 2. + overlap * 0.5).cos(),
                self.offset(offset) + cy + ry * (rad + PI * 2. + overlap * 0.5).sin(),
            ]);
            all.push([
                self.offset(offset) + cx + 0.98 * rx * (rad + overlap).cos(),
                self.offset(offset) + cy + 0.98 * ry * (rad + overlap).sin(),
            ]);
            all.push([
                self.offset(offset) + cx + 0.9 * rx * (rad + overlap * 0.5).cos(),
                self.offset(offset) + cy + 0.9 * ry * (rad + overlap * 0.5).sin(),
            ]);
        }
        (all, core)
    }
    fn ellipse(&mut self, increment: f64, center: Point, r: Point) -> (Vec<Op>, Vec<Point>) {
        let upper = self.range(0.4, 1.);
        let overlap = increment * self.range(0.1, upper);
        let (points, core) = self.ellipse_points(increment, center, r, 1., overlap);
        let mut ops = curve(&points);
        if !self.single && self.roughness != 0. {
            let (second, _) = self.ellipse_points(increment, center, r, 1.5, 0.);
            ops.extend(curve(&second));
        }
        (ops, core)
    }
    fn solid_polygon(&mut self, points: &[Point]) -> OpSet {
        let ops = points
            .iter()
            .enumerate()
            .map(|(i, p)| {
                op(
                    if i == 0 { "move" } else { "lineTo" },
                    vec![p[0] + self.offset(2.), p[1] + self.offset(2.)],
                )
            })
            .collect();
        set("fillPath", ops)
    }
    fn hachure(&mut self, points: &[Point], cross: bool) -> OpSet {
        // The second pass rotates the same polygon back and forth, just as
        // hachure-fill does. Keeping those floating-point operations matters.
        let mut polygon = points.to_vec();
        let mut ops = self.hachure_pass(&mut polygon, 49.);
        if cross {
            ops.extend(self.hachure_pass(&mut polygon, 139.));
        }
        set("fillSketch", ops)
    }
    fn hachure_pass(&mut self, points: &mut [Point], angle: f64) -> Vec<Op> {
        let gap = self.gap.max(0.1);
        let step = if self.roughness >= 1. && self.next() > 0.7 {
            gap
        } else {
            1.
        };
        rotate(points, angle);
        let lines = scan_lines(points, gap, step);
        rotate(points, -angle);
        let mut ops = Vec::new();
        for mut line in lines {
            rotate(&mut line, -angle);
            ops.extend(self.double_line(line[0], line[1], true));
        }
        ops
    }
    fn drawable(&self, shape: &str, sets: Vec<OpSet>) -> Drawable {
        Drawable {
            shape: shape.into(),
            sets,
            options: Options {
                stroke: "#000000".into(),
                stroke_width: self.width,
                fill_weight: self.fill_weight,
                stroke_line_dash: self.dash.clone(),
            },
        }
    }
}
fn op(name: &str, data: Vec<f64>) -> Op {
    Op {
        op: name.into(),
        data,
    }
}
fn set(kind: &str, ops: Vec<Op>) -> OpSet {
    OpSet {
        r#type: kind.into(),
        ops,
    }
}
fn curve(points: &[Point]) -> Vec<Op> {
    let mut ops = Vec::new();
    if points.len() > 3 {
        ops.push(op("move", points[1].to_vec()));
        for i in 1..points.len() - 2 {
            ops.push(op(
                "bcurveTo",
                vec![
                    points[i][0] + (points[i + 1][0] - points[i - 1][0]) / 6.,
                    points[i][1] + (points[i + 1][1] - points[i - 1][1]) / 6.,
                    points[i + 1][0] + (points[i][0] - points[i + 2][0]) / 6.,
                    points[i + 1][1] + (points[i][1] - points[i + 2][1]) / 6.,
                    points[i + 1][0],
                    points[i + 1][1],
                ],
            ));
        }
    }
    ops
}
fn rotate(points: &mut [Point], degrees: f64) {
    let angle = PI / 180. * degrees;
    let c = angle.cos();
    let s = angle.sin();
    for p in points {
        let [x, y] = *p;
        *p = [x * c - y * s, x * s + y * c];
    }
}
#[derive(Clone)]
struct Edge {
    ymin: f64,
    ymax: f64,
    x: f64,
    slope: f64,
}
fn scan_lines(points: &[Point], gap: f64, step: f64) -> Vec<[Point; 2]> {
    let mut edges = Vec::new();
    for i in 0..points.len() {
        let a = points[i];
        let b = points[(i + 1) % points.len()];
        if a[1] != b[1] {
            let ymin = a[1].min(b[1]);
            edges.push(Edge {
                ymin,
                ymax: a[1].max(b[1]),
                x: if ymin == a[1] { a[0] } else { b[0] },
                slope: (b[0] - a[0]) / (b[1] - a[1]),
            });
        }
    }
    edges.sort_by(|a, b| {
        a.ymin
            .total_cmp(&b.ymin)
            .then(a.x.total_cmp(&b.x))
            .then(a.ymax.total_cmp(&b.ymax))
    });
    if edges.is_empty() {
        return vec![];
    }
    let mut y = edges[0].ymin;
    let mut index = 0;
    let mut iteration = 0_u64;
    let mut active: Vec<Edge> = Vec::new();
    let mut lines = Vec::new();
    while index < edges.len() || !active.is_empty() {
        while index < edges.len() && edges[index].ymin <= y {
            active.push(edges[index].clone());
            index += 1;
        }
        active.retain(|e| e.ymax > y);
        active.sort_by(|a, b| a.x.total_cmp(&b.x));
        if step != 1. || (iteration as f64) % gap == 0. {
            for pair in active.chunks_exact(2) {
                lines.push([
                    [(pair[0].x + 0.5).floor(), y],
                    [(pair[1].x + 0.5).floor(), y],
                ]);
            }
        }
        y += step;
        for e in &mut active {
            e.x += step * e.slope;
        }
        iteration += 1;
    }
    lines
}
fn lerp(a: Point, b: Point, t: f64) -> Point {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}
fn distance_sq(a: Point, b: Point) -> f64 {
    (a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2)
}
// points-on-curve 0.2.0 subdivision and Ramer–Douglas–Peucker simplification.
fn flatten(p: [Point; 4], out: &mut Vec<Point>) {
    let ux = (3. * p[1][0] - 2. * p[0][0] - p[3][0]).powi(2);
    let uy = (3. * p[1][1] - 2. * p[0][1] - p[3][1]).powi(2);
    let vx = (3. * p[2][0] - 2. * p[3][0] - p[0][0]).powi(2);
    let vy = (3. * p[2][1] - 2. * p[3][1] - p[0][1]).powi(2);
    if ux.max(vx) + uy.max(vy) < 1. {
        if out
            .last()
            .is_none_or(|last| distance_sq(*last, p[0]).sqrt() > 1.)
        {
            out.push(p[0]);
        }
        out.push(p[3]);
    } else {
        let q1 = lerp(p[0], p[1], 0.5);
        let q2 = lerp(p[1], p[2], 0.5);
        let q3 = lerp(p[2], p[3], 0.5);
        let r1 = lerp(q1, q2, 0.5);
        let r2 = lerp(q2, q3, 0.5);
        let mid = lerp(r1, r2, 0.5);
        flatten([p[0], q1, r1, mid], out);
        flatten([mid, r2, q3, p[3]], out);
    }
}
fn simplify(points: &[Point], epsilon: f64, out: &mut Vec<Point>) {
    let first = points[0];
    let last = *points.last().unwrap();
    let length = distance_sq(first, last);
    let mut max = 0.;
    let mut index = 1;
    for (i, p) in points.iter().enumerate().take(points.len() - 1).skip(1) {
        let d = if length == 0. {
            distance_sq(*p, first)
        } else {
            let t = (((p[0] - first[0]) * (last[0] - first[0])
                + (p[1] - first[1]) * (last[1] - first[1]))
                / length)
                .clamp(0., 1.);
            distance_sq(*p, lerp(first, last, t))
        };
        if d > max {
            max = d;
            index = i;
        }
    }
    if max.sqrt() > epsilon {
        simplify(&points[..=index], epsilon, out);
        simplify(&points[index..], epsilon, out);
    } else {
        if out.is_empty() {
            out.push(first);
        }
        out.push(last);
    }
}
fn path_points(segments: &[Op], roughness: f64) -> Vec<Point> {
    let mut points = Vec::new();
    let mut pending = Vec::new();
    for segment in segments {
        let d = &segment.data;
        if segment.op == "bcurveTo" {
            if pending.is_empty() {
                pending.push(*points.last().unwrap());
            }
            pending.extend([[d[0], d[1]], [d[2], d[3]], [d[4], d[5]]]);
        } else {
            flush_curve(&mut pending, &mut points);
            points.push([d[0], d[1]]);
        }
    }
    flush_curve(&mut pending, &mut points);
    let mut simplified = Vec::new();
    simplify(&points, (1. + roughness) / 2., &mut simplified);
    simplified
}
fn flush_curve(pending: &mut Vec<Point>, points: &mut Vec<Point>) {
    if pending.len() >= 4 {
        let mut out = Vec::new();
        for i in (0..pending.len() - 1).step_by(3) {
            flatten(
                [pending[i], pending[i + 1], pending[i + 2], pending[i + 3]],
                &mut out,
            );
        }
        points.extend(out);
    }
    pending.clear();
}
fn quadratic(segments: &mut Vec<Op>, current: Point, control: Point, end: Point) {
    segments.push(op(
        "bcurveTo",
        vec![
            current[0] + 2. * (control[0] - current[0]) / 3.,
            current[1] + 2. * (control[1] - current[1]) / 3.,
            end[0] + 2. * (control[0] - end[0]) / 3.,
            end[1] + 2. * (control[1] - end[1]) / 3.,
            end[0],
            end[1],
        ],
    ));
}
fn rounded_rectangle(w: f64, h: f64) -> Vec<Op> {
    let r = (w.min(h) * 0.25).min(32.);
    let mut s = vec![op("move", vec![r, 0.]), op("lineTo", vec![w - r, 0.])];
    quadratic(&mut s, [w - r, 0.], [w, 0.], [w, r]);
    s.push(op("lineTo", vec![w, h - r]));
    quadratic(&mut s, [w, h - r], [w, h], [w - r, h]);
    s.push(op("lineTo", vec![r, h]));
    quadratic(&mut s, [r, h], [0., h], [0., h - r]);
    s.push(op("lineTo", vec![0., r]));
    quadratic(&mut s, [0., r], [0., 0.], [r, 0.]);
    s
}
fn rounded_diamond(w: f64, h: f64) -> Vec<Op> {
    let tx = (w / 2.).floor() + 1.;
    let ry = (h / 2.).floor() + 1.;
    let vr = tx.abs() * 0.25;
    let hr = ry.abs() * 0.25;
    vec![
        op("move", vec![tx + vr, hr]),
        op("lineTo", vec![w - vr, ry - hr]),
        op("bcurveTo", vec![w, ry, w, ry, w - vr, ry + hr]),
        op("lineTo", vec![tx + vr, h - hr]),
        op("bcurveTo", vec![tx, h, tx, h, tx - vr, h - hr]),
        op("lineTo", vec![vr, ry + hr]),
        op("bcurveTo", vec![0., ry, 0., ry, vr, ry - hr]),
        op("lineTo", vec![tx - vr, hr]),
        op("bcurveTo", vec![tx, 0., tx, 0., tx + vr, hr]),
    ]
}
pub fn generate(e: &Request) -> Result<Vec<Drawable>, String> {
    let mut o = Rough::new(e);
    let w = e.width;
    let h = e.height;
    let mut sets = Vec::new();
    let outline;
    let shape;
    match e.tool.as_str() {
        "rectangle" | "diamond" if e.rounded => {
            shape = "path";
            let segments = if e.tool == "rectangle" {
                rounded_rectangle(w, h)
            } else {
                rounded_diamond(w, h)
            };
            outline = o.path(&segments);
            if e.filled {
                if e.fill_style == "solid" {
                    let mut fill = o.clone();
                    fill.single = true;
                    if fill.roughness != 0. {
                        fill.roughness += 0.8;
                    }
                    let ops = fill
                        .path(&segments)
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, op)| {
                            if i == 0 || op.op != "move" {
                                Some(op)
                            } else {
                                None
                            }
                        })
                        .collect();
                    o.random = fill.random;
                    sets.push(set("fillPath", ops));
                } else {
                    sets.push(o.hachure(
                        &path_points(&segments, o.roughness),
                        e.fill_style == "cross-hatch",
                    ));
                }
            }
        }
        "rectangle" | "diamond" | "triangle" => {
            shape = if e.tool == "rectangle" {
                "rectangle"
            } else {
                "polygon"
            };
            let points = match e.tool.as_str() {
                "rectangle" => vec![[0., 0.], [w, 0.], [w, h], [0., h]],
                "diamond" => {
                    let tx = (w / 2.).floor() + 1.;
                    let ry = (h / 2.).floor() + 1.;
                    vec![[tx, 0.], [w, ry], [tx, h], [0., ry]]
                }
                _ => vec![[w / 2., 0.], [w, h], [0., h]],
            };
            outline = o.linear(&points, true);
            if e.filled {
                sets.push(if e.fill_style == "solid" {
                    o.solid_polygon(&points)
                } else {
                    o.hachure(&points, e.fill_style == "cross-hatch")
                });
            }
        }
        "ellipse" => {
            shape = "ellipse";
            let psq = (PI * 2. * (((w / 2.).powi(2) + (h / 2.).powi(2)) / 2.).sqrt()).sqrt();
            let steps = (9_f64.max((9. / 200_f64.sqrt()) * psq)).ceil();
            let increment = PI * 2. / steps;
            let r = [(w / 2.).abs() + o.offset(0.), (h / 2.).abs() + o.offset(0.)];
            let center = [w / 2., h / 2.];
            let (ops, points) = o.ellipse(increment, center, r);
            outline = ops;
            if e.filled {
                sets.push(if e.fill_style == "solid" {
                    set("fillPath", o.ellipse(increment, center, r).0)
                } else {
                    o.hachure(&points, e.fill_style == "cross-hatch")
                });
            }
        }
        "line" | "arrow" => {
            shape = if e.rounded { "curve" } else { "linearPath" };
            let points = [[0., 0.], e.end];
            outline = if e.rounded {
                o.curved_line(&points)
            } else {
                o.linear(&points, false)
            };
        }
        _ => return Err(format!("unsupported tool {}", e.tool)),
    }
    sets.push(set("path", outline));
    let body = o.drawable(shape, sets);
    let mut heads = Vec::new();
    if e.tool == "arrow" {
        let ops = &body.sets.last().unwrap().ops;
        if ops.len() >= 2 {
            let data = &ops[ops.len() - 1].data;
            let prev = &ops[ops.len() - 2];
            if data.len() == 6 {
                let tip = [data[4], data[5]];
                let p0 = if prev.op == "move" {
                    [prev.data[0], prev.data[1]]
                } else {
                    [prev.data[4], prev.data[5]]
                };
                let equation = |i: usize| {
                    0.7_f64.powi(3) * tip[i]
                        + 3. * 0.3 * 0.7_f64.powi(2) * data[2 + i]
                        + 3. * 0.3_f64.powi(2) * 0.7 * data[i]
                        + p0[i] * 0.3_f64.powi(3)
                };
                let near = [equation(0), equation(1)];
                let distance = (tip[0] - near[0]).hypot(tip[1] - near[1]);
                if distance != 0. {
                    let size = 25_f64.min(e.end[0].hypot(e.end[1]) * 0.5);
                    let base = [
                        tip[0] - (tip[0] - near[0]) / distance * size,
                        tip[1] - (tip[1] - near[1]) / distance * size,
                    ];
                    for degree in [-20_f64, 20.] {
                        let angle = degree * PI / 180.;
                        let dx = base[0] - tip[0];
                        let dy = base[1] - tip[1];
                        let start = [
                            tip[0] + dx * angle.cos() - dy * angle.sin(),
                            tip[1] + dx * angle.sin() + dy * angle.cos(),
                        ];
                        let mut head = Rough::new(e);
                        head.roughness = head.roughness.min(1.);
                        head.dash = if e.stroke_style == "dotted" {
                            vec![1.5, 4. + e.stroke_width]
                        } else {
                            vec![]
                        };
                        let ops = head.double_line(start, tip, false);
                        heads.push(head.drawable("line", vec![set("path", ops)]));
                    }
                }
            }
        }
    }
    let mut drawables = vec![body];
    drawables.extend(heads);
    Ok(drawables)
}
