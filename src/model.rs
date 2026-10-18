//! Scene edits and history are independent of GTK event delivery.
use crate::{canvas::*, config::Config, keys::Action};
use std::collections::BTreeSet;

const HISTORY_LIMIT: usize = 100;
#[derive(Clone)]
pub enum Drag {
    Draw {
        origin: Point,
    },
    Move {
        origin: Point,
        before: Vec<Element>,
        originals: Vec<Element>,
    },
    Resize {
        bounds: Bounds,
        before: Vec<Element>,
        originals: Vec<Element>,
    },
    Marquee {
        origin: Point,
        end: Point,
        previous: BTreeSet<usize>,
    },
    Erase {
        before: Vec<Element>,
        last: Point,
    },
    Pan {
        origin: Point,
        pan: Point,
    },
}

pub struct Scene {
    pub elements: Vec<Element>,
    pub selected: BTreeSet<usize>,
    pub preview: Option<Element>,
    pub editing: Option<Element>,
    pub edit_index: Option<usize>,
    pub drag: Option<Drag>,
    pub tool: Tool,
    pub tool_lock: bool,
    pub style: Style,
    pub pan: Point,
    pub zoom: f64,
    pub space_down: bool,
    pub clipboard: Vec<Element>,
    undo: Vec<Vec<Element>>,
    redo: Vec<Vec<Element>>,
}
impl Scene {
    pub fn new(config: &Config) -> Self {
        let d = &config.drawing;
        Self {
            elements: vec![],
            selected: BTreeSet::new(),
            preview: None,
            editing: None,
            edit_index: None,
            drag: None,
            tool: Tool::Freehand,
            tool_lock: d.tool_lock,
            style: Style {
                color: parse_color(&d.default_color).unwrap(),
                width: d.default_width,
                roughness: d.roughness,
                fill: parse_color(&d.fill_color).unwrap(),
                fill_style: d.fill_style,
                stroke_style: d.stroke_style,
                font: d.font_family.clone(),
                font_size: d.font_size,
                theme: d.theme,
                rounded: d.rounded,
                stroke_variability: d.stroke_variability,
            },
            pan: (0., 0.),
            zoom: 1.,
            space_down: false,
            clipboard: vec![],
            undo: vec![],
            redo: vec![],
        }
    }
    pub fn world(&self, p: Point) -> Point {
        (
            (p.0 - self.pan.0) / self.zoom,
            (p.1 - self.pan.1) / self.zoom,
        )
    }
    pub fn screen(&self, p: Point) -> Point {
        (p.0 * self.zoom + self.pan.0, p.1 * self.zoom + self.pan.1)
    }
    fn record(&mut self, before: Vec<Element>) {
        if before == self.elements {
            return;
        }
        if self.undo.len() == HISTORY_LIMIT {
            self.undo.remove(0);
        }
        self.undo.push(before);
        self.redo.clear();
    }
    pub fn undo(&mut self) {
        self.cancel_drag();
        if let Some(before) = self.undo.pop() {
            self.redo
                .push(std::mem::replace(&mut self.elements, before));
            self.selected.clear();
        }
    }
    pub fn redo(&mut self) {
        self.cancel_drag();
        if let Some(after) = self.redo.pop() {
            self.undo.push(std::mem::replace(&mut self.elements, after));
            self.selected.clear();
        }
    }
    pub fn selection_bounds(&self) -> Option<Bounds> {
        self.selected
            .iter()
            .filter_map(|&i| self.elements.get(i))
            .map(Element::bounds)
            .reduce(Bounds::union)
    }
    pub fn hit(&self, p: Point) -> Option<usize> {
        self.elements.iter().rposition(|e| e.hit(p, 6. / self.zoom))
    }
    pub fn set_tool(&mut self, tool: Tool) {
        self.cancel_drag();
        self.commit_text();
        self.tool = tool;
        self.selected.clear();
    }
    pub fn begin_text(&mut self, pos: Point, index: Option<usize>) {
        self.commit_text();
        self.edit_index = index;
        self.editing = Some(
            index
                .and_then(|i| self.elements.get(i))
                .cloned()
                .unwrap_or_else(|| {
                    Element::new(
                        Geometry::Text {
                            pos,
                            text: String::new(),
                        },
                        self.style.clone(),
                    )
                }),
        );
        self.selected.clear();
    }
    pub fn commit_text(&mut self) {
        if let Some(e) = self.editing.take() {
            let before = self.elements.clone();
            let nonempty = matches!(&e.geometry,Geometry::Text{text,..} if !text.trim().is_empty());
            if let Some(i) = self.edit_index.take() {
                if nonempty {
                    self.elements[i] = e;
                    self.selected = BTreeSet::from([i]);
                } else {
                    self.elements.remove(i);
                    self.selected.clear();
                }
            } else if nonempty {
                self.elements.push(e);
                self.selected = BTreeSet::from([self.elements.len() - 1]);
            }
            self.record(before);
            if !self.tool_lock {
                self.tool = Tool::Selection;
            }
        }
    }
    pub fn delete(&mut self) {
        let before = self.elements.clone();
        let selected = &self.selected;
        let mut index = 0;
        self.elements.retain(|_| {
            let keep = !selected.contains(&index);
            index += 1;
            keep
        });
        self.selected.clear();
        self.record(before);
    }
    pub fn clear(&mut self) {
        self.cancel_drag();
        self.editing = None;
        self.edit_index = None;
        let before = std::mem::take(&mut self.elements);
        self.selected.clear();
        self.record(before);
    }
    pub fn duplicate(&mut self, offset: Point) {
        let before = self.elements.clone();
        let mut new = BTreeSet::new();
        for i in self.selected.clone() {
            let mut e = self.elements[i].clone();
            e.translate(offset.0, offset.1);
            new.insert(self.elements.len());
            self.elements.push(e);
        }
        self.selected = new;
        self.record(before);
    }
    pub fn copy(&mut self) {
        self.clipboard = self
            .selected
            .iter()
            .map(|&i| self.elements[i].clone())
            .collect();
    }
    pub fn paste(&mut self) {
        if self.clipboard.is_empty() {
            return;
        }
        let before = self.elements.clone();
        self.selected.clear();
        for mut e in self.clipboard.clone() {
            e.translate(20., 20.);
            self.selected.insert(self.elements.len());
            self.elements.push(e);
        }
        self.record(before);
        self.tool = Tool::Selection;
    }
    pub fn nudge(&mut self, dx: f64, dy: f64) {
        let before = self.elements.clone();
        for &i in &self.selected {
            self.elements[i].translate(dx, dy);
        }
        self.record(before);
    }
    pub fn set_theme(&mut self, theme: Theme) {
        self.style.theme = theme;
        for element in self.elements.iter_mut().chain(self.clipboard.iter_mut()) {
            element.style.theme = theme;
        }
        for element in self.preview.iter_mut().chain(self.editing.iter_mut()) {
            element.style.theme = theme;
        }
        for snapshot in self.undo.iter_mut().chain(self.redo.iter_mut()) {
            for element in snapshot {
                element.style.theme = theme;
            }
        }
        if let Some(drag) = &mut self.drag {
            match drag {
                Drag::Move {
                    before, originals, ..
                }
                | Drag::Resize {
                    before, originals, ..
                } => {
                    for element in before.iter_mut().chain(originals.iter_mut()) {
                        element.style.theme = theme;
                    }
                }
                Drag::Erase { before, .. } => {
                    for element in before {
                        element.style.theme = theme;
                    }
                }
                _ => {}
            }
        }
    }
    pub fn restyle(&mut self, change: impl Fn(&mut Style)) {
        change(&mut self.style);
        if let Some(e) = &mut self.editing {
            change(&mut e.style);
        }
        let before = self.elements.clone();
        for &i in &self.selected {
            change(&mut self.elements[i].style);
        }
        self.record(before);
    }
    pub fn reorder(&mut self, action: Action) {
        if self.selected.is_empty() {
            return;
        }
        let before = self.elements.clone();
        let mut tagged: Vec<_> = self
            .elements
            .drain(..)
            .enumerate()
            .map(|(i, e)| (self.selected.contains(&i), e))
            .collect();
        match action {
            Action::BringFront => tagged.sort_by_key(|(s, _)| *s),
            Action::SendBack => tagged.sort_by_key(|(s, _)| !*s),
            Action::BringForward => {
                for i in (0..tagged.len().saturating_sub(1)).rev() {
                    if tagged[i].0 && !tagged[i + 1].0 {
                        tagged.swap(i, i + 1);
                    }
                }
            }
            Action::SendBackward => {
                for i in 1..tagged.len() {
                    if tagged[i].0 && !tagged[i - 1].0 {
                        tagged.swap(i, i - 1);
                    }
                }
            }
            _ => {}
        }
        self.selected = tagged
            .iter()
            .enumerate()
            .filter_map(|(i, (s, _))| s.then_some(i))
            .collect();
        self.elements = tagged.into_iter().map(|(_, e)| e).collect();
        self.record(before);
    }
    pub fn zoom_at(&mut self, factor: f64, screen: Point) {
        let world = self.world(screen);
        self.zoom = (self.zoom * factor).clamp(0.1, 8.);
        self.pan = (
            screen.0 - world.0 * self.zoom,
            screen.1 - world.1 * self.zoom,
        );
    }
    pub fn zoom_fit(&mut self, w: f64, h: f64) {
        if let Some(b) = self
            .elements
            .iter()
            .map(Element::bounds)
            .reduce(Bounds::union)
        {
            self.zoom = ((w - 160.) / b.w.max(1.))
                .min((h - 180.) / b.h.max(1.))
                .clamp(0.1, 8.);
            self.pan = (
                (w - b.w * self.zoom) / 2. - b.x * self.zoom,
                (h - b.h * self.zoom) / 2. - b.y * self.zoom,
            );
        }
    }
    pub fn pointer_down(&mut self, screen: Point, shift: bool, alt: bool, middle: bool) {
        self.commit_text();
        let p = self.world(screen);
        if middle || self.space_down || self.tool == Tool::Hand {
            self.drag = Some(Drag::Pan {
                origin: screen,
                pan: self.pan,
            });
            return;
        }
        match self.tool {
            Tool::Text => self.begin_text(p, None),
            Tool::Selection => {
                if let Some(b) = self.selection_bounds() {
                    if distance(p, (b.x + b.w, b.y + b.h)) <= 9. / self.zoom {
                        self.drag = Some(Drag::Resize {
                            bounds: b,
                            before: self.elements.clone(),
                            originals: self.elements.clone(),
                        });
                        return;
                    }
                }
                if let Some(i) = self.hit(p) {
                    if shift {
                        if !self.selected.insert(i) {
                            self.selected.remove(&i);
                            return;
                        }
                    } else if !self.selected.contains(&i) {
                        self.selected = BTreeSet::from([i]);
                    }
                    let before = self.elements.clone();
                    if alt {
                        let originals: Vec<_> = self
                            .selected
                            .iter()
                            .map(|&i| self.elements[i].clone())
                            .collect();
                        self.selected.clear();
                        for e in originals {
                            self.selected.insert(self.elements.len());
                            self.elements.push(e);
                        }
                    }
                    self.drag = Some(Drag::Move {
                        origin: p,
                        before,
                        originals: self.elements.clone(),
                    });
                } else {
                    if !shift {
                        self.selected.clear();
                    }
                    self.drag = Some(Drag::Marquee {
                        origin: p,
                        end: p,
                        previous: self.selected.clone(),
                    });
                }
            }
            Tool::Eraser => {
                self.selected.clear();
                self.drag = Some(Drag::Erase {
                    before: self.elements.clone(),
                    last: p,
                });
                self.erase_at(p);
            }
            tool => {
                self.selected.clear();
                let geometry = if tool == Tool::Freehand {
                    Geometry::Stroke(vec![p])
                } else {
                    Geometry::Shape {
                        tool,
                        start: p,
                        end: p,
                    }
                };
                self.preview = Some(Element::new(geometry, self.style.clone()));
                self.drag = Some(Drag::Draw { origin: p });
            }
        }
    }
    fn erase_at(&mut self, p: Point) {
        let zoom = self.zoom;
        self.elements.retain(|e| !e.hit(p, 10. / zoom));
    }
    pub fn pointer_move(&mut self, screen: Point, shift: bool, alt: bool) {
        let p = self.world(screen);
        if let Some(mut drag) = self.drag.take() {
            match &mut drag {
                Drag::Pan { origin, pan } => {
                    self.pan = (pan.0 + screen.0 - origin.0, pan.1 + screen.1 - origin.1)
                }
                Drag::Draw { origin } => {
                    if let Some(e) = &mut self.preview {
                        match &mut e.geometry {
                            Geometry::Stroke(points) => {
                                if points
                                    .last()
                                    .is_none_or(|&last| distance(last, p) >= 0.5 / self.zoom)
                                {
                                    points.push(p);
                                }
                            }
                            Geometry::Shape { tool, start, end } => {
                                (*start, *end) = constrained_shape(*tool, *origin, p, shift, alt);
                            }
                            _ => {}
                        }
                    }
                }
                Drag::Move {
                    origin, originals, ..
                } => {
                    self.elements.clone_from(originals);
                    let mut dx = p.0 - origin.0;
                    let mut dy = p.1 - origin.1;
                    if shift {
                        if dx.abs() > dy.abs() {
                            dy = 0.;
                        } else {
                            dx = 0.;
                        }
                    }
                    for &i in &self.selected {
                        self.elements[i].translate(dx, dy);
                    }
                }
                Drag::Resize {
                    bounds, originals, ..
                } => {
                    self.elements.clone_from(originals);
                    let mut w = (p.0 - bounds.x).max(1.);
                    let mut h = (p.1 - bounds.y).max(1.);
                    if shift {
                        let scale = (w / bounds.w.max(1.)).max(h / bounds.h.max(1.));
                        w = bounds.w * scale;
                        h = bounds.h * scale;
                    }
                    let to = Bounds {
                        x: bounds.x,
                        y: bounds.y,
                        w,
                        h,
                    };
                    for &i in &self.selected {
                        self.elements[i].resize(*bounds, to);
                    }
                }
                Drag::Marquee {
                    origin,
                    end,
                    previous,
                } => {
                    let b = Bounds::between(*origin, p);
                    self.selected = previous.clone();
                    for (i, e) in self.elements.iter().enumerate() {
                        if b.encloses(e.bounds()) {
                            self.selected.insert(i);
                        }
                    }
                    *end = p;
                }
                Drag::Erase { last, .. } => {
                    let steps = (distance(*last, p) / (5. / self.zoom))
                        .ceil()
                        .clamp(1., 2000.) as usize;
                    for i in 1..=steps {
                        let t = i as f64 / steps as f64;
                        self.erase_at((last.0 + (p.0 - last.0) * t, last.1 + (p.1 - last.1) * t));
                    }
                    *last = p;
                }
            }
            self.drag = Some(drag);
        }
    }
    pub fn pointer_up(&mut self, screen: Point, shift: bool, alt: bool) {
        self.pointer_move(screen, shift, alt);
        match self.drag.take() {
            Some(Drag::Draw { .. }) => {
                if let Some(e) = self.preview.take() {
                    let b = e.bounds();
                    if matches!(e.geometry, Geometry::Stroke(_)) || b.w > 2. || b.h > 2. {
                        let before = self.elements.clone();
                        self.elements.push(e);
                        self.record(before);
                        if !self.tool_lock && self.tool != Tool::Freehand {
                            self.tool = Tool::Selection;
                            self.selected = BTreeSet::from([self.elements.len() - 1]);
                        }
                    }
                }
            }
            Some(
                Drag::Move { before, .. }
                | Drag::Resize { before, .. }
                | Drag::Erase { before, .. },
            ) => self.record(before),
            _ => {}
        }
    }
    pub fn cancel_drag(&mut self) {
        if let Some(
            Drag::Move { before, .. } | Drag::Resize { before, .. } | Drag::Erase { before, .. },
        ) = self.drag.take()
        {
            self.elements = before;
            self.selected.clear();
        }
        self.preview = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn scene() -> Scene {
        Scene::new(&Config::default())
    }
    fn rect(s: &mut Scene) {
        s.tool = Tool::Rectangle;
        s.pointer_down((10., 10.), false, false, false);
        s.pointer_up((110., 90.), false, false);
    }
    #[test]
    fn undo_redo_and_clear_are_transactions() {
        let mut s = scene();
        rect(&mut s);
        let e = s.elements.clone();
        s.clear();
        assert!(s.elements.is_empty());
        s.undo();
        assert_eq!(s.elements, e);
        s.undo();
        assert!(s.elements.is_empty());
        s.redo();
        assert_eq!(s.elements, e);
        s.undo();
        rect(&mut s);
        s.redo();
        assert_eq!(s.elements.len(), 1);
    }
    #[test]
    fn drag_cancel_restores_and_release_keeps_final_position() {
        let mut s = scene();
        rect(&mut s);
        s.set_tool(Tool::Selection);
        let original = s.elements.clone();
        s.pointer_down((10., 40.), false, false, false);
        s.pointer_move((60., 80.), false, false);
        assert_ne!(s.elements, original);
        s.cancel_drag();
        assert_eq!(s.elements, original);
        s.pointer_down((10., 40.), false, false, false);
        s.pointer_up((60., 80.), false, false);
        assert_eq!(s.elements[0].bounds().x, 60.);
        s.undo();
        assert_eq!(s.elements, original);
    }
    #[test]
    fn eraser_is_single_undo_even_across_fast_motion() {
        let mut s = scene();
        rect(&mut s);
        s.tool = Tool::Eraser;
        s.pointer_down((0., 40.), false, false, false);
        s.pointer_up((120., 40.), false, false);
        assert!(s.elements.is_empty());
        s.undo();
        assert_eq!(s.elements.len(), 1);
    }
    #[test]
    fn theme_changes_the_whole_scene_and_survives_undo() {
        let mut s = scene();
        rect(&mut s);
        rect(&mut s);
        s.set_theme(Theme::Light);
        assert!(s.elements.iter().all(|e| e.style.theme == Theme::Light));
        s.undo();
        s.redo();
        assert!(s.elements.iter().all(|e| e.style.theme == Theme::Light));
        assert_eq!(s.style.theme, Theme::Light);
    }
    #[test]
    fn zoom_preserves_pointer_world_position() {
        let mut s = scene();
        s.pan = (30., 40.);
        let p = (200., 300.);
        let world = s.world(p);
        s.zoom_at(2., p);
        assert_eq!(s.world(p), world);
    }
    #[test]
    fn multiline_text_and_empty_edit_history() {
        let mut s = scene();
        s.begin_text((10., 10.), None);
        if let Geometry::Text { text, .. } = &mut s.editing.as_mut().unwrap().geometry {
            *text = "hello\nworld".into();
        }
        s.commit_text();
        assert_eq!(s.elements.len(), 1);
        s.undo();
        assert!(s.elements.is_empty());
        s.redo();
        s.begin_text((0., 0.), Some(0));
        if let Geometry::Text { text, .. } = &mut s.editing.as_mut().unwrap().geometry {
            text.clear();
        }
        s.commit_text();
        assert!(s.elements.is_empty());
        s.undo();
        assert_eq!(s.elements.len(), 1);
    }
}
