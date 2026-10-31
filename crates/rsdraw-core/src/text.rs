//! Text metrics and baseline placement from Excalidraw's font-metadata.ts and
//! textMeasurements.ts. Pango supplies shaping; Cairo supplies rasterization.
use crate::canvas::Style;

/// Excalidraw normalizes line endings and expands each tab to eight spaces.
pub fn normalize(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .replace('\t', "        ")
}

#[derive(Clone, Copy, Debug)]
pub struct FontMetrics {
    pub units_per_em: f64,
    pub ascender: f64,
    pub descender: f64,
    pub line_height: f64,
}
pub fn metrics(family: &str) -> FontMetrics {
    let (units_per_em, ascender, descender, line_height) = match family {
        "Nunito" => (1000., 1011., -353., 1.25),
        "Lilita One" => (1000., 923., -220., 1.15),
        "Comic Shanns" => (1000., 750., -250., 1.25),
        "Helvetica" => (2048., 1577., -471., 1.15),
        "Cascadia" | "Cascadia Code" => (2048., 1900., -480., 1.2),
        "Liberation Sans" => (2048., 1854., -434., 1.15),
        "Assistant" => (2048., 1021., -287., 1.25),
        "Xiaolai" => (1000., 880., -144., 1.25),
        _ => (1000., 886., -374., 1.25),
    };
    FontMetrics {
        units_per_em,
        ascender,
        descender,
        line_height,
    }
}
impl FontMetrics {
    pub fn baseline(self, size: f64) -> f64 {
        let em = size / self.units_per_em;
        let gap = (size * self.line_height - em * self.ascender + em * self.descender) / 2.;
        em * self.ascender + gap
    }
}

pub fn layout(cr: &cairo::Context, text: &str, style: &Style) -> pango::Layout {
    let layout = pangocairo::functions::create_layout(cr);
    // Canvas measureText uses fractional advances; Pango's default rounds them.
    layout.context().set_round_glyph_positions(false);
    let mut options = cairo::FontOptions::new().expect("font options");
    options.set_hint_metrics(cairo::HintMetrics::Off);
    pangocairo::functions::context_set_font_options(&layout.context(), Some(&options));
    let mut font = pango::FontDescription::new();
    font.set_family(&style.font);
    font.set_absolute_size(style.font_size * pango::SCALE as f64);
    font.set_weight(pango::Weight::Normal);
    layout.set_font_description(Some(&font));
    layout.set_text(&normalize(text));
    let attrs = pango::AttrList::new();
    attrs.insert(pango::AttrInt::new_line_height_absolute(
        (style.font_size * metrics(&style.font).line_height * pango::SCALE as f64).round() as i32,
    ));
    layout.set_attributes(Some(&attrs));
    layout
}

pub fn measure(cr: &cairo::Context, text: &str, style: &Style) -> (f64, f64) {
    let normalized = normalize(text);
    let mut width = 0_f64;
    let mut count = 0;
    for line in normalized.split('\n') {
        // Empty lines are measured as a space in measureText, including trailing
        // newlines. The line box height is independent of fallback font metrics.
        let layout = layout(cr, if line.is_empty() { " " } else { line }, style);
        width = width.max(layout.size().0 as f64 / pango::SCALE as f64);
        count += 1;
    }
    (
        width,
        count as f64 * style.font_size * metrics(&style.font).line_height,
    )
}

pub fn draw(cr: &cairo::Context, pos: (f64, f64), text: &str, style: &Style) {
    let metrics = metrics(&style.font);
    for (index, line) in normalize(text).split('\n').enumerate() {
        let layout = layout(cr, line, style);
        // Excalidraw draws each line with an alphabetic baseline. Rendering a
        // multiline Pango layout lets fallback fonts move subsequent baselines.
        let baseline = metrics.baseline(style.font_size)
            + index as f64 * style.font_size * metrics.line_height;
        cr.move_to(
            pos.0,
            pos.1 + baseline - layout.baseline() as f64 / pango::SCALE as f64,
        );
        pangocairo::functions::show_layout(cr, &layout);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_matches_browser_reference_metrics() {
        crate::fonts::register().unwrap();
        let oracle: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/fixtures/text.json")).unwrap();
        let surface = cairo::ImageSurface::create(cairo::Format::ARgb32, 1, 1).unwrap();
        let cr = cairo::Context::new(&surface).unwrap();
        let mut style = crate::canvas::tests::style();
        style.font = "Excalifont".into();
        for fixture in oracle["fixtures"].as_array().unwrap() {
            let text = fixture["text"].as_str().unwrap();
            style.font_size = fixture["fontSize"].as_f64().unwrap();
            let (width, height) = measure(&cr, text, &style);
            let expected_width = fixture["width"].as_f64().unwrap();
            // Pango measures in 1/1024 px, Chrome in fractional font advances.
            assert!(
                (width - expected_width).abs() < 0.08,
                "{text:?} at {}px: width {width} != {expected_width}",
                style.font_size
            );
            assert!((height - fixture["height"].as_f64().unwrap()).abs() < 1e-9);
            assert!(
                (metrics(&style.font).baseline(style.font_size)
                    - fixture["baseline"].as_f64().unwrap())
                .abs()
                    < 1e-9
            );
        }
    }
    #[test]
    fn reference_normalization_and_baselines() {
        assert_eq!(normalize("a\r\nb\rc\td\n"), "a\nb\nc        d\n");
        assert!((metrics("Excalifont").baseline(20.) - 17.62).abs() < 1e-12);
        assert!((metrics("Excalifont").baseline(36.) - 31.716).abs() < 1e-12);
        assert_eq!(metrics("Cascadia").line_height, 1.2);
    }
}
