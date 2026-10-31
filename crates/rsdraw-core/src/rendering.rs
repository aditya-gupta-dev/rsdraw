//! Native, deterministic Excalidraw-compatible path generation.
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Request {
    pub tool: String,
    pub width: f64,
    pub height: f64,
    pub end: [f64; 2],
    pub points: Vec<(f64, f64)>,
    pub seed: u32,
    pub stroke_width: f64,
    pub roughness: f64,
    pub stroke_style: String,
    pub fill_style: String,
    pub filled: bool,
    pub rounded: bool,
    pub variability: String,
}

pub fn generate(request: &Value) -> Result<RenderData, String> {
    let request: Request = serde_json::from_value(request.clone()).map_err(|e| e.to_string())?;
    generate_request(&request)
}

pub fn generate_request(request: &Request) -> Result<RenderData, String> {
    if ![
        request.width,
        request.height,
        request.end[0],
        request.end[1],
        request.stroke_width,
        request.roughness,
    ]
    .iter()
    .all(|n| n.is_finite())
        || request.width < 0.
        || request.height < 0.
        || request.stroke_width <= 0.
        || request.roughness < 0.
    {
        return Err("invalid drawing dimensions or style".into());
    }
    if request.tool == "freehand" {
        return freehand(
            &request.points,
            request.stroke_width,
            request.variability == "variable",
            request.seed as u64,
        );
    }
    Ok(RenderData {
        drawables: crate::rough::generate(request)?,
        outline: vec![],
    })
}

pub fn freehand(
    points: &[(f64, f64)],
    width: f64,
    variable: bool,
    _seed: u64,
) -> Result<RenderData, String> {
    if !width.is_finite()
        || width <= 0.
        || points.iter().any(|p| !p.0.is_finite() || !p.1.is_finite())
    {
        return Err("invalid freehand input".into());
    }
    let outline = if variable {
        crate::variable_freehand::outline(points, width)
    } else {
        crate::freehand::outline(points, width)
    };
    if outline
        .iter()
        .flatten()
        .any(|coordinate| !coordinate.is_finite())
    {
        return Err("non-finite freehand outline".into());
    }
    Ok(freehand_path(outline))
}
fn freehand_path(outline: Vec<[f64; 2]>) -> RenderData {
    use crate::freehand::trim_coordinate as trim;
    if outline.is_empty() {
        return RenderData {
            drawables: vec![],
            outline,
        };
    }
    let first = [trim(outline[0][0]), trim(outline[0][1])];
    let mut from = first;
    let mut ops = Vec::with_capacity(outline.len() + 2);
    ops.push(Op {
        op: "move".into(),
        data: first.to_vec(),
    });
    for (i, control) in outline.iter().enumerate() {
        let next = outline[(i + 1) % outline.len()];
        let end = [
            trim((control[0] + next[0]) / 2.),
            trim((control[1] + next[1]) / 2.),
        ];
        let control = [trim(control[0]), trim(control[1])];
        ops.push(Op {
            op: "bcurveTo".into(),
            data: vec![
                from[0] + 2. * (control[0] - from[0]) / 3.,
                from[1] + 2. * (control[1] - from[1]) / 3.,
                end[0] + 2. * (control[0] - end[0]) / 3.,
                end[1] + 2. * (control[1] - end[1]) / 3.,
                end[0],
                end[1],
            ],
        });
        from = end;
    }
    ops.push(Op {
        op: "lineTo".into(),
        data: first.to_vec(),
    });
    RenderData {
        outline,
        drawables: vec![Drawable {
            shape: "path".into(),
            sets: vec![OpSet {
                r#type: "fillPath".into(),
                ops,
            }],
            options: Options {
                stroke: "none".into(),
                stroke_width: 1.,
                fill_weight: -1.,
                stroke_line_dash: vec![],
            },
        }],
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct RenderData {
    pub drawables: Vec<Drawable>,
    #[serde(default)]
    pub outline: Vec<[f64; 2]>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Drawable {
    pub shape: String,
    pub sets: Vec<OpSet>,
    pub options: Options,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpSet {
    pub r#type: String,
    pub ops: Vec<Op>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Op {
    pub op: String,
    pub data: Vec<f64>,
}
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Options {
    pub stroke: String,
    pub stroke_width: f64,
    pub fill_weight: f64,
    #[serde(default)]
    pub stroke_line_dash: Vec<f64>,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn close(actual: &Value, expected: &Value, path: &str) {
        match (actual, expected) {
            (Value::Number(a), Value::Number(b)) => {
                let a = a.as_f64().unwrap();
                let b = b.as_f64().unwrap();
                assert!((a - b).abs() <= 1e-9, "{path}: {a} != {b}");
            }
            (Value::Array(a), Value::Array(b)) => {
                assert_eq!(a.len(), b.len(), "{path} length");
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    close(a, b, &format!("{path}[{i}]"));
                }
            }
            (Value::Object(a), Value::Object(b)) => {
                for (key, value) in b {
                    close(
                        a.get(key).unwrap_or(&Value::Null),
                        value,
                        &format!("{path}.{key}"),
                    );
                }
            }
            _ => assert_eq!(actual, expected, "{path}"),
        }
    }
    fn fill_pixels(data: &RenderData) -> Vec<u8> {
        let mut surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 512, 384).unwrap();
        {
            let cr = cairo::Context::new(&surface).unwrap();
            cr.translate(100., 150.);
            cr.set_source_rgba(1., 0., 0., 1.);
            cr.set_fill_rule(cairo::FillRule::EvenOdd);
            for d in &data.drawables {
                for set in &d.sets {
                    cr.new_path();
                    for op in &set.ops {
                        let p = &op.data;
                        match op.op.as_str() {
                            "move" => cr.move_to(p[0], p[1]),
                            "lineTo" => cr.line_to(p[0], p[1]),
                            "bcurveTo" => cr.curve_to(p[0], p[1], p[2], p[3], p[4], p[5]),
                            _ => unreachable!(),
                        }
                    }
                    cr.fill().unwrap();
                }
            }
        }
        let pixels = surface.data().unwrap().to_vec();
        pixels
    }
    #[test]
    fn native_freehand_matches_reference_outline_and_pixels() {
        let oracle: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/excalidraw.json")).unwrap();
        for fixture in oracle["fixtures"].as_array().unwrap() {
            let r = &fixture["request"];
            if r["tool"] != "freehand" {
                continue;
            }
            let points: Vec<(f64, f64)> = serde_json::from_value(r["points"].clone()).unwrap();
            let actual = freehand(
                &points,
                r["strokeWidth"].as_f64().unwrap(),
                r["variability"] == "variable",
                r["seed"].as_u64().unwrap(),
            )
            .unwrap();
            let expected: RenderData = serde_json::from_value(fixture["expected"].clone()).unwrap();
            close(
                &serde_json::json!(actual.outline),
                &serde_json::json!(expected.outline),
                "native outline",
            );
            assert!(
                fill_pixels(&actual) == fill_pixels(&expected),
                "{} points, {}",
                points.len(),
                r["variability"]
            );
        }
    }
    #[test]
    fn invalid_outline_cannot_poison_the_cairo_context() {
        // Distinct raw samples can smooth to the same point in the reference.
        assert!(freehand(&[(0., 0.), (20., 0.), (10., 0.)], 2., false, 1).is_err());
    }
    #[test]
    fn native_precision_matches_reference_decimal_boundaries() {
        for (input, expected) in [
            (1.15, 1.15),
            (-1.15, -1.15),
            (1.005, 1.0),
            (-1.005, -1.0),
            (6.123233995736766e-17, 6.12),
            (1e-7, 1e-7),
            (1.234e-7, 1.23),
            (9999.9999, 9999.99),
        ] {
            assert_eq!(crate::freehand::trim_coordinate(input), expected);
        }
    }
    #[test]
    fn paths_match_the_actual_excalidraw_reference() {
        let oracle: Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/excalidraw.json")).unwrap();
        for (i, fixture) in oracle["fixtures"].as_array().unwrap().iter().enumerate() {
            let request = &fixture["request"];
            // Freehand's direct quadratic fill has its own pixel/outline comparison.
            if request["tool"] == "freehand" {
                continue;
            }
            let actual = generate(request).unwrap();
            let expected: RenderData = serde_json::from_value(fixture["expected"].clone()).unwrap();
            close(
                &serde_json::to_value(actual).unwrap(),
                &serde_json::to_value(expected).unwrap(),
                &format!("case {i} {}", request["tool"]),
            );
        }
    }
}
