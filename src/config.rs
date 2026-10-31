use crate::canvas::{parse_color, FillStyle, StrokeStyle, StrokeVariability, Theme};
use crate::keys::{Action, Bindings, KeySpec};
use serde::Deserialize;
use std::{collections::BTreeMap, fs, path::PathBuf};

pub const DEFAULT_CONFIG: &str = include_str!("../config/default.toml");

pub fn config_path() -> PathBuf {
    if let Some(path) = std::env::var_os("RSDRAW_CONFIG") {
        return path.into();
    }
    let base = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(std::env::var_os("HOME").unwrap_or_else(|| ".".into())).join(".config")
        });
    base.join("rsdraw/config.toml")
}

#[derive(Deserialize, Clone)]
#[serde(default, deny_unknown_fields)]
pub struct DrawingConfig {
    pub default_color: String,
    pub default_width: f64,
    pub font_family: String,
    pub font_size: f64,
    pub roughness: f64,
    pub fill_color: String,
    pub fill_style: FillStyle,
    pub stroke_style: StrokeStyle,
    pub tool_lock: bool,
    pub theme: Theme,
    pub rounded: bool,
    pub stroke_variability: StrokeVariability,
}
impl Default for DrawingConfig {
    fn default() -> Self {
        Self {
            default_color: "#1e1e1e".into(),
            default_width: 3.0,
            font_family: "Excalifont".into(),
            font_size: 24.0,
            roughness: 1.0,
            fill_color: "transparent".into(),
            fill_style: FillStyle::Solid,
            stroke_style: StrokeStyle::Solid,
            tool_lock: true,
            theme: Theme::Dark,
            rounded: true,
            stroke_variability: StrokeVariability::Constant,
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(deny_unknown_fields)]
pub struct Config {
    #[serde(default)]
    pub drawing: DrawingConfig,
    #[serde(default)]
    pub keys: BTreeMap<Action, KeySpec>,
}
impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).expect("bundled config must be valid")
    }
}
impl Config {
    pub fn parse(content: &str) -> Result<Self, String> {
        let mut cfg: Self = toml::from_str(content).map_err(|e| format!("{e}"))?;
        // Explicit user bindings take precedence over inherited defaults. This
        // also lets pre-Excalidraw configs keep their numeric color shortcuts.
        let explicit = cfg.keys.clone();
        Bindings::compile(&explicit)?;
        let explicit_bindings: Vec<_> = explicit
            .iter()
            .flat_map(|(&action, spec)| spec.names().into_iter().map(move |name| (action, name)))
            .map(|(action, name)| {
                crate::keys::Binding::parse(name).map(|binding| (action, binding))
            })
            .collect::<Result<_, _>>()?;
        let mut keys = Self::default().keys;
        for (action, spec) in &mut keys {
            if !explicit.contains_key(action) {
                *spec = KeySpec::Many(
                    spec.names()
                        .into_iter()
                        .filter(|name| {
                            crate::keys::Binding::parse(name).is_ok_and(|b| {
                                !explicit_bindings.iter().any(|(other, binding)| {
                                    *binding == b
                                        && !matches!(
                                            (*action, *other),
                                            (Action::FinishText, Action::Cancel)
                                                | (Action::Cancel, Action::FinishText)
                                        )
                                })
                            })
                        })
                        .map(str::to_owned)
                        .collect(),
                );
            }
        }
        keys.append(&mut cfg.keys);
        cfg.keys = keys;
        let d = &cfg.drawing;
        for (name, value, min, max) in [
            ("default_width", d.default_width, 0.5, 50.0),
            ("font_size", d.font_size, 6.0, 256.0),
            ("roughness", d.roughness, 0.0, 3.0),
        ] {
            if !value.is_finite() || !(min..=max).contains(&value) {
                return Err(format!("drawing.{name} must be between {min} and {max}"));
            }
        }
        parse_color(&d.default_color).ok_or("invalid drawing.default_color")?;
        parse_color(&d.fill_color).ok_or("invalid drawing.fill_color")?;
        if d.font_family.trim().is_empty() {
            return Err("font_family cannot be empty".into());
        }
        Bindings::compile(&cfg.keys)?;
        Ok(cfg)
    }
    pub fn help_lines(&self) -> Vec<(String, String)> {
        self.keys
            .iter()
            .filter(|(_, s)| !s.names().is_empty())
            .map(|(&a, s)| (s.display(), a.label().into()))
            .collect()
    }
}

pub fn load(path: &std::path::Path) -> Result<Config, String> {
    if !path.exists() {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        fs::write(path, DEFAULT_CONFIG).map_err(|e| format!("{}: {e}", path.display()))?;
        eprintln!("created config at {}", path.display());
    }
    Config::parse(&fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?)
}

pub fn print_help(cfg: &Config, path: &std::path::Path) {
    println!("rsdraw — hand-drawn screen annotations for Wayland\n\nUsage: rsdraw [--config PATH] [COMMAND]\n\nKeybindings ({}):", path.display());
    for (key, label) in cfg.help_lines() {
        println!("  {key:<35} {label}");
    }
    println!("\nMouse: Shift constrains shapes / adds selection; Alt draws from center / duplicates.\nSpace+drag or middle drag pans; Ctrl+wheel zooms. Double-click edits text.\nText: Enter inserts a new line; Escape or Ctrl+Enter finishes.\n\nCommands:\n  toggle, clear, undo, redo, quit, picker\n  tool <selection|hand|rectangle|diamond|ellipse|triangle|arrow|line|freehand|text|eraser>\n  color <name|#hex>, fill <name|#hex|transparent>, width <0.5..50>\n  font <family>, font-size <6..256>, roughness <0..3>\n  fill-style <hachure|cross_hatch|solid>, stroke-style <solid|dashed|dotted>\n  theme <light|dark>, roundness <round|sharp>\n  stroke-variability <constant|variable>\n  help, config-path, check-config\n\nSee README.md and docs/CONTROLS.md for the full interaction guide.");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_compile_and_overrides_merge() {
        Config::parse(DEFAULT_CONFIG).unwrap();
        let c = Config::parse("[keys]\ntool_freehand = ['b', '9']\nquit = []").unwrap();
        assert_eq!(c.keys[&Action::ToolFreehand].display(), "b / 9");
        assert!(c.keys[&Action::Quit].names().is_empty());
        assert_eq!(c.keys[&Action::ToolText].display(), "t / 8");
        assert_eq!(c.keys[&Action::SaveScreenshot].display(), "Ctrl+s");
        assert_eq!(
            Config::parse("[drawing]\ntheme='light'")
                .unwrap()
                .drawing
                .theme,
            Theme::Light
        );
        assert_eq!(
            Config::parse("[drawing]\ntheme='dark'")
                .unwrap()
                .drawing
                .theme,
            Theme::Dark
        );
    }
    #[test]
    fn explicit_keys_override_inherited_shortcuts() {
        let c = Config::parse("[keys]\ncolor_red='1'\nquit='Ctrl+x'").unwrap();
        assert_eq!(c.keys[&Action::ToolSelection].display(), "v");
        assert!(c.keys[&Action::Cut].names().is_empty());
        assert_eq!(c.keys[&Action::Quit].display(), "Ctrl+x");
    }
    #[test]
    fn invalid_config_is_reported() {
        for s in [
            "[drawing]\nroughness=nan",
            "[drawing]\ntheme='invalid'",
            "[drawing]\ndefault_color='oops'",
            "[keys]\nundo='r'\ntool_rectangle='r'",
            "[keys]\nundoo='z'",
        ] {
            assert!(Config::parse(s).is_err(), "accepted {s}");
        }
    }
}
