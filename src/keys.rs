use gdk4::{Key, ModifierType as Mod};
use serde::Deserialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Action {
    ToolSelection,
    ToolHand,
    ToolRectangle,
    ToolDiamond,
    ToolCircle,
    ToolArrow,
    ToolLine,
    ToolFreehand,
    ToolText,
    ToolEraser,
    ToolTriangle,
    LockTool,
    Undo,
    Redo,
    Delete,
    Clear,
    SelectAll,
    Duplicate,
    Copy,
    Cut,
    Paste,
    EditText,
    FinishText,
    Cancel,
    Help,
    SaveScreenshot,
    Picker,
    FillPicker,
    ColorRed,
    ColorGreen,
    ColorBlue,
    ColorYellow,
    ColorWhite,
    ColorOrange,
    BrushIncrease,
    BrushDecrease,
    FontIncrease,
    FontDecrease,
    ZoomIn,
    ZoomOut,
    ZoomReset,
    ZoomFit,
    NudgeLeft,
    NudgeRight,
    NudgeUp,
    NudgeDown,
    NudgeLeftFast,
    NudgeRightFast,
    NudgeUpFast,
    NudgeDownFast,
    BringForward,
    SendBackward,
    BringFront,
    SendBack,
    Quit,
}

impl Action {
    pub fn label(self) -> &'static str {
        use Action::*;
        match self {
            ToolSelection => "Selection",
            ToolHand => "Hand / pan",
            ToolRectangle => "Rectangle",
            ToolDiamond => "Diamond",
            ToolCircle => "Ellipse",
            ToolArrow => "Arrow",
            ToolLine => "Line",
            ToolFreehand => "Draw",
            ToolText => "Text",
            ToolEraser => "Eraser",
            ToolTriangle => "Triangle",
            LockTool => "Keep tool active",
            Undo => "Undo",
            Redo => "Redo",
            Delete => "Delete selection",
            Clear => "Clear canvas",
            SelectAll => "Select all",
            Duplicate => "Duplicate selection",
            Copy => "Copy selection",
            Cut => "Cut selection",
            Paste => "Paste selection / text",
            EditText => "Edit selected text",
            FinishText => "Finish text",
            Cancel => "Cancel / selection",
            Help => "Keyboard shortcuts",
            SaveScreenshot => "Save screenshot",
            Picker => "Stroke color",
            FillPicker => "Background color",
            ColorRed => "Red",
            ColorGreen => "Green",
            ColorBlue => "Blue",
            ColorYellow => "Yellow",
            ColorWhite => "White",
            ColorOrange => "Orange",
            BrushIncrease => "Stroke width +",
            BrushDecrease => "Stroke width −",
            FontIncrease => "Font size +",
            FontDecrease => "Font size −",
            ZoomIn => "Zoom in",
            ZoomOut => "Zoom out",
            ZoomReset => "Reset zoom",
            ZoomFit => "Zoom to fit",
            NudgeLeft => "Move left",
            NudgeRight => "Move right",
            NudgeUp => "Move up",
            NudgeDown => "Move down",
            NudgeLeftFast => "Move left 10px",
            NudgeRightFast => "Move right 10px",
            NudgeUpFast => "Move up 10px",
            NudgeDownFast => "Move down 10px",
            BringForward => "Bring forward",
            SendBackward => "Send backward",
            BringFront => "Bring to front",
            SendBack => "Send to back",
            Quit => "Quit",
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum KeySpec {
    One(String),
    Many(Vec<String>),
}
impl KeySpec {
    pub fn names(&self) -> Vec<&str> {
        match self {
            Self::One(s) => vec![s],
            Self::Many(v) => v.iter().map(String::as_str).collect(),
        }
    }
    pub fn display(&self) -> String {
        self.names().join(" / ")
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Binding {
    key: Key,
    mods: Mod,
}
impl Binding {
    pub fn parse(s: &str) -> Result<Self, String> {
        let mut mods = Mod::empty();
        let mut parts: Vec<_> = s.split('+').map(str::trim).collect();
        let name = parts
            .pop()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| format!("invalid binding: {s:?}; use 'plus' for +"))?;
        for part in parts {
            mods |= match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => Mod::CONTROL_MASK,
                "shift" => Mod::SHIFT_MASK,
                "alt" => Mod::ALT_MASK,
                "super" => Mod::SUPER_MASK,
                _ => return Err(format!("unknown modifier {part:?} in {s:?}")),
            };
        }
        let alias = match name.to_ascii_lowercase().as_str() {
            "esc" | "escape" => "Escape",
            "enter" | "return" => "Return",
            "delete" | "del" => "Delete",
            "backspace" => "BackSpace",
            "left" => "Left",
            "right" => "Right",
            "up" => "Up",
            "down" => "Down",
            "tab" => "Tab",
            "?" => "question",
            "<" => "less",
            ">" => "greater",
            "{" => "braceleft",
            "}" => "braceright",
            ";" => "semicolon",
            "'" => "apostrophe",
            "`" => "grave",
            "\\" => "backslash",
            "[" => "bracketleft",
            "]" => "bracketright",
            "/" => "slash",
            "." => "period",
            "," => "comma",
            "=" => "equal",
            "-" => "minus",
            _ => name,
        };
        if matches!(
            alias,
            "question" | "greater" | "less" | "plus" | "braceleft" | "braceright"
        ) {
            mods |= Mod::SHIFT_MASK;
        }
        let raw = Key::from_name(alias).ok_or_else(|| format!("unknown key {name:?} in {s:?}"))?;
        let key = Self::normalize(raw);
        Ok(Self { key, mods })
    }
    pub fn matches(&self, key: Key, mods: Mod) -> bool {
        let mask = Mod::CONTROL_MASK | Mod::SHIFT_MASK | Mod::ALT_MASK | Mod::SUPER_MASK;
        // GDK reports shifted key symbols (e.g. Ctrl+Shift+] becomes Ctrl+}).
        self.key == Self::normalize(key) && self.mods == (mods & mask)
    }
    fn normalize(key: Key) -> Key {
        match key {
            Key::braceleft => Key::bracketleft,
            Key::braceright => Key::bracketright,
            Key::KP_Enter => Key::Return,
            Key::KP_Delete => Key::Delete,
            _ => key.to_lower(),
        }
    }
}

#[derive(Clone)]
pub struct Bindings(Vec<(Action, Binding)>);
impl Bindings {
    pub fn compile(keys: &BTreeMap<Action, KeySpec>) -> Result<Self, String> {
        let mut bindings = Vec::new();
        for (&action, spec) in keys {
            for name in spec.names() {
                let b = Binding::parse(name)?;
                if let Some((other, _)) = bindings.iter().find(|(a, old)| *a != action && *old == b)
                {
                    // Escape is intentionally contextual: finish text versus cancel.
                    if !matches!(
                        (action, *other),
                        (Action::Cancel, Action::FinishText) | (Action::FinishText, Action::Cancel)
                    ) {
                        return Err(format!(
                            "binding {name:?} conflicts: {other:?} and {action:?}"
                        ));
                    }
                }
                bindings.push((action, b));
            }
        }
        Ok(Self(bindings))
    }
    pub fn action(&self, key: Key, mods: Mod, editing: bool) -> Option<Action> {
        self.0
            .iter()
            .find(|(a, b)| {
                if editing && !matches!(a, Action::FinishText | Action::Paste) {
                    return false;
                }
                if !editing && *a == Action::FinishText {
                    return false;
                }
                b.matches(key, mods)
            })
            .map(|(a, _)| *a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shifted_letters_and_lock_modifiers() {
        assert!(Binding::parse("Ctrl+Shift+z")
            .unwrap()
            .matches(Key::Z, Mod::CONTROL_MASK | Mod::SHIFT_MASK | Mod::LOCK_MASK));
        assert!(!Binding::parse("z")
            .unwrap()
            .matches(Key::Z, Mod::SHIFT_MASK));
        assert!(Binding::parse("Ctrl+Shift+]")
            .unwrap()
            .matches(Key::braceright, Mod::CONTROL_MASK | Mod::SHIFT_MASK));
        assert!(Binding::parse("?")
            .unwrap()
            .matches(Key::question, Mod::SHIFT_MASK));
    }
    #[test]
    fn rejects_typographical_errors() {
        assert!(Binding::parse("Crtl+z").is_err());
        assert!(Binding::parse("Ctrl+").is_err());
        assert!(Binding::parse("not-a-key").is_err());
    }
}
