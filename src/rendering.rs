//! Executes the pinned upstream JS generators. Cairo receives their original
//! path operations; there is no independent sketch algorithm to drift from them.
use rquickjs::{Context, Function, Runtime};
use serde::Deserialize;
use serde_json::Value;

struct Engine {
    context: Context,
    _runtime: Runtime,
}
impl Engine {
    fn new() -> Result<Self, String> {
        let runtime = Runtime::new().map_err(|e| e.to_string())?;
        runtime.set_memory_limit(64 * 1024 * 1024);
        let context = Context::full(&runtime).map_err(|e| e.to_string())?;
        context.with(|ctx| -> Result<(), String> {
            for source in [
                // RoughJS's SVG parser uses legacy RegExp.$1 capture statics.
                // QuickJS omits them; preserve standard browser match semantics.
                "(()=>{let captures=[];const exec=RegExp.prototype.exec;RegExp.prototype.exec=function(s){const result=exec.call(this,s);if(result)captures=result;return result;};for(let i=1;i<=9;i++)Object.defineProperty(RegExp,'$'+i,{get:()=>captures[i]||''});})();",
                concat!(
                    "globalThis.Rough=(()=>{const module={exports:{}};",
                    include_str!("../assets/vendor/rough-4.6.4.cjs"),
                    ";return module.exports;})();"
                ),
                concat!(
                    "globalThis.PerfectFreehand=(()=>{const exports={};",
                    include_str!("../assets/vendor/perfect-freehand-1.2.0.cjs"),
                    ";return exports;})();"
                ),
                include_str!("../assets/vendor/excalidraw-laser-pointer.js"),
                include_str!("../assets/rendering/excalidraw.js"),
            ] {
                ctx.eval::<(), _>(source)
                    .map_err(|e| format!("renderer initialization: {e}: {:?}", ctx.catch()))?;
            }
            Ok(())
        })?;
        Ok(Self {
            context,
            _runtime: runtime,
        })
    }
    fn generate(&self, json: &str) -> Result<RenderData, String> {
        self.context.with(|ctx| {
            let f: Function = ctx
                .globals()
                .get("rsdrawGenerate")
                .map_err(|e| e.to_string())?;
            let output: String = f
                .call((json,))
                .map_err(|e| format!("upstream renderer: {e}: {:?}", ctx.catch()))?;
            serde_json::from_str(&output).map_err(|e| e.to_string())
        })
    }
}
thread_local! {static ENGINE:Result<Engine,String>=Engine::new();}
pub fn initialize() -> Result<(), String> {
    ENGINE.with(|e| e.as_ref().map(|_| ()).map_err(Clone::clone))
}
pub fn generate(request: &Value) -> Result<RenderData, String> {
    let json = serde_json::to_string(request).map_err(|e| e.to_string())?;
    ENGINE.with(|e| e.as_ref().map_err(Clone::clone)?.generate(&json))
}

#[derive(Debug, Clone, Deserialize)]
pub struct RenderData {
    pub drawables: Vec<Drawable>,
    #[serde(default)]
    pub outline: Vec<[f64; 2]>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Drawable {
    pub shape: String,
    pub sets: Vec<OpSet>,
    pub options: Options,
}
#[derive(Debug, Clone, Deserialize)]
pub struct OpSet {
    pub r#type: String,
    pub ops: Vec<Op>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct Op {
    pub op: String,
    pub data: Vec<f64>,
}
#[derive(Debug, Clone, Deserialize)]
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
    #[test]
    fn paths_match_the_actual_excalidraw_reference() {
        let oracle: Value =
            serde_json::from_str(include_str!("../tests/fixtures/excalidraw.json")).unwrap();
        for (i, fixture) in oracle["fixtures"].as_array().unwrap().iter().enumerate() {
            let request = &fixture["request"];
            // Compare the complete result in the JS engine to catch every op and option.
            let actual: Value = ENGINE.with(|engine| {
                engine.as_ref().unwrap().context.with(|ctx| {
                    let f: Function = ctx.globals().get("rsdrawGenerate").unwrap();
                    let result: String = f.call((request.to_string(),)).unwrap();
                    serde_json::from_str(&result).unwrap()
                })
            });
            close(
                &actual,
                &fixture["expected"],
                &format!("case {i} {}", request["tool"]),
            );
            generate(request).unwrap(); // Also validate the native path decoder.
        }
    }
}
