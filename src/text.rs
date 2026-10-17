//! Geist-only shaping and shared Unicode editing, selection, clipboard and history.
use cosmic_text::{
    Action, Attrs, Buffer, Cursor, Edit, Editor, Family, FontSystem, Metrics, Motion, Selection,
    Shaping,
};
use std::sync::Arc;
pub const FONTS: &[&[u8]] = &[
    include_bytes!("../assets/fonts/Geist.ttf"),
    include_bytes!("../assets/fonts/GeistMono.ttf"),
    include_bytes!("../assets/fonts/GeistPixel-Circle.ttf"),
    include_bytes!("../assets/fonts/GeistPixel-Grid.ttf"),
    include_bytes!("../assets/fonts/GeistPixel-Line.ttf"),
    include_bytes!("../assets/fonts/GeistPixel-Square.ttf"),
    include_bytes!("../assets/fonts/GeistPixel-Triangle.ttf"),
];
pub fn font_system() -> FontSystem {
    let mut db = cosmic_text::fontdb::Database::new();
    for (index, font) in FONTS.iter().enumerate() {
        let ids = db.load_font_source(cosmic_text::fontdb::Source::Binary(Arc::new(font.to_vec())));
        // Pixel cuts share a typographic family in the upstream font metadata.
        // Give each cut an internal alias so an application can select it explicitly.
        if index >= 2 {
            for id in ids {
                let mut face = db.face(id).unwrap().clone();
                face.families[0].0 = format!(
                    "Geist Pixel {}",
                    ["Circle", "Grid", "Line", "Square", "Triangle"][index - 2]
                );
                db.remove_face(id);
                db.push_face_info(face);
            }
        }
    }
    db.set_sans_serif_family("Geist");
    db.set_monospace_family("Geist Mono");
    db.set_serif_family("Geist");
    FontSystem::new_with_locale_and_db("en-US".into(), db)
}
#[derive(Clone)]
struct Snapshot {
    text: String,
    cursor: Cursor,
    selection: Selection,
}
pub struct TextEditor {
    pub editor: Editor<'static>,
    pub preedit: String,
    pub preedit_cursor: Option<(usize, usize)>,
    undo: Vec<Snapshot>,
    redo: Vec<Snapshot>,
    last_edit: f64,
    pub multiline: bool,
}
impl TextEditor {
    pub fn new(fonts: &mut FontSystem, text: &str, multiline: bool) -> Self {
        let mut b = Buffer::new(fonts, Metrics::new(14., if multiline { 22. } else { 20. }));
        b.set_wrap(cosmic_text::Wrap::None);
        b.set_text(
            text,
            &Attrs::new().family(Family::Name(if multiline { "Geist Mono" } else { "Geist" })),
            Shaping::Advanced,
            None,
        );
        Self {
            editor: Editor::new(b),
            preedit: String::new(),
            preedit_cursor: None,
            undo: vec![],
            redo: vec![],
            last_edit: f64::NEG_INFINITY,
            multiline,
        }
    }
    pub fn value(&self) -> String {
        self.editor.with_buffer(|b| {
            b.lines
                .iter()
                .map(|l| l.text())
                .collect::<Vec<_>>()
                .join("\n")
        })
    }
    fn snapshot(&self) -> Snapshot {
        Snapshot {
            text: self.value(),
            cursor: self.editor.cursor(),
            selection: self.editor.selection(),
        }
    }
    fn restore(&mut self, fonts: &mut FontSystem, s: Snapshot) {
        self.editor.with_buffer_mut(|b| {
            b.set_text(
                &s.text,
                &Attrs::new().family(Family::Name(if self.multiline {
                    "Geist Mono"
                } else {
                    "Geist"
                })),
                Shaping::Advanced,
                None,
            )
        });
        self.editor.set_cursor(s.cursor);
        self.editor.set_selection(s.selection);
        self.preedit.clear();
        self.editor.shape_as_needed(fonts, false);
    }
    fn checkpoint(&mut self, now: f64, coalesce: bool) {
        if !coalesce || now - self.last_edit > 0.3 || !self.redo.is_empty() {
            self.undo.push(self.snapshot());
            if self.undo.len() > 256 {
                self.undo.remove(0);
            }
        }
        self.redo.clear();
        self.last_edit = now;
    }
    pub fn insert(&mut self, fonts: &mut FontSystem, text: &str, now: f64) {
        let text = if self.multiline {
            text.to_owned()
        } else {
            text.replace(['\n', '\r'], "")
        };
        if text.is_empty() {
            return;
        }
        self.checkpoint(now, true);
        self.editor.insert_string(&text, None);
        self.editor.shape_as_needed(fonts, false);
        self.preedit.clear();
    }
    pub fn horizontal_scroll(&self) -> f32 {
        self.editor.with_buffer(|b| b.scroll().horizontal)
    }
    pub fn cursor_position(&self) -> Option<(f32, f32)> {
        let (x, y) = self.editor.cursor_position()?;
        Some((x as f32 - self.horizontal_scroll(), y as f32))
    }
    pub fn action(&mut self, fonts: &mut FontSystem, action: Action, shift: bool, now: f64) {
        let horizontal = self.horizontal_scroll() as i32;
        let action = match action {
            Action::Click { x, y } => Action::Click {
                x: x + horizontal,
                y,
            },
            Action::DoubleClick { x, y } => Action::DoubleClick {
                x: x + horizontal,
                y,
            },
            Action::TripleClick { x, y } => Action::TripleClick {
                x: x + horizontal,
                y,
            },
            Action::Drag { x, y } => Action::Drag {
                x: x + horizontal,
                y,
            },
            other => other,
        };
        let editing = matches!(
            action,
            Action::Backspace | Action::Delete | Action::Enter | Action::Indent | Action::Unindent
        );
        if editing {
            self.checkpoint(now, false)
        } else {
            self.last_edit = f64::NEG_INFINITY;
        }
        if matches!(action, Action::Motion(_)) {
            if shift {
                if matches!(self.editor.selection(), Selection::None) {
                    self.editor
                        .set_selection(Selection::Normal(self.editor.cursor()));
                }
            } else {
                self.editor.set_selection(Selection::None);
            }
        }
        self.editor.action(fonts, action);
        self.editor.shape_as_needed(fonts, false);
    }
    pub fn select_all(&mut self, fonts: &mut FontSystem) {
        self.editor
            .action(fonts, Action::Motion(Motion::BufferStart));
        self.editor
            .set_selection(Selection::Normal(self.editor.cursor()));
        self.editor.action(fonts, Action::Motion(Motion::BufferEnd));
        self.last_edit = f64::NEG_INFINITY;
    }
    pub fn undo(&mut self, fonts: &mut FontSystem) {
        if let Some(s) = self.undo.pop() {
            self.redo.push(self.snapshot());
            self.restore(fonts, s);
            self.last_edit = f64::NEG_INFINITY;
        }
    }
    pub fn redo(&mut self, fonts: &mut FontSystem) {
        if let Some(s) = self.redo.pop() {
            self.undo.push(self.snapshot());
            self.restore(fonts, s);
            self.last_edit = f64::NEG_INFINITY;
        }
    }
    pub fn cut(&mut self, fonts: &mut FontSystem, now: f64) -> Option<String> {
        let text = self.editor.copy_selection()?;
        self.checkpoint(now, false);
        self.editor.delete_selection();
        self.editor.shape_as_needed(fonts, false);
        Some(text)
    }
    pub fn resize(&mut self, fonts: &mut FontSystem, w: f32, h: f32, scale: f32) {
        self.editor.with_buffer_mut(|b| {
            b.set_metrics(Metrics::new(
                14. * scale,
                (if self.multiline { 22. } else { 20. }) * scale,
            ));
            b.set_size(Some(w), Some(h));
        });
        self.editor.shape_as_needed(fonts, false);
    }
}

/// Measure using the same shaping rules as the GPU renderer.
pub fn measure(
    fonts: &mut FontSystem,
    text: &str,
    role: crate::theme::TypeRole,
    scale: f32,
) -> f32 {
    let style = role.style();
    let mut b = Buffer::new(fonts, Metrics::new(style.size * scale, style.line * scale));
    b.set_text(
        text,
        &Attrs::new()
            .family(Family::Name(style.family))
            .weight(cosmic_text::Weight(style.weight))
            .letter_spacing(style.tracking),
        Shaping::Advanced,
        None,
    );
    b.shape_until_scroll(fonts, false);
    b.layout_runs().map(|r| r.line_w).fold(0., f32::max)
}
