use serde::Deserialize;
use std::fs;
use std::path::PathBuf;

pub fn config_dir() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME").unwrap_or_else(|_| {
        let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
        format!("{}/.config", home)
    });
    PathBuf::from(base).join("rsdraw")
}

pub fn config_path() -> PathBuf {
    config_dir().join("config.toml")
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct Config {
    pub drawing: DrawingConfig,
    pub keys: KeysConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            drawing: DrawingConfig::default(),
            keys: KeysConfig::default(),
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct DrawingConfig {
    pub default_color: String,
    pub default_width: f64,
    pub font_family: String,
}

impl Default for DrawingConfig {
    fn default() -> Self {
        Self {
            default_color: "red".to_string(),
            default_width: 3.0,
            font_family: "sans-serif".to_string(),
        }
    }
}

#[derive(Deserialize, Clone)]
#[serde(default)]
pub struct KeysConfig {
    pub quit: String,
    pub undo: String,
    pub help: String,

    pub color_red: String,
    pub color_green: String,
    pub color_blue: String,
    pub color_yellow: String,
    pub color_white: String,
    pub color_orange: String,

    pub brush_increase: String,
    pub brush_decrease: String,

    pub tool_freehand: String,
    pub tool_rectangle: String,
    pub tool_circle: String,
    pub tool_triangle: String,
    pub tool_arrow: String,
    pub tool_text: String,
    pub picker: String,
}

impl Default for KeysConfig {
    fn default() -> Self {
        Self {
            quit: "Ctrl+x".into(),
            undo: "Ctrl+z".into(),
            help: "F1".into(),
            picker: "p".into(),
            color_red: "1".into(),
            color_green: "2".into(),
            color_blue: "3".into(),
            color_yellow: "4".into(),
            color_white: "5".into(),
            color_orange: "6".into(),
            brush_increase: "equal".into(),
            brush_decrease: "minus".into(),
            tool_freehand: "f".into(),
            tool_rectangle: "r".into(),
            tool_circle: "c".into(),
            tool_triangle: "t".into(),
            tool_arrow: "a".into(),
            tool_text: "x".into(),
        }
    }
}
const DEFAULT_CONFIG: &str = r#"# rsdraw configuration (Linux)
# Keybinding format: [Modifier+]Key
# Modifiers: Ctrl, Shift, Alt, Super
# Keys: a-z, 0-9, F1-F12, Escape, space, equal, minus, plus, period, comma ...

[drawing]
default_color = "red"
default_width = 3.0
font_family   = "sans-serif" # e.g. "sans-serif", "monospace", "JetBrains Mono", "Fira Code"

[keys]
# Global
quit           = "Ctrl+x"
undo           = "Ctrl+z"
help           = "F1"
picker         = "p"

# Colors (names or hex: #RGB, #RGBA, #RRGGBB, #RRGGBBAA)
color_red      = "1"
color_green    = "2"
color_blue     = "3"
color_yellow   = "4"
color_white    = "5"
color_orange   = "6"

# Brush size
brush_increase = "equal"
brush_decrease = "minus"

# Tools
tool_freehand  = "f"
tool_rectangle = "r"
tool_circle    = "c"
tool_triangle  = "t"
tool_arrow     = "a"
tool_text      = "x"
"#;

pub fn load() -> Config {
    let path = config_path();

    if !path.exists() {
        let _ = fs::create_dir_all(config_dir());
        let _ = fs::write(&path, DEFAULT_CONFIG);
        eprintln!("created default config at {}", path.display());
    }

    match fs::read_to_string(&path) {
        Ok(content) => match toml::from_str(&content) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("config parse error: {}. using defaults.", e);
                Config::default()
            }
        },
        Err(e) => {
            eprintln!("config read error: {}. using defaults.", e);
            Config::default()
        }
    }
}

pub fn print_help(config: &Config) {
    let k = &config.keys;
    println!("rsdraw v0.1.0 — Screen annotation tool for Linux (Wayland / Hyprland)\n");
    println!("Keybindings ({})\n", config_path().display());

    let sections: &[(&str, &[(&str, &str)])] = &[
        (
            "General",
            &[
                (&k.quit, "Quit rsdraw"),
                (&k.undo, "Undo last element"),
                (&k.help, "Toggle help panel"),
                (&k.picker, "Toggle color palette / picker"),
            ],
        ),
        (
            "Tools",
            &[
                (&k.tool_freehand, "Freehand pen"),
                (&k.tool_rectangle, "Rectangle"),
                (&k.tool_circle, "Circle / ellipse"),
                (&k.tool_triangle, "Triangle"),
                (&k.tool_arrow, "Arrow"),
                (&k.tool_text, "Text typing"),
            ],
        ),
        (
            "Colors",
            &[
                (&k.color_red, "Red"),
                (&k.color_green, "Green"),
                (&k.color_blue, "Blue"),
                (&k.color_yellow, "Yellow"),
                (&k.color_white, "White"),
                (&k.color_orange, "Orange"),
            ],
        ),
        (
            "Brush",
            &[
                (&k.brush_increase, "Increase width / text size"),
                (&k.brush_decrease, "Decrease width / text size"),
            ],
        ),
    ];

    for (title, entries) in sections {
        println!("  {}:", title);
        for (key, desc) in *entries {
            println!("    {:<16} {}", key, desc);
        }
        println!();
    }

    println!("IPC commands:\n");
    println!("  rsdraw toggle         Toggle draw / passthrough");
    println!("  rsdraw clear          Clear canvas");
    println!("  rsdraw undo           Undo last element");
    println!("  rsdraw quit           Quit");
    println!("  rsdraw picker         Toggle color palette / picker");
    println!("  rsdraw tool <name>    Switch tool (freehand/rect/circle/tri/arrow/text)");
    println!("  rsdraw font <name>    Set font family (e.g. monospace, 'JetBrains Mono')");
    println!("  rsdraw color <name>   Set color (name, #RGB, #RGBA, #RRGGBB, #RRGGBBAA)");
    println!("  rsdraw width <n>      Set brush width (1-50)");
    println!("  rsdraw help           Show this help");
    println!("\nConfig: {}", config_path().display());
}

pub fn help_lines(config: &Config) -> Vec<(String, String)> {
    let k = &config.keys;
    vec![
        (k.quit.clone(), "Quit".into()),
        (k.undo.clone(), "Undo".into()),
        (k.help.clone(), "Toggle help".into()),
        (k.picker.clone(), "Color picker".into()),
        (k.tool_freehand.clone(), "Freehand".into()),
        (k.tool_rectangle.clone(), "Rectangle".into()),
        (k.tool_circle.clone(), "Circle".into()),
        (k.tool_triangle.clone(), "Triangle".into()),
        (k.tool_arrow.clone(), "Arrow".into()),
        (k.tool_text.clone(), "Text".into()),
        (k.color_red.clone(), "Red".into()),
        (k.color_green.clone(), "Green".into()),
        (k.color_blue.clone(), "Blue".into()),
        (k.color_yellow.clone(), "Yellow".into()),
        (k.color_white.clone(), "White".into()),
        (k.color_orange.clone(), "Orange".into()),
        (k.brush_increase.clone(), "Size +".into()),
        (k.brush_decrease.clone(), "Size −".into()),
    ]
}
