//! Retained interaction state. Input is applied before animation and rendering.
//! Pointer-down activates commands. Continuous controls capture until release;
//! thumb drags commit on release. Focus/disabled are independent state layers.
use crate::{
    animation::{Clock, Curve, Scroll, Transform, Tween},
    geometry::{Point, Rect},
    layout::{self, Grid, Layout},
    scene::{Primitive, Scene},
    text::{TextEditor, font_system},
    theme::{Color, Theme, TypeRole},
    widget::{ButtonStyle, Id, Kind, Widget},
};
use cosmic_text::{Action as TextAction, Edit, Motion};
use std::collections::{BTreeMap, BTreeSet};
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    Number(f64),
    Text(String),
    Selection(usize),
}
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Changed(Id, Value),
    Click(Id),
    Context(Id, Point),
    MidiLearn(Id),
    MidiBound(Id, u8),
    Reorder { item: Id, before: Id },
    Dismissed,
    Command(String),
}
#[derive(Clone, Copy, Debug, Default)]
pub struct Modifiers {
    pub shift: bool,
    pub control: bool,
    pub alt: bool,
    pub command: bool,
}
impl Modifiers {
    fn shortcut(self) -> bool {
        self.control || self.command
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Key {
    Tab,
    Space,
    Enter,
    Escape,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Backspace,
    Delete,
    Character(String),
    F9,
    F10,
    F11,
    F12,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointerButton {
    Primary,
    Secondary,
}
#[derive(Clone, Debug)]
pub enum Input {
    /// Wake an embedded host to consume application-owned parameter channels.
    Refresh,
    Leave,
    Move(Point),
    Down {
        position: Point,
        button: PointerButton,
        touch: bool,
    },
    Up(Point),
    Cancel,
    Wheel {
        delta: f32,
    },
    Key {
        key: Key,
        modifiers: Modifiers,
    },
    Text(String),
    ImePreedit(String, Option<(usize, usize)>),
    ImeCommit(String),
    Midi {
        cc: u8,
        value: u8,
    },
    Focus(bool),
}
struct State {
    value: Tween,
    press: Tween,
    model: f64,
    hover_since: Option<f64>,
    editor: Option<TextEditor>,
    kind: std::mem::Discriminant<Kind>,
    choice_widths: Vec<f32>,
    bounds: Option<Rect>,
    order: usize,
    reorder: Option<(Rect, Tween)>,
    word: bool,
    word_at: f64,
    learn: bool,
    cc: Option<u8>,
    bound_at: f64,
    peak: f32,
    peak_at: f64,
    appeared: f64,
    live_at: f64,
    live_due: f64,
    live_secondary: bool,
    live_swapped: bool,
    hit_at: f64,
}
impl State {
    fn new(w: &Widget, now: f64, fonts: &mut cosmic_text::FontSystem) -> Self {
        Self {
            value: Tween::new(w.value(), w.value(), now, 0., Curve::OutExpo),
            press: Tween::new(0., 0., now, 0., Curve::OutExpo),
            model: w.value(),
            hover_since: None,
            kind: std::mem::discriminant(&w.kind),
            choice_widths: Vec::new(),
            bounds: None,
            order: usize::MAX,
            reorder: None,
            editor: match &w.kind {
                Kind::Input {
                    value, multiline, ..
                } => Some(TextEditor::new(fonts, value, *multiline)),
                _ => None,
            },
            word: w.value() >= 0.5,
            word_at: -1.,
            learn: false,
            cc: None,
            bound_at: -1.,
            peak: -60.,
            peak_at: now,
            appeared: now,
            live_at: -1000.,
            live_due: now + w.live_tile.as_ref().map_or(f64::INFINITY, |l| l.interval),
            live_secondary: false,
            live_swapped: false,
            hit_at: -1000.,
        }
    }
}
#[derive(Clone)]
struct Node {
    w: Widget,
    bounds: Rect,
    clip: Rect,
}
#[derive(Clone, Debug)]
struct Drag {
    id: Id,
    start: Point,
    last: Point,
    last_at: f64,
    velocity: f32,
    value: f64,
    thumb: bool,
    touch: bool,
    moved: bool,
    started: f64,
}
#[derive(Clone, Debug)]
pub enum Overlay {
    CommandPalette {
        query: String,
        commands: Vec<String>,
    },
    Dialog {
        title: String,
        message: String,
        actions: Vec<String>,
    },
    Menu {
        position: Point,
        items: Vec<String>,
        owner: Option<Id>,
    },
    Picker {
        owner: Id,
        options: Vec<String>,
        selected: usize,
        bounds: Rect,
    },
}
#[derive(Clone, Debug)]
struct Popup {
    kind: Overlay,
    at: f64,
    selected: usize,
    previous_focus: Option<Id>,
    scroll: f32,
    candidate: Tween,
}
#[derive(Clone, Debug)]
struct ClosingPopup {
    popup: Popup,
    at: f64,
    duration: f64,
    bounds: Rect,
    rows: Vec<Rect>,
    reveal: Rect,
    opacity: f32,
    dim_opacity: f32,
    selected: usize,
}
#[derive(Clone, Debug)]
struct Toast {
    text: String,
    at: f64,
}
#[derive(Clone, Copy, Debug, Default)]
pub struct DebugOptions {
    pub grid: bool,
    pub baselines: bool,
    pub hit_areas: bool,
    pub timings: bool,
}
pub struct Ui {
    pub clock: Clock,
    pub theme: Theme,
    previous_theme: Theme,
    theme_at: f64,
    nodes: Vec<Node>,
    states: BTreeMap<Id, State>,
    pub focus: Option<Id>,
    keyboard_focus: bool,
    focus_rect: [Tween; 4],
    hover: Option<Id>,
    pointer: Point,
    drag: Option<Drag>,
    pub modifiers: Modifiers,
    events: Vec<Event>,
    fonts: cosmic_text::FontSystem,
    clipboard: Option<arboard::Clipboard>,
    popup: Option<Popup>,
    closing_popup: Option<ClosingPopup>,
    toast: Option<Toast>,
    pub debug: DebugOptions,
    pub viewport: Rect,
    pub scroll: Scroll,
    scroll_bounds: Option<Rect>,
    scroll_at: f64,
    last_click: Option<(Id, f64)>,
    pub frame_ms: f32,
    page_at: f64,
    page_back: bool,
    page_outgoing: Option<Scene>,
    page_pending: bool,
    touch_start: Option<Point>,
    touch_scrolling: bool,
    touch_velocity: f32,
    pointer_at: f64,
    pub touch: bool,
    window_focused: bool,
}
impl Default for Ui {
    fn default() -> Self {
        Self::new(Theme::default())
    }
}
impl Ui {
    pub fn new(theme: Theme) -> Self {
        Self {
            clock: Clock::default(),
            previous_theme: theme.clone(),
            theme,
            theme_at: -1.,
            nodes: vec![],
            states: BTreeMap::new(),
            focus: None,
            keyboard_focus: false,
            focus_rect: [Tween::new(0., 0., 0., 0., Curve::OutExpo); 4],
            hover: None,
            pointer: Point::default(),
            drag: None,
            modifiers: Modifiers::default(),
            events: vec![],
            fonts: font_system(),
            clipboard: None,
            popup: None,
            closing_popup: None,
            toast: None,
            debug: DebugOptions::default(),
            viewport: Rect::default(),
            scroll: Scroll::default(),
            scroll_bounds: None,
            scroll_at: -1.,
            last_click: None,
            frame_ms: 0.,
            page_at: -10.,
            page_back: false,
            page_outgoing: None,
            page_pending: false,
            touch_start: None,
            touch_scrolling: false,
            touch_velocity: 0.,
            pointer_at: 0.,
            touch: false,
            window_focused: true,
        }
    }
    pub fn set_theme(&mut self, theme: Theme) {
        if self.theme != theme {
            self.previous_theme = self.current_theme();
            self.theme = theme;
            self.theme_at = self.clock.now();
        }
    }
    pub fn current_theme(&self) -> Theme {
        let t = ((self.clock.now() - self.theme_at) / 0.2).clamp(0., 1.) as f32;
        self.previous_theme.mix(&self.theme, t)
    }
    pub fn drain_events(&mut self) -> impl Iterator<Item = Event> + '_ {
        self.events.drain(..)
    }
    pub fn show_dialog(
        &mut self,
        title: impl Into<String>,
        message: impl Into<String>,
        actions: Vec<String>,
    ) {
        self.open(Overlay::Dialog {
            title: title.into(),
            message: message.into(),
            actions,
        });
    }
    pub fn command_palette(&mut self, commands: Vec<String>) {
        self.open(Overlay::CommandPalette {
            query: String::new(),
            commands,
        });
    }
    pub fn context_menu(&mut self, point: Point, items: Vec<String>) {
        self.open(Overlay::Menu {
            position: point,
            items,
            owner: None,
        });
    }
    pub fn notify(&mut self, text: impl Into<String>) {
        self.toast = Some(Toast {
            text: text.into(),
            at: self.clock.now(),
        });
    }
    fn open(&mut self, kind: Overlay) {
        self.input(Input::Cancel);
        self.closing_popup = None;
        let selected = if let Overlay::Picker { selected, .. } = &kind {
            *selected
        } else {
            0
        };
        self.popup = Some(Popup {
            kind,
            selected,
            at: self.clock.now(),
            previous_focus: self.focus,
            scroll: 0.,
            candidate: Tween::new(
                selected as f64,
                selected as f64,
                self.clock.now(),
                0.,
                Curve::OutExpo,
            ),
        });
        self.keep_picker_candidate_visible();
        self.keyboard_focus = false;
    }
    pub fn dismiss(&mut self) {
        self.close_popup(None);
    }
    fn close_popup(&mut self, selection: Option<usize>) {
        if let Some(p) = self.popup.take() {
            self.focus = p.previous_focus;
            if !matches!(p.kind, Overlay::Menu { .. }) {
                let now = self.clock.now();
                let (bounds, rows) = self.popup_geometry(&p);
                let desktop = matches!(p.kind, Overlay::Picker { .. }) && self.viewport.w >= 720.;
                let selected = selection.unwrap_or(match &p.kind {
                    Overlay::Picker { selected, .. } => *selected,
                    _ => p.selected,
                });
                let reveal = if desktop {
                    self.picker_reveal_rect(&p, now)
                } else {
                    bounds
                };
                self.closing_popup = Some(ClosingPopup {
                    opacity: Curve::OutExpo.sample(((now - p.at) / 0.12).clamp(0., 1.)) as f32,
                    dim_opacity: ((now - p.at) / 0.16).clamp(0., 1.) as f32,
                    popup: p,
                    at: now,
                    duration: if self.theme.reduce_motion {
                        0.1
                    } else if desktop {
                        0.2
                    } else {
                        0.14
                    },
                    bounds,
                    rows,
                    reveal,
                    selected,
                });
            }
            self.events.push(Event::Dismissed);
        }
    }
    pub fn navigate(&mut self, back: bool) {
        self.page_outgoing = Some(self.scene());
        self.page_pending = true;
        self.page_back = back;
        self.page_at = self.clock.now();
        self.scroll.offset = 0.;
    }
    /// Diff keyed nodes; removing a key also releases focus and pointer capture.
    pub fn reconcile(&mut self, root: Widget, viewport: Rect) {
        self.viewport = viewport;
        let now = self.clock.now();
        let mut nodes = vec![];
        struct LayoutPass {
            nodes: Vec<Node>,
            scale: f32,
            offset: f32,
            scroll: Option<(Rect, f32)>,
        }
        impl LayoutPass {
            fn visit(
                &mut self,
                mut w: Widget,
                fallback: Rect,
                clip: Rect,
                offset: f32,
                disabled: bool,
            ) {
                w.disabled |= disabled;
                let bounds = w.bounds.unwrap_or(fallback).translate(0., -offset);
                let children = std::mem::take(&mut w.children);
                let scale = self.scale;
                let sizes: Vec<_> = children
                    .iter()
                    .map(|c| {
                        let mut s = c.size;
                        s.height *= scale;
                        s.width = s.width.map(|v| v * scale);
                        s
                    })
                    .collect();
                let placements: Vec<_> = match w.layout {
                    Layout::Vertical { gap, padding } => {
                        layout::stack(bounds, &sizes, false, gap * scale, padding * scale)
                    }
                    Layout::Horizontal { gap, padding } => {
                        layout::stack(bounds, &sizes, true, gap * scale, padding * scale)
                    }
                    Layout::Tiles { columns } => children
                        .iter()
                        .enumerate()
                        .map(|(i, c)| {
                            Rect::new(
                                bounds.x + (i % columns.max(1)) as f32 * 104. * scale,
                                bounds.y + (i / columns.max(1)) as f32 * 104. * scale,
                                c.size.width.unwrap_or(96.) * scale,
                                c.size.height * scale,
                            )
                        })
                        .collect(),
                    _ => children.iter().map(|_| bounds).collect(),
                };
                if w.scrollable {
                    assert!(
                        self.scroll.is_none(),
                        "Only one scroll viewport can be mounted per Ui"
                    );
                    let bottom = children
                        .iter()
                        .zip(&placements)
                        .map(|(c, r)| c.bounds.unwrap_or(*r).bottom())
                        .fold(bounds.bottom(), f32::max);
                    self.scroll = Some((bounds, (bottom - bounds.bottom()).max(0.)));
                }
                let scrollable = w.scrollable;
                let disabled = w.disabled;
                self.nodes.push(Node { w, bounds, clip });
                for (child, r) in children.into_iter().zip(placements) {
                    // Explicit bounds are absolute logical coordinates. Stack bounds already
                    // contain their ancestor's translation, so apply each offset exactly once.
                    let delta = if scrollable { self.offset } else { 0. };
                    self.visit(child, r, clip.intersect(bounds), delta, disabled);
                }
            }
        }
        let mut pass = LayoutPass {
            nodes: Vec::new(),
            scale: self.theme.text_scale,
            offset: self.scroll.offset,
            scroll: None,
        };
        pass.visit(root, viewport, viewport, 0., false);
        self.scroll_bounds = pass.scroll.map(|v| v.0);
        self.scroll.max = pass.scroll.map_or(0., |v| v.1);
        nodes.append(&mut pass.nodes);
        let mut keys = BTreeSet::new();
        for (order, n) in nodes.iter().enumerate() {
            assert!(
                n.w.id.0 != 0,
                "Widget ID zero is reserved for the accessibility root"
            );
            assert!(keys.insert(n.w.id), "duplicate widget key {:?}", n.w.id);
            let w = &n.w;
            let s = self
                .states
                .entry(w.id)
                .or_insert_with(|| State::new(w, now, &mut self.fonts));
            if s.kind != std::mem::discriminant(&w.kind) {
                *s = State::new(w, now, &mut self.fonts);
            }
            if s.order != usize::MAX
                && s.order != order
                && matches!(w.kind, Kind::ListItem { .. })
                && let Some(old) = s.bounds
            {
                let from = if let Some((from, tween)) = s.reorder {
                    from.lerp(old, tween.sample(now).0 as f32)
                } else {
                    old
                };
                s.reorder = Some((
                    from,
                    Tween::new(
                        0.,
                        1.,
                        now,
                        if self.theme.reduce_motion { 0.1 } else { 0.16 },
                        Curve::OutExpo,
                    ),
                ));
            }
            s.order = order;
            s.bounds = Some(n.bounds);

            if let Kind::Choice { options, pivot, .. } = &w.kind {
                let role = if *pivot {
                    TypeRole::Title
                } else {
                    TypeRole::Body
                };
                s.choice_widths = options
                    .iter()
                    .map(|text| {
                        crate::text::measure(&mut self.fonts, text, role, self.theme.text_scale)
                    })
                    .collect();
            }
            if s.model != w.value() {
                s.model = w.value();
                s.value.retarget(
                    w.value(),
                    now,
                    if self.theme.reduce_motion {
                        0.1
                    } else if matches!(w.kind, Kind::Toggle { .. }) {
                        0.18
                    } else {
                        0.24
                    },
                    if matches!(w.kind, Kind::Progress { .. }) {
                        Curve::Linear
                    } else {
                        Curve::OutExpo
                    },
                );
            }
            if let Kind::Input {
                value, multiline, ..
            } = &w.kind
            {
                if s.editor.as_ref().is_none_or(|e| e.value() != *value) {
                    s.editor = Some(TextEditor::new(&mut self.fonts, value, *multiline));
                }
                if let Some(e) = &mut s.editor {
                    let r = input_rect(n);
                    e.resize(
                        &mut self.fonts,
                        (r.w - 24.).max(1.),
                        (r.h - 8.).max(1.),
                        self.theme.text_scale,
                    );
                }
            }
        }
        self.states.retain(|id, _| keys.contains(id));
        self.nodes = nodes;
        if self
            .focus
            .is_some_and(|id| !self.nodes.iter().any(|n| n.w.id == id && n.w.interactive()))
        {
            self.focus = None;
        }
        if self.drag.as_ref().is_some_and(|d| !keys.contains(&d.id)) {
            self.drag = None;
        }
        if self.page_pending {
            self.page_pending = false;
        }
        self.update_focus_rect();
    }
    fn node(&self, id: Id) -> Option<Node> {
        self.nodes.iter().find(|n| n.w.id == id).cloned()
    }
    fn hit(&self, p: Point) -> Option<Id> {
        self.nodes
            .iter()
            .rev()
            .find(|n| {
                n.w.interactive()
                    && ((hit_bounds(n.bounds, if self.touch { 48. } else { 40. }).contains(p)
                        && n.clip.contains(p))
                        || self.pivot_previous_rect(n).is_some_and(|r| r.contains(p)))
            })
            .map(|n| n.w.id)
    }
    fn update_focus_rect(&mut self) {
        if let Some(n) = self.focus.and_then(|id| self.node(id)) {
            let r = n.bounds.inset(-4.);
            let values = [r.x, r.y, r.w, r.h];
            for (t, v) in self.focus_rect.iter_mut().zip(values) {
                if t.target != v as f64 {
                    t.retarget(
                        v as f64,
                        self.clock.now(),
                        if self.theme.reduce_motion { 0. } else { 0.14 },
                        Curve::OutExpo,
                    );
                }
            }
        }
    }
    fn change(&mut self, id: Id, value: Value, animate: bool) {
        let now = self.clock.now();
        let Some(s) = self.states.get_mut(&id) else {
            return;
        };
        let n = match value {
            Value::Bool(v) => Some(u8::from(v) as f64),
            Value::Number(v) => Some(v),
            Value::Selection(v) => Some(v as f64),
            _ => None,
        };
        if let Some(n) = n {
            s.model = n;
            if animate {
                s.value.retarget(
                    n,
                    now,
                    if self.theme.reduce_motion { 0.1 } else { 0.18 },
                    Curve::OutExpo,
                )
            } else {
                s.value = Tween::new(n, n, now, 0., Curve::Linear)
            }
        }
        self.events.push(Event::Changed(id, value));
    }
    fn activate(&mut self, id: Id) {
        let Some(n) = self.node(id) else { return };
        if !n.w.interactive() {
            return;
        }
        let value = self.states[&id].model;
        match n.w.kind {
            Kind::Toggle { .. } => self.change(id, Value::Bool(value < 0.5), true),
            Kind::Picker { options, .. } => self.open(Overlay::Picker {
                owner: id,
                options,
                selected: value as usize,
                bounds: n.bounds,
            }),
            Kind::ListItem { .. } => {
                self.events.push(Event::Click(id));
            }
            Kind::Tile { .. } => {
                self.states.get_mut(&id).unwrap().hit_at = self.clock.now();
                self.events.push(Event::Click(id));
            }
            Kind::Button { .. } | Kind::IconButton { .. } => self.events.push(Event::Click(id)),
            _ => {}
        }
    }
    pub fn focus_widget(&mut self, id: Id) {
        if self.node(id).is_some_and(|n| n.w.interactive()) {
            self.focus = Some(id);
            self.keyboard_focus = true;
            self.update_focus_rect();
        }
    }
    pub fn begin_midi_learn(&mut self, id: Id) {
        if !self
            .node(id)
            .is_some_and(|n| n.w.interactive() && n.w.range().is_some())
        {
            return;
        }
        for s in self.states.values_mut() {
            s.learn = false;
        }
        if let Some(s) = self.states.get_mut(&id) {
            s.learn = true;
            self.events.push(Event::MidiLearn(id));
        }
    }
    pub fn accessibility_activate(&mut self, id: Id) {
        self.activate(id);
    }
    pub fn accessibility_value(&mut self, id: Id, value: f64) {
        if let Some((min, max, _, _)) = self.node(id).and_then(|n| n.w.range()) {
            self.change(id, Value::Number(value.clamp(min, max)), false);
        }
    }
    pub fn input(&mut self, input: Input) {
        let now = self.clock.now();
        if let Input::Midi { cc, value } = input {
            let learned = self.states.iter().find(|(_, s)| s.learn).map(|(id, _)| *id);
            if let Some(id) = learned {
                let s = self.states.get_mut(&id).unwrap();
                s.learn = false;
                s.cc = Some(cc);
                s.bound_at = now;
                self.events.push(Event::MidiBound(id, cc));
            }
            let ids: Vec<_> = self
                .states
                .iter()
                .filter(|(_, s)| s.cc == Some(cc))
                .map(|(id, _)| *id)
                .collect();
            for id in ids {
                if let Some((min, max, _, _)) = self
                    .node(id)
                    .filter(|n| n.w.interactive())
                    .and_then(|n| n.w.range())
                {
                    self.change(
                        id,
                        Value::Number(min + (max - min) * value as f64 / 127.),
                        false,
                    );
                }
            }
            return;
        }
        if self.popup.is_some() {
            self.popup_input(input);
            return;
        }
        match input {
            Input::Refresh => {}
            Input::Leave => {
                if let Some(s) = self.hover.and_then(|id| self.states.get_mut(&id)) {
                    s.hover_since = None;
                }
                self.hover = None;
            }
            Input::Move(p) => {
                let old = self.pointer;
                let dt = (now - self.pointer_at).max(0.001);
                self.pointer_at = now;
                if let Some(start) = self
                    .touch_start
                    .filter(|start| self.scroll_bounds.is_some_and(|r| r.contains(*start)))
                {
                    let range_drag = self
                        .drag
                        .as_ref()
                        .and_then(|d| self.node(d.id))
                        .is_some_and(|n| {
                            n.w.range().is_some()
                                || matches!(n.w.kind, Kind::Toggle { .. } | Kind::Input { .. })
                        });
                    if !range_drag
                        && (self.touch_scrolling
                            || ((p.y - start.y).abs() > 8.
                                && (p.y - start.y).abs() > (p.x - start.x).abs()))
                    {
                        self.touch_scrolling = true;
                        self.scroll.drag(old.y - p.y);
                        self.touch_velocity = (old.y - p.y) / dt as f32;
                        self.scroll_at = now;
                        if let Some(d) = self.drag.take() {
                            self.states.get_mut(&d.id).unwrap().press.retarget(
                                0.,
                                now,
                                0.22,
                                Curve::OutExpo,
                            );
                        }
                    }
                }
                self.pointer = p;
                let hit = self.hit(p);
                if hit != self.hover {
                    if let Some(s) = self.hover.and_then(|i| self.states.get_mut(&i)) {
                        s.hover_since = None
                    }
                    if let Some(s) = hit.and_then(|i| self.states.get_mut(&i)) {
                        s.hover_since = Some(now)
                    }
                    self.hover = hit;
                }
                if let Some(mut drag) = self.drag.take() {
                    if let Some(n) = self.node(drag.id) {
                        drag.moved |= (p.x - drag.start.x).abs() + (p.y - drag.start.y).abs() > 4.;
                        let dt = (now - drag.last_at).max(0.001);
                        drag.velocity = (p.x - drag.last.x) / dt as f32;
                        match n.w.kind {
                            Kind::Toggle { .. } if drag.thumb => {
                                let v =
                                    (drag.value + (p.x - drag.start.x) as f64 / 28.).clamp(0., 1.);
                                self.states.get_mut(&drag.id).unwrap().value =
                                    Tween::new(v, v, now, 0., Curve::Linear);
                            }
                            Kind::Slider { min, max, .. } => {
                                let sensitivity = if self.modifiers.shift { 0.1 } else { 1. };
                                let v = (self.states[&drag.id].model
                                    + (p.x - drag.last.x) as f64 / n.bounds.w.max(1.) as f64
                                        * (max - min)
                                        * sensitivity)
                                    .clamp(min, max);
                                self.change(drag.id, Value::Number(v), false);
                            }
                            Kind::Knob { min, max, .. } => {
                                let sensitivity = if self.modifiers.shift { 0.1 } else { 1. };
                                let v = (self.states[&drag.id].model
                                    + (drag.last.y - p.y) as f64 / 200.
                                        * (max - min)
                                        * sensitivity)
                                    .clamp(min, max);
                                self.change(drag.id, Value::Number(v), false);
                            }
                            Kind::Input { .. } => {
                                let r = input_rect(&n);
                                if let Some(e) =
                                    self.states.get_mut(&drag.id).unwrap().editor.as_mut()
                                {
                                    e.action(
                                        &mut self.fonts,
                                        TextAction::Drag {
                                            x: (p.x - r.x - 12.) as i32,
                                            y: (p.y - r.y - 8.) as i32,
                                        },
                                        false,
                                        now,
                                    );
                                }
                            }
                            _ => {}
                        }
                    }
                    drag.last = p;
                    drag.last_at = now;
                    self.drag = Some(drag);
                }
            }
            Input::Down {
                position: p,
                button,
                touch,
            } => {
                self.pointer = p;
                self.pointer_at = now;
                self.touch = touch;
                if touch {
                    self.touch_start = Some(p);
                    self.scroll.velocity = 0.;
                }
                self.keyboard_focus = false;
                let Some(id) = self.hit(p) else {
                    self.focus = None;
                    return;
                };
                self.focus = Some(id);
                let n = self.node(id).unwrap();
                if button == PointerButton::Secondary {
                    self.events.push(Event::Context(id, p));
                    if n.w.range().is_some() {
                        self.open(Overlay::Menu {
                            position: p,
                            items: vec!["MIDI learn".into()],
                            owner: Some(id),
                        });
                    }
                    return;
                }
                let double = self
                    .last_click
                    .is_some_and(|(i, t)| i == id && now - t < 0.35);
                self.last_click = Some((id, now));
                let v = self.states[&id].value.sample(now).0;
                self.states
                    .get_mut(&id)
                    .unwrap()
                    .press
                    .retarget(1., now, 0.06, Curve::OutExpo);
                let mut thumb = false;
                match &n.w.kind {
                    Kind::Toggle { .. } => {
                        let track = toggle_track(n.bounds);
                        let tr =
                            Rect::new(track.x + 6. + v as f32 * 28. - 6., track.y - 8., 24., 36.);
                        thumb = tr.contains(p);
                        if !thumb {
                            self.activate(id)
                        }
                    }
                    Kind::Slider {
                        min, max, default, ..
                    }
                    | Kind::Knob {
                        min, max, default, ..
                    } => {
                        if double {
                            {
                                self.change(id, Value::Number(*default), true);
                                let s = self.states.get_mut(&id).unwrap();
                                s.value.duration =
                                    if self.theme.reduce_motion { 0.1 } else { 0.16 };
                            }
                        } else if matches!(n.w.kind, Kind::Slider { .. }) {
                            let value = (*min
                                + ((p.x - n.bounds.x) / n.bounds.w).clamp(0., 1.) as f64
                                    * (*max - *min))
                                .clamp(*min, *max);
                            self.change(id, Value::Number(value), false);
                        }
                    }
                    Kind::Choice { options, .. } => {
                        if self.pivot_previous_rect(&n).is_some_and(|r| r.contains(p)) {
                            let previous = (self.states[&id].model as usize).saturating_sub(1);
                            self.change(id, Value::Selection(previous), true);
                        } else {
                            let spans = self.choice_spans(&n);
                            if self.choice_clip(&n).contains(p)
                                && let Some(i) = spans.iter().position(|r| r.contains(p))
                                && i < options.len()
                            {
                                self.change(id, Value::Selection(i), true);
                            }
                        }
                    }
                    Kind::Input { .. } => {
                        let r = input_rect(&n);
                        if p.x > r.right() - 32.
                            && !self.states[&id].editor.as_ref().unwrap().multiline
                        {
                            let e = self.states.get_mut(&id).unwrap().editor.as_mut().unwrap();
                            e.select_all(&mut self.fonts);
                            e.action(&mut self.fonts, TextAction::Delete, false, now);
                            self.events.push(Event::Changed(id, Value::Text(e.value())));
                        } else {
                            let action = if double {
                                TextAction::DoubleClick {
                                    x: (p.x - r.x - 12.) as i32,
                                    y: (p.y - r.y - 8.) as i32,
                                }
                            } else {
                                TextAction::Click {
                                    x: (p.x - r.x - 12.) as i32,
                                    y: (p.y - r.y - 8.) as i32,
                                }
                            };
                            self.states
                                .get_mut(&id)
                                .unwrap()
                                .editor
                                .as_mut()
                                .unwrap()
                                .action(&mut self.fonts, action, false, now);
                        }
                    }
                    _ => self.activate(id),
                }
                if self.popup.is_some() {
                    self.states
                        .get_mut(&id)
                        .unwrap()
                        .press
                        .retarget(0., now, 0.22, Curve::OutExpo);
                    return;
                }
                self.drag = Some(Drag {
                    id,
                    start: p,
                    last: p,
                    last_at: now,
                    velocity: 0.,
                    value: if double {
                        v
                    } else {
                        self.states[&id].value.sample(now).0
                    },
                    thumb,
                    touch,
                    moved: false,
                    started: now,
                });
                self.update_focus_rect();
            }
            Input::Up(p) => {
                if self.touch_scrolling {
                    self.scroll.velocity = self.touch_velocity;
                    self.touch_scrolling = false;
                    self.touch_start = None;
                    self.scroll_at = now;
                }
                if let Some(d) = self.drag.take()
                    && let Some(n) = self.node(d.id)
                {
                    if d.thumb {
                        let v = self.states[&d.id].value.sample(now).0;
                        let on = if d.velocity.abs() > 600. {
                            d.velocity > 0.
                        } else if !d.moved {
                            d.value < 0.5
                        } else {
                            v >= 0.5
                        };
                        self.change(d.id, Value::Bool(on), true);
                    }
                    if d.moved
                        && matches!(n.w.kind, Kind::ListItem { .. })
                        && let Some(before) = self.hit(p).filter(|id| *id != d.id)
                    {
                        self.events.push(Event::Reorder { item: d.id, before });
                    }
                    self.states.get_mut(&d.id).unwrap().press.retarget(
                        0.,
                        now,
                        0.22,
                        Curve::OutExpo,
                    );
                }
                if let Some(start) = self.touch_start.take()
                    && (p.x - start.x).abs() > 80.
                {
                    let pivot = self
                        .nodes
                        .iter()
                        .find(|n| matches!(n.w.kind, Kind::Choice { pivot: true, .. }))
                        .cloned();
                    if let Some(n) = pivot
                        && let Kind::Choice { options, .. } = n.w.kind
                    {
                        let current = self.states[&n.w.id].model as usize;
                        let selected = if p.x < start.x {
                            (current + 1).min(options.len().saturating_sub(1))
                        } else {
                            current.saturating_sub(1)
                        };
                        self.change(n.w.id, Value::Selection(selected), true);
                    }
                }
            }
            Input::Cancel => {
                if let Some(d) = self.drag.take()
                    && let Some(s) = self.states.get_mut(&d.id)
                {
                    s.press.retarget(0., now, 0.22, Curve::OutExpo);
                    if d.thumb {
                        s.value.retarget(s.model, now, 0.18, Curve::OutExpo)
                    }
                }
                self.touch_start = None;
                self.touch_scrolling = false;
                self.touch_velocity = 0.;
            }
            Input::Wheel { delta } => {
                if let Some(id) = self.hover {
                    if let Some(editor) = self.states.get_mut(&id).and_then(|s| s.editor.as_mut()) {
                        editor.action(
                            &mut self.fonts,
                            TextAction::Scroll {
                                pixels: -delta * 22.,
                            },
                            false,
                            now,
                        );
                        return;
                    }
                    if let Some((min, max, step, _)) = self.node(id).and_then(|n| n.w.range()) {
                        let factor = if self.modifiers.shift { 0.1 } else { 1. };
                        self.change(
                            id,
                            Value::Number(
                                (self.states[&id].model + delta as f64 * step * factor)
                                    .clamp(min, max),
                            ),
                            false,
                        );
                        return;
                    }
                }
                self.scroll.drag(-delta * 32.);
                self.scroll_at = now;
            }
            Input::Key { key, modifiers } => {
                self.modifiers = modifiers;
                self.key(key);
            }
            Input::Text(text) | Input::ImeCommit(text) => {
                if let Some(id) = self.focus
                    && let Some(e) = self.states.get_mut(&id).and_then(|s| s.editor.as_mut())
                {
                    e.insert(&mut self.fonts, &text, now);
                    self.events.push(Event::Changed(id, Value::Text(e.value())));
                }
            }
            Input::ImePreedit(text, cursor) => {
                if let Some(e) = self
                    .focus
                    .and_then(|id| self.states.get_mut(&id))
                    .and_then(|s| s.editor.as_mut())
                {
                    e.preedit = text;
                    e.preedit_cursor = cursor;
                }
            }
            Input::Focus(f) => {
                self.window_focused = f;
                if !f {
                    self.input(Input::Cancel);
                }
            }
            Input::Midi { .. } => {}
        }
    }
    fn key(&mut self, key: Key) {
        let m = self.modifiers;
        match key {
            Key::F9 => {
                self.debug.grid = !self.debug.grid;
                return;
            }
            Key::F10 => {
                self.debug.baselines = !self.debug.baselines;
                return;
            }
            Key::F11 => {
                self.debug.hit_areas = !self.debug.hit_areas;
                return;
            }
            Key::F12 => {
                self.debug.timings = !self.debug.timings;
                return;
            }
            Key::Character(ref c) if m.shortcut() && m.shift && c.eq_ignore_ascii_case("s") => {
                self.clock.slow = !self.clock.slow;
                return;
            }
            Key::Space if m.shortcut() && m.shift => {
                self.clock.paused = !self.clock.paused;
                return;
            }
            Key::Character(ref c) if m.shortcut() && m.shift && c == "." => {
                self.clock.step(1. / 144.);
                return;
            }
            Key::Escape => {
                for s in self.states.values_mut() {
                    s.learn = false;
                }
                if let Some(e) = self
                    .focus
                    .and_then(|id| self.states.get_mut(&id))
                    .and_then(|s| s.editor.as_mut())
                {
                    e.preedit.clear();
                    e.action(&mut self.fonts, TextAction::Escape, false, self.clock.now());
                }
                return;
            }
            Key::Tab => {
                let ids: Vec<_> = self
                    .nodes
                    .iter()
                    .filter(|n| n.w.interactive())
                    .map(|n| n.w.id)
                    .collect();
                if ids.is_empty() {
                    return;
                }
                let i = self.focus.and_then(|id| ids.iter().position(|i| *i == id));
                let next = if m.shift {
                    i.map_or(ids.len() - 1, |i| (i + ids.len() - 1) % ids.len())
                } else {
                    i.map_or(0, |i| (i + 1) % ids.len())
                };
                self.focus = Some(ids[next]);
                self.keyboard_focus = true;
                self.update_focus_rect();
                return;
            }
            _ => {}
        }
        let Some(id) = self.focus else { return };
        let Some(n) = self.node(id) else { return };
        self.keyboard_focus = true;
        if self.states[&id].editor.is_some() {
            self.edit_key(id, key);
            return;
        }
        if matches!(key, Key::Space | Key::Enter) {
            self.activate(id);
            return;
        }
        if matches!(n.w.kind, Kind::Toggle { .. }) {
            match key {
                Key::Left => self.change(id, Value::Bool(false), true),
                Key::Right => self.change(id, Value::Bool(true), true),
                _ => {}
            }
            return;
        }
        if let Some((min, max, step, default)) = n.w.range() {
            let value = self.states[&id].model;
            let step = step * if m.shift { 0.1 } else { 1. };
            let v = match key {
                Key::Left | Key::Down => Some(value - step),
                Key::Right | Key::Up => Some(value + step),
                Key::PageDown => Some(value - step * 10.),
                Key::PageUp => Some(value + step * 10.),
                Key::Home => Some(min),
                Key::End => Some(max),
                Key::Backspace => Some(default),
                _ => None,
            };
            if let Some(v) = v {
                self.change(id, Value::Number(v.clamp(min, max)), false);
            }
            return;
        }
        if let Kind::Choice { options, .. } = n.w.kind {
            let current = self.states[&id].model as usize;
            let selected = match key {
                Key::Left | Key::Up => current.saturating_sub(1),
                Key::Right | Key::Down => (current + 1).min(options.len().saturating_sub(1)),
                Key::Home => 0,
                Key::End => options.len().saturating_sub(1),
                _ => current,
            };
            self.change(id, Value::Selection(selected), true);
        } else if matches!(n.w.kind, Kind::ListItem { .. }) && matches!(key, Key::Up | Key::Down) {
            let ids: Vec<_> = self
                .nodes
                .iter()
                .filter(|n| n.w.interactive() && matches!(n.w.kind, Kind::ListItem { .. }))
                .map(|n| n.w.id)
                .collect();
            if let Some(i) = ids.iter().position(|i| *i == id) {
                let next = if key == Key::Up {
                    i.saturating_sub(1)
                } else {
                    (i + 1).min(ids.len() - 1)
                };
                if m.alt {
                    self.events.push(Event::Reorder {
                        item: id,
                        before: ids[next],
                    });
                } else {
                    self.focus_widget(ids[next]);
                }
            }
        }
    }
    fn edit_key(&mut self, id: Id, key: Key) {
        let now = self.clock.now();
        let m = self.modifiers;
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
        let e = self.states.get_mut(&id).unwrap().editor.as_mut().unwrap();
        let before = e.value();
        let shortcut = if let Key::Character(c) = &key {
            if m.shortcut() {
                Some(c.to_lowercase())
            } else {
                None
            }
        } else {
            None
        };
        if let Some(c) = shortcut {
            match c.as_str() {
                "a" => e.select_all(&mut self.fonts),
                "c" => {
                    if let Some(text) = e.editor.copy_selection()
                        && let Some(cb) = &mut self.clipboard
                    {
                        let _ = cb.set_text(text);
                    }
                }
                "x" => {
                    if let Some(text) = e.cut(&mut self.fonts, now)
                        && let Some(cb) = &mut self.clipboard
                    {
                        let _ = cb.set_text(text);
                    }
                }
                "v" => {
                    if let Some(text) = self.clipboard.as_mut().and_then(|cb| cb.get_text().ok()) {
                        e.insert(&mut self.fonts, &text, now);
                    }
                }
                "z" => {
                    if m.shift {
                        e.redo(&mut self.fonts)
                    } else {
                        e.undo(&mut self.fonts)
                    }
                }
                "y" => e.redo(&mut self.fonts),
                _ => {}
            }
        } else {
            let action = match key {
                Key::Left => Some(TextAction::Motion(if m.shortcut() {
                    Motion::LeftWord
                } else {
                    Motion::Left
                })),
                Key::Right => Some(TextAction::Motion(if m.shortcut() {
                    Motion::RightWord
                } else {
                    Motion::Right
                })),
                Key::Up => Some(TextAction::Motion(Motion::Up)),
                Key::Down => Some(TextAction::Motion(Motion::Down)),
                Key::Home => Some(TextAction::Motion(if m.shortcut() {
                    Motion::BufferStart
                } else {
                    Motion::Home
                })),
                Key::End => Some(TextAction::Motion(if m.shortcut() {
                    Motion::BufferEnd
                } else {
                    Motion::End
                })),
                Key::PageUp => Some(TextAction::Motion(Motion::PageUp)),
                Key::PageDown => Some(TextAction::Motion(Motion::PageDown)),
                Key::Backspace => Some(TextAction::Backspace),
                Key::Delete => Some(TextAction::Delete),
                Key::Enter if e.multiline => Some(TextAction::Enter),
                _ => None,
            };
            if let Some(action) = action {
                e.action(&mut self.fonts, action, m.shift, now);
            }
        }
        let after = e.value();
        if before != after {
            self.events.push(Event::Changed(id, Value::Text(after)));
        }
    }
    fn popup_geometry(&self, p: &Popup) -> (Rect, Vec<Rect>) {
        match &p.kind {
            Overlay::CommandPalette { query, commands } => {
                let labels = palette_matches(query, commands);
                let left =
                    Grid::new(self.viewport.w, self.viewport.h, self.theme.text_scale).margin;
                let h = (96. + labels.len() as f32 * 48.).min(self.viewport.h);
                let b = Rect::new(0., 0., self.viewport.w, h);
                (
                    b,
                    labels
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            Rect::new(left, 88. + i as f32 * 48., self.viewport.w - left * 2., 48.)
                        })
                        .collect(),
                )
            }
            Overlay::Dialog { actions, .. } => {
                let b = Rect::new(0., (self.viewport.h - 200.) / 2., self.viewport.w, 200.);
                let left =
                    Grid::new(self.viewport.w, self.viewport.h, self.theme.text_scale).margin;
                let rows = actions
                    .iter()
                    .enumerate()
                    .map(|(i, _)| Rect::new(left + i as f32 * 160., b.bottom() - 56., 144., 40.))
                    .collect();
                (b, rows)
            }
            Overlay::Menu {
                position, items, ..
            } => {
                let w = 200f32.min(self.viewport.w);
                let h = items.len() as f32 * 36.;
                let b = Rect::new(
                    position.x.min(self.viewport.right() - w).max(0.),
                    position.y.min(self.viewport.bottom() - h).max(0.),
                    w,
                    h,
                );
                (
                    b,
                    items
                        .iter()
                        .enumerate()
                        .map(|(i, _)| Rect::new(b.x, b.y + i as f32 * 36., w, 36.))
                        .collect(),
                )
            }
            Overlay::Picker {
                bounds,
                options,
                selected,
                ..
            } => {
                let scale = self.theme.text_scale;
                let row_height = PICKER_ROW_HEIGHT * scale;
                let compact = self.viewport.w < 720.;
                let padding = if compact {
                    0.
                } else {
                    PICKER_MENU_PADDING * scale
                };
                let margin = if compact { 24. } else { 8. };
                let available_height = (self.viewport.h - 2. * margin).max(0.);
                let content_height = options.len() as f32 * row_height;
                let h = (content_height + 2. * padding).min(available_height);
                // On desktop, align the selected option's text with the closed
                // value, rather than placing the first option below the field.
                let anchor = picker_value_rect(*bounds, scale).y - (row_height - 24. * scale) / 2.;
                let desired_y = if compact {
                    margin
                } else {
                    anchor - padding - *selected as f32 * row_height
                };
                let top = self.viewport.y + margin;
                let y = desired_y.clamp(top, (self.viewport.bottom() - margin - h).max(top));
                let width = if compact {
                    (self.viewport.w - 2. * margin).max(0.)
                } else {
                    bounds.w.min((self.viewport.w - 2. * margin).max(0.))
                };
                let x = if compact {
                    self.viewport.x + margin
                } else {
                    bounds.x.clamp(
                        self.viewport.x + margin,
                        (self.viewport.right() - margin - width).max(self.viewport.x + margin),
                    )
                };
                let b = Rect::new(x, y, width, h);
                let visible_height = (h - 2. * padding).max(0.);
                let max_scroll = (content_height - visible_height).max(0.);
                // Hover does not move the menu under the pointer. Only explicit
                // scrolling or keyboard navigation changes the overflow offset.
                let scroll = p.scroll.clamp(0., max_scroll);
                (
                    b,
                    options
                        .iter()
                        .enumerate()
                        .map(|(i, _)| {
                            Rect::new(
                                b.x,
                                b.y + padding + i as f32 * row_height - scroll,
                                b.w,
                                row_height,
                            )
                        })
                        .collect(),
                )
            }
        }
    }
    fn keep_picker_candidate_visible(&mut self) {
        let Some(p) = self.popup.as_ref() else { return };
        if !matches!(p.kind, Overlay::Picker { .. }) {
            return;
        }
        let (bounds, rows) = self.popup_geometry(p);
        let padding = if self.viewport.w < 720. {
            0.
        } else {
            PICKER_MENU_PADDING * self.theme.text_scale
        };
        let inner = bounds.inset(padding);
        let Some(row) = rows.get(p.selected) else {
            return;
        };
        let adjustment = if row.y < inner.y {
            row.y - inner.y
        } else if row.bottom() > inner.bottom() {
            row.bottom() - inner.bottom()
        } else {
            0.
        };
        let scroll = (p.scroll + adjustment).max(0.);
        self.popup.as_mut().unwrap().scroll = scroll;
    }
    fn popup_input(&mut self, input: Input) {
        if let Some(Popup {
            kind: Overlay::CommandPalette { query, .. },
            selected,
            ..
        }) = &mut self.popup
        {
            use unicode_segmentation::UnicodeSegmentation;
            match &input {
                Input::Text(text) | Input::ImeCommit(text) => {
                    query.push_str(text);
                    *selected = 0;
                    return;
                }
                Input::Key {
                    key: Key::Backspace,
                    ..
                } => {
                    if let Some((index, _)) = query.grapheme_indices(true).next_back() {
                        query.truncate(index);
                    }
                    *selected = 0;
                    return;
                }
                Input::Key {
                    key: Key::Space, ..
                } => return,
                _ => {}
            }
        }
        let keyboard_navigation = matches!(
            &input,
            Input::Key {
                key: Key::Up | Key::Down | Key::Left | Key::Right | Key::Tab | Key::Home | Key::End,
                ..
            }
        );
        let p = self.popup.as_ref().unwrap().clone();
        let (full_bounds, rows) = self.popup_geometry(&p);
        let bounds = if matches!(p.kind, Overlay::Picker { .. }) && self.viewport.w >= 720. {
            self.picker_reveal_rect(&p, self.clock.now())
        } else {
            full_bounds
        };
        let mut commit = None;
        match input {
            Input::Key { key, modifiers } => match key {
                Key::Escape => {
                    self.dismiss();
                    return;
                }
                Key::Up | Key::Left => {
                    self.popup.as_mut().unwrap().selected = p.selected.saturating_sub(1);
                    self.keyboard_focus = true;
                }
                Key::Down | Key::Right | Key::Tab => {
                    let next = if modifiers.shift {
                        p.selected.saturating_sub(1)
                    } else {
                        (p.selected + 1) % rows.len().max(1)
                    };
                    self.popup.as_mut().unwrap().selected = next;
                    self.keyboard_focus = true;
                }
                Key::Home => self.popup.as_mut().unwrap().selected = 0,
                Key::End => self.popup.as_mut().unwrap().selected = rows.len().saturating_sub(1),
                Key::Enter | Key::Space => commit = Some(p.selected),
                _ => {}
            },
            Input::Down { position, .. } => {
                self.pointer = position;
                commit = rows
                    .iter()
                    .position(|r| bounds.intersect(*r).contains(position));
                if commit.is_none() && !bounds.contains(position) {
                    self.dismiss();
                }
            }
            Input::Wheel { delta } if matches!(p.kind, Overlay::Picker { .. }) => {
                let padding = if self.viewport.w < 720. {
                    0.
                } else {
                    PICKER_MENU_PADDING * self.theme.text_scale
                };
                let content_height = rows.len() as f32 * PICKER_ROW_HEIGHT * self.theme.text_scale;
                let max_scroll = (content_height - (full_bounds.h - 2. * padding).max(0.)).max(0.);
                self.popup.as_mut().unwrap().scroll =
                    (p.scroll - delta * 32. * self.theme.text_scale).clamp(0., max_scroll);
                self.keyboard_focus = false;
            }
            Input::Move(position) => {
                self.pointer = position;
                if let Some(i) = rows
                    .iter()
                    .position(|r| bounds.intersect(*r).contains(position))
                {
                    let p = self.popup.as_mut().unwrap();
                    p.selected = i;
                    p.candidate =
                        Tween::new(i as f64, i as f64, self.clock.now(), 0., Curve::OutExpo);
                    self.keyboard_focus = false;
                }
            }
            _ => {}
        }
        if keyboard_navigation {
            self.keyboard_focus = true;
            self.keep_picker_candidate_visible();
            if let Some(p) = &mut self.popup {
                p.candidate.retarget(
                    p.selected as f64,
                    self.clock.now(),
                    if self.theme.reduce_motion
                        || self.viewport.w < 720.
                        || !matches!(p.kind, Overlay::Picker { .. })
                    {
                        0.
                    } else {
                        0.14
                    },
                    Curve::OutExpo,
                );
            }
        }
        if let Some(i) = commit.filter(|i| *i < rows.len()) {
            match p.kind {
                Overlay::CommandPalette { query, commands } => self.events.push(Event::Command(
                    palette_matches(&query, &commands)[i].clone(),
                )),
                Overlay::Dialog { actions, .. } => {
                    self.events.push(Event::Command(actions[i].clone()))
                }
                Overlay::Menu { items, owner, .. } => {
                    if let Some(id) = owner {
                        if items[i] == "MIDI learn" {
                            self.begin_midi_learn(id);
                        }
                    } else {
                        self.events.push(Event::Command(items[i].clone()));
                    }
                }
                Overlay::Picker { owner, .. } => {
                    self.change(owner, Value::Selection(i), false);
                    self.close_popup(Some(i));
                    return;
                }
            }
            self.dismiss();
        }
    }
    pub fn advance(&mut self, dt: f64) {
        let before = self.clock.now();
        self.clock.advance(dt);
        let now = self.clock.now();
        let dt = now - before;
        self.scroll.tick(dt as f32);
        for (id, s) in &mut self.states {
            if let Some(live) = self
                .nodes
                .iter()
                .find(|n| n.w.id == *id)
                .and_then(|n| n.w.live_tile.as_ref())
            {
                if now >= s.live_due && self.hover != Some(*id) {
                    s.live_at = now;
                    s.live_due = now + live.interval;
                    s.live_swapped = false;
                }
                if s.live_at >= s.appeared
                    && now - s.live_at >= if self.theme.reduce_motion { 0.05 } else { 0.2 }
                    && !s.live_swapped
                {
                    s.live_secondary = !s.live_secondary;
                    s.live_swapped = true;
                }
            }
            let word = s.value.sample(now).0 >= 0.5;
            if word != s.word {
                s.word = word;
                s.word_at = now;
            }
            if let Some(n) = self.nodes.iter().find(|n| n.w.id == *id)
                && let Kind::Meter { db, .. } = n.w.kind
            {
                if db > s.peak {
                    s.peak = db;
                    s.peak_at = now;
                } else if now - s.peak_at > 0.8 {
                    s.peak = (s.peak - 40. * dt as f32).max(db);
                }
            }
        }
        if self.toast.as_ref().is_some_and(|t| now - t.at > 2.82) {
            self.toast = None;
        }
        if self
            .closing_popup
            .as_ref()
            .is_some_and(|closing| now - closing.at >= closing.duration)
        {
            self.closing_popup = None;
        }
        if now - self.page_at > 0.5 {
            self.page_outgoing = None;
        }
        if let Some(d) = self
            .drag
            .as_ref()
            .filter(|d| d.touch && !d.moved && now - d.started >= 0.5)
            .cloned()
        {
            self.events.push(Event::Context(d.id, d.last));
            if self.node(d.id).is_some_and(|n| n.w.range().is_some()) {
                self.open(Overlay::Menu {
                    position: d.last,
                    items: vec!["MIDI learn".into()],
                    owner: Some(d.id),
                });
            }
            self.drag = None;
        }
    }
    pub fn needs_frame(&self) -> bool {
        let now = self.clock.now();
        if self.clock.paused {
            return false;
        }
        self.scroll.velocity.abs() > 0.1
            || self.scroll.offset < -0.1
            || self.scroll.offset > self.scroll.max + 0.1
            || (now - self.scroll_at < 1.0)
            || self.theme_at + 0.2 > now
            || self.page_at + 0.65 > now
            || self.states.values().any(|s| {
                s.value.active(now)
                    || now - s.hit_at < 0.3
                    || now - s.live_at < if self.theme.reduce_motion { 0.1 } else { 0.4 }
                    || s.reorder.is_some_and(|(_, t)| t.active(now))
                    || s.press.active(now)
                    || now - s.word_at < 0.17
                    || s.learn
                    || now - s.bound_at < 0.36
            })
            || self
                .focus_rect
                .iter()
                .any(|t| self.keyboard_focus && t.active(now))
            || self.toast.is_some()
            || self.closing_popup.is_some()
            || self
                .popup
                .as_ref()
                .is_some_and(|p| self.keyboard_focus && p.candidate.active(now))
            || self.popup.as_ref().is_some_and(|p| {
                now - p.at
                    < if matches!(&p.kind,Overlay::Picker{options,..} if options.len()>8) {
                        0.65
                    } else {
                        0.45
                    }
            })
            || self.drag.as_ref().is_some_and(|d| d.touch && !d.moved)
            || self
                .states
                .values()
                .any(|s| s.hover_since.is_some_and(|t| now - t < 0.7))
            || self
                .nodes
                .iter()
                .filter(|n| matches!(n.w.kind, Kind::Tile { .. }))
                .enumerate()
                .any(|(i, n)| {
                    now - self.states[&n.w.id].appeared
                        < if self.theme.reduce_motion {
                            0.1
                        } else {
                            0.4 + i as f64 * 0.03
                        }
                })
            || self
                .nodes
                .iter()
                .any(|n| matches!(n.w.kind, Kind::Progress { value: None }))
            || self
                .states
                .values()
                .any(|s| s.peak > -60. && now - s.peak_at < 2.4)
    }
    /// Next timer boundary: a focused caret sleeps between hard blink edges.
    pub fn next_wakeup(&self) -> Option<f64> {
        if !self.window_focused || self.clock.paused {
            return None;
        }
        let now = self.clock.now();
        let caret = self.text_focused().then(|| 0.53 - now.rem_euclid(0.53));
        let live = self
            .states
            .iter()
            .filter(|(id, _)| self.hover != Some(**id))
            .map(|(_, s)| s.live_due - now)
            .filter(|v| v.is_finite())
            .map(|v| v.max(0.001))
            .min_by(f64::total_cmp);
        match (caret, live) {
            (Some(a), Some(b)) => Some(a.min(b)),
            (a, b) => a.or(b),
        }
    }
    pub fn ime_cursor_rect(&self) -> Option<Rect> {
        let n = self.focus.and_then(|id| self.node(id))?;
        let editor = self.states.get(&n.w.id)?.editor.as_ref()?;
        let (x, y) = editor.cursor_position()?;
        let b = input_rect(&n);
        Some(Rect::new(
            b.x + 12. + x,
            b.y + 8. + y,
            2.,
            20. * self.theme.text_scale,
        ))
    }
    pub fn drag_restore_position(&self) -> Option<Point> {
        self.drag
            .as_ref()
            .and_then(|d| self.node(d.id))
            .and_then(|n| matches!(n.w.kind, Kind::Knob { .. }).then_some(n.bounds.center()))
    }
    pub fn dragging(&self) -> bool {
        self.drag
            .as_ref()
            .and_then(|d| self.node(d.id))
            .is_some_and(|n| n.w.range().is_some())
    }
    pub fn text_focused(&self) -> bool {
        if matches!(
            self.popup.as_ref().map(|p| &p.kind),
            Some(Overlay::CommandPalette { .. })
        ) {
            return true;
        }
        self.focus
            .and_then(|id| self.states.get(&id))
            .is_some_and(|s| s.editor.is_some())
    }
    pub fn scene(&self) -> Scene {
        let mut scene = Scene::default();
        let theme = self.current_theme();
        let now = self.clock.now();
        for (index, n) in self.nodes.iter().enumerate() {
            let start = scene.draws.len();
            self.paint_node(&mut scene, n, &theme);
            if now - self.page_at < 0.65 {
                let t = crate::animation::feather(now - self.page_at, index, theme.reduce_motion);
                scene.transform_since(start, t, Point::new(n.bounds.x, n.bounds.center().y));
            }
        }
        if let Some(old) = &self.page_outgoing {
            let elapsed = now - self.page_at;
            let t = Curve::InExpo.sample((elapsed / 0.25).clamp(0., 1.)) as f32;
            let mut old = old.clone();
            old.transform_since(
                0,
                crate::animation::turnstile(t, self.page_back, true, theme.reduce_motion),
                Point::new(self.viewport.x, self.viewport.center().y),
            );
            if elapsed < 0.25 {
                old.draws.append(&mut scene.draws);
                scene = old;
            }
        }
        if self.keyboard_focus
            && self.popup.is_none()
            && self.closing_popup.is_none()
            && self.focus.is_some()
        {
            let v = self.focus_rect.map(|t| t.sample(now).0 as f32);
            scene.stroke(
                Rect::new(v[0], v[1], v[2], v[3]),
                2.,
                theme.accent,
                self.viewport,
            );
        }
        if let Some(closing) = &self.closing_popup {
            let p = &closing.popup;
            if matches!(p.kind, Overlay::Picker { .. }) && self.viewport.w >= 720. {
                self.paint_desktop_picker(&mut scene, p, &theme, Some(closing));
            } else {
                let start = scene.draws.len();
                self.paint_popup(&mut scene, p, &theme);
                let f = Curve::InExpo.sample(((now - closing.at) / closing.duration).clamp(0., 1.))
                    as f32;
                for (index, draw) in scene.draws[start..].iter_mut().enumerate() {
                    draw.transform.opacity *= 1. - f;
                    if index > 0 && !theme.reduce_motion {
                        draw.transform.y += 24. * f;
                    }
                }
            }
        }
        if let Some(p) = &self.popup {
            self.paint_popup(&mut scene, p, &theme);
        }
        if let Some(toast) = &self.toast {
            let age = now - toast.at;
            let opacity = if age < 0.16 {
                (age / 0.16) as f32
            } else if age > 2.66 {
                ((2.82 - age) / 0.16) as f32
            } else {
                1.
            };
            let r = Rect::new(
                0.,
                if theme.reduce_motion {
                    0.
                } else {
                    -8. * (1. - opacity)
                },
                self.viewport.w,
                48.,
            );
            scene.rect(r, theme.surface.alpha(opacity), self.viewport);
            scene.stroke(r, 1., theme.line.alpha(opacity), self.viewport);
            scene.text(
                r.inset(12.),
                &toast.text,
                TypeRole::Body,
                theme.fg.alpha(opacity),
                self.viewport,
            );
        }
        if self.popup.is_none()
            && self.drag.is_none()
            && let Some(n) = self.hover.and_then(|id| self.node(id))
            && let Some(text) = self.tooltip_text(&n)
            && let Some(at) = self.states[&n.w.id].hover_since
        {
            let elapsed = now - at - 0.6;
            if elapsed >= 0. {
                let a = (elapsed / 0.1).clamp(0., 1.) as f32;
                let w = (text.chars().count() as f32 * 6.5 + 24.).min(self.viewport.w - 16.);
                let anchor = self
                    .pivot_previous_rect(&n)
                    .filter(|r| r.contains(self.pointer))
                    .unwrap_or(n.bounds);
                let r = Rect::new(
                    anchor.x.min(self.viewport.w - w - 8.).max(8.),
                    (anchor.bottom() + 8.).min(self.viewport.h - 40.),
                    w,
                    32.,
                );
                scene.rect(r, theme.surface.alpha(a), self.viewport);
                scene.stroke(r, 1., theme.line.alpha(a), self.viewport);
                scene.text(
                    r.inset(8.),
                    &text,
                    TypeRole::Caption,
                    theme.fg.alpha(a),
                    self.viewport,
                );
            }
        }
        if let Some(bounds) = self.scroll_bounds.filter(|_| self.scroll.max > 0.) {
            let edge = self.pointer.x > bounds.right() - 12. && bounds.contains(self.pointer);
            let age = now - self.scroll_at;
            let alpha = if edge {
                1.
            } else if age < 0.12 {
                (age / 0.12) as f32
            } else if age < 0.8 {
                1.
            } else {
                ((1. - age) / 0.2).clamp(0., 1.) as f32
            };
            if alpha > 0. {
                let height = (bounds.h * bounds.h / (bounds.h + self.scroll.max)).max(24.);
                let y = bounds.y
                    + (bounds.h - height) * (self.scroll.offset / self.scroll.max).clamp(0., 1.);
                scene.rect(
                    Rect::new(
                        bounds.right() - if edge { 8. } else { 2. },
                        y,
                        if edge { 8. } else { 2. },
                        height,
                    ),
                    theme.line_strong.alpha(alpha),
                    self.viewport,
                );
            }
        }
        self.paint_debug(&mut scene, &theme);
        scene
    }
    /// Keep the Metro title axis and give cropped predecessors a pointer target.
    fn pivot_previous_rect(&self, n: &Node) -> Option<Rect> {
        let Kind::Choice {
            options,
            pivot: true,
            ..
        } = &n.w.kind
        else {
            return None;
        };
        let active = self.states.get(&n.w.id)?.model as usize;
        if active == 0 || active >= options.len() {
            return None;
        }
        let size = (if self.touch { 48f32 } else { 40. }).max(40. * self.theme.text_scale);
        // Reserve the touch-sized lane in both modes. Touch-down may enlarge the
        // hit area, but must not relocate a chevron the user has already seen.
        let lane = 48f32.max(48. * self.theme.text_scale);
        let y = n.bounds.y + (44. * self.theme.text_scale - lane) / 2.;
        let gutter = Rect::new(n.bounds.x - lane, y, lane, lane);
        let occupied = self.nodes.iter().any(|other| {
            other.w.id != n.w.id && !matches!(other.w.kind, Kind::Group | Kind::Custom) && {
                let intersection = gutter.intersect(other.bounds);
                intersection.w > 0. && intersection.h > 0.
            }
        });
        let slot = if gutter.x >= self.viewport.x && !occupied {
            gutter
        } else {
            Rect::new((n.bounds.right() - lane).max(n.bounds.x), y, lane, lane)
        };
        let rect = Rect::new(
            slot.center().x - size / 2.,
            slot.center().y - size / 2.,
            size,
            size,
        );
        Some(rect.intersect(self.viewport).intersect(Rect::new(
            self.viewport.x,
            n.clip.y,
            self.viewport.w,
            n.clip.h,
        )))
    }
    fn choice_clip(&self, n: &Node) -> Rect {
        let mut clip = n.clip.intersect(n.bounds);
        if let Some(previous) = self.pivot_previous_rect(n).filter(|r| r.x >= n.bounds.x) {
            let lane = 48f32.max(48. * self.theme.text_scale);
            clip.w =
                (previous.center().x - lane / 2. - 8. * self.theme.text_scale - clip.x).max(0.);
        }
        clip
    }
    fn tooltip_text(&self, n: &Node) -> Option<String> {
        if self
            .pivot_previous_rect(n)
            .is_some_and(|r| r.contains(self.pointer))
            && let Kind::Choice { options, .. } = &n.w.kind
        {
            let active = self.states[&n.w.id].model as usize;
            return options
                .get(active - 1)
                .map(|label| format!("previous: {label}"));
        }
        n.w.tooltip.clone()
    }
    fn choice_spans(&self, n: &Node) -> Vec<Rect> {
        let Kind::Choice { pivot, .. } = &n.w.kind else {
            return vec![];
        };
        let Some(s) = self.states.get(&n.w.id) else {
            return vec![];
        };
        let widths = &s.choice_widths;
        let mut x = n.bounds.x;
        let mut spans: Vec<_> = widths
            .iter()
            .map(|width| {
                let r = Rect::new(x, n.bounds.y, *width, n.bounds.h);
                x += width + 24. * self.theme.text_scale;
                r
            })
            .collect();
        if *pivot && !spans.is_empty() {
            let v = if self.theme.reduce_motion {
                s.model
            } else {
                s.value.sample(self.clock.now()).0
            }
            .clamp(0., (spans.len() - 1) as f64);
            let a = v.floor() as usize;
            let b = v.ceil() as usize;
            let offset = spans[a].x + (spans[b].x - spans[a].x) * v.fract() as f32 - n.bounds.x;
            for r in &mut spans {
                r.x -= offset;
            }
        }
        spans
    }
    fn paint_node(&self, scene: &mut Scene, n: &Node, t: &Theme) {
        let Some(s) = self.states.get(&n.w.id) else {
            return;
        };
        if t.reduce_motion && s.value.active(self.clock.now()) {
            let f = ((self.clock.now() - s.value.start) / 0.1).clamp(0., 1.) as f32;
            let start = scene.draws.len();
            self.paint_node_value(scene, n, t, Some(s.value.from));
            for d in &mut scene.draws[start..] {
                d.transform.opacity *= 1. - f;
            }
            let start = scene.draws.len();
            self.paint_node_value(scene, n, t, Some(s.value.target));
            for d in &mut scene.draws[start..] {
                d.transform.opacity *= f;
            }
        } else {
            self.paint_node_value(scene, n, t, None);
        }
    }
    fn paint_node_value(
        &self,
        scene: &mut Scene,
        n: &Node,
        t: &Theme,
        override_value: Option<f64>,
    ) {
        let Some(s) = self.states.get(&n.w.id) else {
            return;
        };
        let r = n.bounds;
        let clip = n.clip;
        let now = self.clock.now();
        let value = override_value.unwrap_or_else(|| s.value.sample(now).0);
        let press = s.press.sample(now).0 as f32;
        let hover = self.hover == Some(n.w.id);
        let focus = self.focus == Some(n.w.id);
        let disabled = n.w.disabled;
        let fg = if disabled { t.fg3 } else { t.fg };
        let secondary = if disabled { t.fg3 } else { t.fg2 };
        let accent = if disabled { t.fg3 } else { t.accent };
        let outline = if disabled {
            t.line
        } else if hover {
            t.fg
        } else {
            t.line_strong
        };
        let text_scale = t.text_scale;
        let start = scene.draws.len();
        match &n.w.kind {
            Kind::Group | Kind::Custom => {}
            Kind::Label { role } => {
                let text = if matches!(
                    role,
                    TypeRole::Display | TypeRole::Title | TypeRole::Heading
                ) {
                    n.w.label.to_lowercase()
                } else {
                    n.w.label.clone()
                };
                let b = if matches!(role, TypeRole::Display | TypeRole::Title) {
                    r.translate(-0.06 * role.style().size, 0.)
                } else {
                    r
                };
                scene.text(
                    b,
                    text,
                    *role,
                    if matches!(role, TypeRole::Label | TypeRole::Caption) {
                        secondary
                    } else {
                        fg
                    },
                    clip,
                );
            }
            Kind::Toggle { .. } => {
                scene.text(
                    Rect::new(r.x, r.y + 8., r.w - 80., 20. * text_scale),
                    &n.w.label,
                    TypeRole::Body,
                    secondary,
                    clip,
                );
                let elapsed = if override_value.is_some() {
                    1.
                } else {
                    now - s.word_at
                };
                let word = if override_value.is_some() {
                    value >= 0.5
                } else {
                    s.word
                };
                let incoming = if word { "On" } else { "Off" };
                let outgoing = if word { "Off" } else { "On" };
                let word_bounds = Rect::new(r.x, r.y + 32., r.w - 80., 28. * text_scale);
                if (0. ..0.17).contains(&elapsed) {
                    let out = (1. - elapsed / 0.09).clamp(0., 1.) as f32;
                    let inc = ((elapsed - 0.04) / 0.12).clamp(0., 1.) as f32;
                    scene.text(
                        word_bounds
                            .translate(0., if t.reduce_motion { 0. } else { 6. * (1. - out) }),
                        outgoing,
                        TypeRole::Value,
                        fg.alpha(out),
                        clip,
                    );
                    scene.text(
                        word_bounds.translate(
                            0.,
                            if t.reduce_motion {
                                0.
                            } else {
                                -6. * (1. - inc)
                            },
                        ),
                        incoming,
                        TypeRole::Value,
                        fg.alpha(inc),
                        clip,
                    );
                } else {
                    scene.text(word_bounds, incoming, TypeRole::Value, fg, clip);
                }
                let track = toggle_track(r);
                let progress = value.clamp(0., 1.) as f32;
                scene.stroke(
                    track,
                    2.,
                    if disabled {
                        outline
                    } else {
                        outline.mix(accent, progress)
                    },
                    clip,
                );
                if progress > 0. && !disabled {
                    // Both endpoints must be continuous. Using the thumb's center
                    // leaves a 12px cap that pops in/out when the tween starts/ends.
                    // Keep the thumb's travel/timing and wipe the full track extent.
                    scene.rect(
                        Rect::new(track.x, track.y, track.w * progress, track.h),
                        accent,
                        clip,
                    );
                }
                if press > 0. && !disabled {
                    scene.rect(track.inset(2.), t.accent.alpha(0.25 * press), clip);
                }
                let width = 12. + 4. * press;
                let x =
                    track.x + 6. + 28. * progress - if progress > 0.5 { width - 12. } else { 0. };
                scene.rect(
                    Rect::new(x, track.y - 4., width, 28.),
                    if disabled { t.fg3 } else { fg },
                    clip,
                );
            }
            Kind::Slider {
                min, max, bipolar, ..
            } => {
                let p = ((value - min) / (max - min).max(f64::EPSILON)).clamp(0., 1.) as f32;
                let y = r.y + 44.;
                scene.text(
                    Rect::new(r.x, r.y, r.w * 0.65, 20. * text_scale),
                    &n.w.label,
                    TypeRole::Label,
                    secondary,
                    clip,
                );
                scene.text(
                    Rect::new(r.right() - 72., r.y, 72., 20. * text_scale),
                    if s.learn {
                        "…".into()
                    } else {
                        format_value(value)
                    },
                    TypeRole::MonoValue,
                    if press > 0. || s.learn { accent } else { fg },
                    clip,
                );
                scene.rect(Rect::new(r.x, y - 1., r.w, 2.), outline, clip);
                let zero = if *bipolar {
                    ((-min) / (max - min)) as f32
                } else {
                    0.
                };
                let left = r.x + r.w * zero.min(p);
                let width = r.w * (p - zero).abs();
                scene.rect(Rect::new(left, y - 3., width, 6.), accent, clip);
                let thumb = Rect::new(
                    r.x + r.w * p - (12. + 4. * press) / 2.,
                    y - 14.,
                    12. + 4. * press,
                    28.,
                );
                scene.rect(thumb, fg, clip);
                if hover {
                    scene.rect(Rect::new(thumb.x, thumb.y, thumb.w, 2.), accent, clip);
                }
            }
            Kind::Knob { min, max, .. } => {
                let p = ((value - min) / (max - min).max(f64::EPSILON)).clamp(0., 1.) as f32;
                let arc = Rect::new(r.x + 8., r.y + 8., 56., 56.);
                let angle = std::f32::consts::PI * 0.75;
                let sweep = std::f32::consts::PI * 1.5;
                scene.arc(
                    arc,
                    angle,
                    sweep,
                    2.,
                    if hover { t.line_strong } else { t.line },
                    clip,
                );
                scene.arc(arc, angle, sweep * p, 4. + 2. * press, accent, clip);
                let a = angle + sweep * p;
                let center = arc.center();
                scene.push(
                    Primitive::Line {
                        from: Point::new(center.x + a.cos() * 21., center.y + a.sin() * 21.),
                        to: Point::new(center.x + a.cos() * 27., center.y + a.sin() * 27.),
                        width: 2.,
                        color: fg,
                    },
                    clip,
                );
                let text = if s.learn {
                    "…".into()
                } else {
                    format_value(value)
                };
                let width = text.chars().count() as f32 * 8.4 * text_scale;
                scene.text(
                    Rect::new(
                        center.x - width / 2.,
                        center.y - 10. * text_scale,
                        width + 4.,
                        24. * text_scale,
                    ),
                    text,
                    TypeRole::MonoValue,
                    if s.learn { accent } else { fg },
                    clip,
                );
                scene.text(
                    Rect::new(r.x + 8., r.y + 72., r.w - 8., 16. * text_scale),
                    &n.w.label,
                    TypeRole::Label,
                    secondary,
                    clip,
                );
            }
            Kind::Button { style } => {
                match style {
                    ButtonStyle::Primary => scene.rect(
                        r,
                        if disabled {
                            t.line
                        } else if press > 0. {
                            t.accent_press
                        } else if hover {
                            t.accent.mix(Color::hex(0xffffff), 0.1)
                        } else {
                            accent
                        },
                        clip,
                    ),
                    ButtonStyle::Secondary => scene.stroke(r, 2., outline, clip),
                    ButtonStyle::Text => {
                        if hover || focus && self.keyboard_focus {
                            scene.rect(Rect::new(r.x, r.bottom() - 2., r.w, 2.), accent, clip);
                        }
                    }
                }
                let role = if *style == ButtonStyle::Text {
                    TypeRole::Body
                } else {
                    TypeRole::Value
                };
                scene.text(
                    Rect::new(
                        r.x + if *style == ButtonStyle::Text { 0. } else { 16. },
                        r.y + (r.h - role.style().line * text_scale) / 2.,
                        r.w - 24.,
                        role.style().line * text_scale,
                    ),
                    &n.w.label,
                    role,
                    if *style == ButtonStyle::Primary && !disabled {
                        t.on_accent
                    } else {
                        fg
                    },
                    clip,
                );
            }
            Kind::Input {
                multiline, error, ..
            } => {
                let b = input_rect(n);
                if !multiline {
                    scene.text(
                        Rect::new(r.x, r.y, r.w, 16. * text_scale),
                        &n.w.label,
                        TypeRole::Label,
                        secondary,
                        clip,
                    );
                    scene.stroke(
                        b,
                        2.,
                        if error.is_some() {
                            t.danger
                        } else if focus {
                            accent
                        } else {
                            outline
                        },
                        clip,
                    );
                }
                if let Some(e) = &s.editor {
                    let text_bounds =
                        Rect::new(b.x + 12., b.y + 8., (b.w - 24.).max(0.), (b.h - 8.).max(0.));
                    let text_clip = clip.intersect(b.inset(4.));
                    if let Some((a, z)) = e.editor.selection_bounds() {
                        e.editor.with_buffer(|buffer| {
                            for run in buffer.layout_runs() {
                                for (x, width) in run.highlight(a, z) {
                                    scene.rect(
                                        Rect::new(
                                            text_bounds.x + x - e.horizontal_scroll(),
                                            text_bounds.y + run.line_top,
                                            width,
                                            run.line_height,
                                        ),
                                        accent.alpha(0.4),
                                        text_clip,
                                    );
                                }
                            }
                        });
                    }
                    e.editor.with_buffer(|buffer| {
                        scene.push(
                            Primitive::ShapedText {
                                bounds: text_bounds,
                                buffer: std::sync::Arc::new(buffer.clone()),
                                color: fg,
                            },
                            text_clip,
                        )
                    });
                    if focus && self.window_focused {
                        if let Some((x, y)) = e.cursor_position() {
                            let caret = Rect::new(
                                text_bounds.x + x,
                                text_bounds.y + y,
                                2.,
                                20. * text_scale,
                            );
                            if ((now / 0.53) as u64).is_multiple_of(2) {
                                scene.rect(caret, accent, text_clip);
                            }
                            if !e.preedit.is_empty() {
                                let width =
                                    (e.preedit.chars().count() as f32 * 9. * text_scale).max(8.);
                                scene.rect(
                                    Rect::new(caret.x, caret.y, width, 22. * text_scale),
                                    t.bg,
                                    text_clip,
                                );
                                scene.text(
                                    Rect::new(caret.x, caret.y, width, 22. * text_scale),
                                    &e.preedit,
                                    TypeRole::Body,
                                    fg,
                                    text_clip,
                                );
                                scene.rect(
                                    Rect::new(caret.x, caret.y + 20. * text_scale, width, 1.),
                                    accent,
                                    text_clip,
                                );
                            }
                        }
                        if !multiline && !e.value().is_empty() {
                            scene.text(
                                Rect::new(b.right() - 28., b.y + 8., 24., 24.),
                                "×",
                                TypeRole::Value,
                                secondary,
                                clip,
                            );
                        }
                    }
                }
                if let Some(error) = error {
                    scene.text(
                        Rect::new(r.x, b.bottom() + 4., r.w, 16.),
                        error,
                        TypeRole::Caption,
                        t.danger,
                        clip,
                    );
                }
            }
            Kind::ListItem {
                selected,
                secondary: subtitle,
            } => {
                let progress = value.clamp(0., 1.) as f32;
                if progress > 0. {
                    scene.rect(
                        Rect::new(r.x, r.center().y - r.h * progress / 2., 4., r.h * progress),
                        accent,
                        clip,
                    );
                }
                let inset = if *selected { 12. } else { 0. };
                scene.text(
                    Rect::new(r.x + inset, r.y + 8., r.w - inset, 28. * text_scale),
                    &n.w.label,
                    TypeRole::Value,
                    fg.mix(accent, value.clamp(0., 1.) as f32),
                    clip,
                );
                if !subtitle.is_empty() {
                    scene.text(
                        Rect::new(r.x + inset, r.y + 36., r.w - inset, 16. * text_scale),
                        subtitle,
                        TypeRole::Caption,
                        secondary,
                        clip,
                    );
                }
                if self
                    .drag
                    .as_ref()
                    .is_some_and(|d| d.id == n.w.id && d.moved)
                {
                    scene.stroke(r, 1., t.line, clip);
                }
            }
            Kind::Picker { options, .. } => {
                let overlay_owns_row = self.viewport.w >= 720.
                    && (self.popup.as_ref().is_some_and(
                        |p| matches!(&p.kind,Overlay::Picker{owner,..} if *owner==n.w.id),
                    ) || !t.reduce_motion
                        && self.closing_popup.as_ref().is_some_and(
                            |c| matches!(&c.popup.kind,Overlay::Picker{owner,..} if *owner==n.w.id),
                        ));
                // The anchored overlay owns the value during its handoff. Suppress
                // the covered field to avoid ghost labels through a fading surface.
                if overlay_owns_row {
                    return;
                }
                if hover {
                    scene.stroke(r, 2., outline, clip);
                }
                let inset = PICKER_TEXT_INSET * text_scale;
                scene.text(
                    Rect::new(
                        r.x + inset,
                        r.y + 8. * text_scale,
                        (r.w - 2. * inset).max(0.),
                        20. * text_scale,
                    ),
                    &n.w.label,
                    TypeRole::Body,
                    secondary,
                    clip,
                );
                let text = options
                    .get(s.model.max(0.) as usize)
                    .map(String::as_str)
                    .unwrap_or("");
                scene.text(
                    picker_value_rect(r, text_scale),
                    text,
                    TypeRole::Value,
                    fg,
                    clip,
                );
            }
            Kind::Choice { options, pivot, .. } => {
                let spans = self.choice_spans(n);
                let active = s.model.max(0.) as usize;
                for (i, (option, b)) in options.iter().zip(&spans).enumerate() {
                    scene.text(
                        *b,
                        if *pivot {
                            option.to_lowercase()
                        } else {
                            option.clone()
                        },
                        if *pivot {
                            TypeRole::Title
                        } else {
                            TypeRole::Body
                        },
                        if i == active {
                            fg
                        } else if *pivot {
                            t.fg3
                        } else {
                            secondary
                        },
                        self.choice_clip(n),
                    );
                }
                if !pivot && !spans.is_empty() {
                    let position = value.clamp(0., (spans.len() - 1) as f64);
                    let a = position.floor() as usize;
                    let b = position.ceil() as usize;
                    let underline = spans[a].lerp(spans[b], position.fract() as f32);
                    scene.rect(
                        Rect::new(underline.x, r.bottom() - 2., underline.w, 2.),
                        accent,
                        clip,
                    );
                }
                if let Some(previous) = self.pivot_previous_rect(n) {
                    let over = previous.contains(self.pointer);
                    let pressed = self
                        .drag
                        .as_ref()
                        .is_some_and(|d| d.id == n.w.id && previous.contains(d.start));
                    let size = 20. * text_scale;
                    let shift = if pressed && !t.reduce_motion {
                        -4. * press
                    } else {
                        0.
                    };
                    crate::icons::Icon::CaretLeft.paint(
                        scene,
                        Rect::new(
                            previous.center().x - size / 2. + shift,
                            previous.center().y - size / 2.,
                            size,
                            size,
                        ),
                        if disabled {
                            t.fg3
                        } else if pressed {
                            accent
                        } else if over {
                            fg
                        } else {
                            secondary
                        },
                        self.viewport.intersect(Rect::new(
                            self.viewport.x,
                            n.clip.y,
                            self.viewport.w,
                            n.clip.h,
                        )),
                    );
                }
            }
            Kind::Tile {
                value: primary,
                active,
            } => {
                let value = if s.live_secondary {
                    n.w.live_tile.as_ref().map_or(primary, |l| &l.secondary)
                } else {
                    primary
                };
                let pulse = (1. - ((now - s.hit_at) / 0.3)).clamp(0., 1.) as f32;
                if *active {
                    scene.rect(r, accent, clip);
                } else {
                    scene.stroke(r, 1., t.line, clip);
                }
                if !*active && pulse > 0. {
                    scene.rect(r, accent.alpha(pulse), clip);
                }
                let c = if *active {
                    t.on_accent
                } else {
                    fg.mix(t.on_accent, pulse)
                };
                scene.text(
                    Rect::new(r.x + 12., r.y + 8., r.w - 24., 72. * text_scale),
                    value,
                    if r.h >= 160. {
                        TypeRole::PixelDisplay
                    } else {
                        TypeRole::MonoLarge
                    },
                    c,
                    clip,
                );
                scene.text(
                    Rect::new(r.x + 12., r.bottom() - 28., r.w - 24., 20. * text_scale),
                    &n.w.label,
                    TypeRole::Label,
                    if *active { c } else { secondary },
                    clip,
                );
            }
            Kind::Meter { db, vertical } => {
                let level = ((db + 60.) / 60.).clamp(0., 1.);
                let peak = ((s.peak + 60.) / 60.).clamp(0., 1.);
                let b = if *vertical {
                    Rect::new(r.x, r.y, 4., r.h)
                } else {
                    Rect::new(r.x, r.y, r.w, 4.)
                };
                scene.rect(b, t.line, clip);
                let fill = if *vertical {
                    Rect::new(b.x, b.bottom() - b.h * level, b.w, b.h * level)
                } else {
                    Rect::new(b.x, b.y, b.w * level, b.h)
                };
                scene.rect(fill, accent, clip);
                let marker = if *vertical {
                    Rect::new(b.x, b.bottom() - b.h * peak, b.w, 2.)
                } else {
                    Rect::new(b.x + b.w * peak, b.y, 2., b.h)
                };
                scene.rect(marker, fg, clip);
                if *db > 0. {
                    scene.rect(
                        if *vertical {
                            Rect::new(b.x, b.y, b.w, 8.)
                        } else {
                            Rect::new(b.right() - 8., b.y, 8., b.h)
                        },
                        t.danger,
                        clip,
                    );
                }
            }
            Kind::Progress { value: progress } => {
                if progress.is_some() {
                    scene.rect(Rect::new(r.x, r.y + 1., r.w, 2.), t.line, clip);
                    scene.rect(
                        Rect::new(r.x, r.y, r.w * value.clamp(0., 1.) as f32, 4.),
                        accent,
                        clip,
                    );
                } else {
                    for i in 0..5 {
                        let phase = ((now - i as f64 * 0.16).rem_euclid(3.2) / 3.2) as f32;
                        let p = phase * phase * (3. - 2. * phase);
                        scene.rect(Rect::new(r.x + p * r.w, r.y, 4., 4.), accent, clip);
                    }
                }
            }
            Kind::IconButton { icon, expanded } => {
                let icon_start = scene.draws.len();
                let size = if r.w < 40. { 16. } else { 20. };
                icon.paint(
                    scene,
                    Rect::new(r.x + (r.w - size) / 2., r.y + (r.h - size) / 2., size, size),
                    if disabled {
                        t.fg3
                    } else if press > 0. {
                        accent
                    } else if hover {
                        fg
                    } else {
                        secondary
                    },
                    clip,
                );
                if !t.reduce_motion {
                    use crate::icons::Icon;
                    let mut tr = Transform::default();
                    if expanded.is_some() {
                        tr.rotate_z = match icon {
                            Icon::Plus => 45. * value as f32,
                            Icon::CaretUp
                            | Icon::CaretDown
                            | Icon::CaretLeft
                            | Icon::CaretRight => 180. * value as f32,
                            _ => 0.,
                        };
                    } else {
                        match icon {
                            Icon::ArrowUp | Icon::CaretUp => tr.y = -4. * press,
                            Icon::ArrowDown | Icon::CaretDown => tr.y = 4. * press,
                            Icon::ArrowLeft | Icon::CaretLeft => tr.x = -4. * press,
                            Icon::ArrowRight | Icon::CaretRight => tr.x = 4. * press,
                            _ => {}
                        }
                    }
                    scene.transform_since(icon_start, tr, r.center());
                    if matches!(icon, Icon::DotsThree | Icon::DotsThreeVertical) {
                        for draw in &mut scene.draws[icon_start..] {
                            if let Primitive::Triangle { points, .. } = &mut draw.primitive {
                                let horizontal = *icon == Icon::DotsThree;
                                let center = if horizontal {
                                    r.center().x
                                } else {
                                    r.center().y
                                };
                                let mid = points
                                    .iter()
                                    .map(|p| if horizontal { p.x } else { p.y })
                                    .sum::<f32>()
                                    / 3.;
                                let shift = if mid < center - 3. {
                                    -2. * press
                                } else if mid > center + 3. {
                                    2. * press
                                } else {
                                    0.
                                };
                                for p in points {
                                    if horizontal {
                                        p.x += shift
                                    } else {
                                        p.y += shift
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        if (s.learn || now - s.bound_at < 0.36) && !disabled {
            if s.learn && !t.reduce_motion {
                let r = r.inset(-4.);
                let perimeter = 2. * (r.w + r.h);
                let point = |mut d: f32| {
                    d = d.rem_euclid(perimeter);
                    if d < r.w {
                        Point::new(r.x + d, r.y)
                    } else if d < r.w + r.h {
                        Point::new(r.right(), r.y + d - r.w)
                    } else if d < 2. * r.w + r.h {
                        Point::new(r.right() - (d - r.w - r.h), r.bottom())
                    } else {
                        Point::new(r.x, r.bottom() - (d - 2. * r.w - r.h))
                    }
                };
                let phase = (now as f32 * 24.).rem_euclid(10.);
                for i in 0..(perimeter / 10.).ceil() as usize {
                    let a = i as f32 * 10. + phase;
                    let b = (a + 6.).min(perimeter);
                    scene.push(
                        Primitive::Line {
                            from: point(a),
                            to: point(b),
                            width: 2.,
                            color: accent,
                        },
                        clip,
                    );
                }
            } else {
                let alpha = if s.learn {
                    1.
                } else {
                    ((0.36 - (now - s.bound_at)) / 0.12).clamp(0., 1.) as f32
                };
                scene.stroke(r.inset(-4.), 2., accent.alpha(alpha), clip);
            }
        }
        if let Some(cc) = s.cc {
            scene.text(
                Rect::new(r.x, r.bottom() + 4., r.w, 20.),
                format!("CC {cc}"),
                TypeRole::MonoValue,
                secondary,
                clip,
            );
        }
        if t.tilt
            && !t.reduce_motion
            && !disabled
            && matches!(
                n.w.kind,
                Kind::Button { .. } | Kind::Tile { .. } | Kind::ListItem { .. }
            )
        {
            let mut transform = crate::animation::tilt(self.pointer, r, press);
            if self
                .drag
                .as_ref()
                .is_some_and(|d| d.id == n.w.id && d.moved)
                && matches!(n.w.kind, Kind::ListItem { .. })
            {
                transform.scale = 1.02;
            }
            scene.transform_since(start, transform, r.center());
        }
        if matches!(n.w.kind, Kind::Tile { .. }) {
            let index = self
                .nodes
                .iter()
                .filter(|node| matches!(node.w.kind, Kind::Tile { .. }))
                .position(|node| node.w.id == n.w.id)
                .unwrap_or(0);
            let elapsed = now - s.appeared;
            let f = if t.reduce_motion {
                (elapsed / 0.1).clamp(0., 1.) as f32
            } else {
                Curve::OutExpo.sample(((elapsed - index as f64 * 0.03) / 0.4).clamp(0., 1.)) as f32
            };
            let live = now - s.live_at;
            for draw in &mut scene.draws[start..] {
                draw.transform.opacity *= f;
                if !t.reduce_motion {
                    draw.transform.rotate_x += -60. * (1. - f);
                    draw.transform.y += 20. * (1. - f);
                }
                if live >= 0. && live < if t.reduce_motion { 0.1 } else { 0.4 } {
                    if t.reduce_motion {
                        draw.transform.opacity *= ((live - 0.05).abs() / 0.05) as f32;
                    } else {
                        draw.transform.rotate_x += if live < 0.2 {
                            -90. * Curve::InExpo.sample(live / 0.2) as f32
                        } else {
                            90. * (1. - Curve::OutExpo.sample((live - 0.2) / 0.2) as f32)
                        };
                    }
                }
            }
        }
        if let Some((from, tween)) = s.reorder.filter(|(_, tween)| tween.active(now)) {
            let f = tween.sample(now).0 as f32;
            for draw in &mut scene.draws[start..] {
                if t.reduce_motion {
                    draw.transform.opacity *= f;
                } else {
                    draw.transform.x += (from.x - r.x) * (1. - f);
                    draw.transform.y += (from.y - r.y) * (1. - f);
                }
            }
        }
    }
    fn picker_reveal_rect(&self, p: &Popup, now: f64) -> Rect {
        let (bounds, rows) = self.popup_geometry(p);
        let Overlay::Picker { selected, .. } = &p.kind else {
            return bounds;
        };
        if self.theme.reduce_motion {
            return bounds;
        }
        let from = rows
            .get(*selected)
            .copied()
            .unwrap_or(bounds)
            .inset(-PICKER_MENU_PADDING * self.theme.text_scale)
            .intersect(bounds);
        let f = Curve::OutExpo.sample(((now - p.at) / 0.22).clamp(0., 1.)) as f32;
        from.lerp(bounds, f)
    }
    fn paint_desktop_picker(
        &self,
        scene: &mut Scene,
        p: &Popup,
        t: &Theme,
        closing: Option<&ClosingPopup>,
    ) {
        let Overlay::Picker {
            owner,
            bounds: field,
            options,
            selected,
            ..
        } = &p.kind
        else {
            return;
        };
        let now = self.clock.now();
        let (bounds, rows) =
            closing.map_or_else(|| self.popup_geometry(p), |c| (c.bounds, c.rows.clone()));
        let selected = closing.map_or(*selected, |c| c.selected);
        let closed_value =
            picker_value_rect(self.node(*owner).map_or(*field, |n| n.bounds), t.text_scale);
        let row_text = |row: Rect| {
            Rect::new(
                row.x + PICKER_TEXT_INSET * t.text_scale,
                row.y + (row.h - 24. * t.text_scale) / 2.,
                (row.w - 2. * PICKER_TEXT_INSET * t.text_scale).max(0.),
                24. * t.text_scale,
            )
        };
        let source = rows
            .get(selected)
            .copied()
            .map(row_text)
            .unwrap_or(closed_value);
        let open_alpha = Curve::OutExpo.sample(((now - p.at) / 0.12).clamp(0., 1.)) as f32;
        let (panel, alpha, dim_alpha, progress) = if let Some(c) = closing {
            let time = ((now - c.at) / c.duration).clamp(0., 1.);
            let progress = if t.reduce_motion {
                0.
            } else {
                Curve::OutExpo.sample(time) as f32
            };
            let row_height = PICKER_ROW_HEIGHT * t.text_scale;
            let padding = PICKER_MENU_PADDING * t.text_scale;
            let target = Rect::new(
                closed_value.x - PICKER_TEXT_INSET * t.text_scale,
                closed_value.y - (row_height - closed_value.h) / 2. - padding,
                closed_value.w + 2. * PICKER_TEXT_INSET * t.text_scale,
                row_height + 2. * padding,
            );
            (
                if t.reduce_motion {
                    bounds
                } else {
                    c.reveal.lerp(target, progress)
                },
                c.opacity * (1. - Curve::InExpo.sample(time) as f32),
                c.dim_opacity * (1. - Curve::OutExpo.sample(time) as f32),
                progress,
            )
        } else {
            (
                self.picker_reveal_rect(p, now),
                open_alpha,
                ((now - p.at) / 0.16).clamp(0., 1.) as f32,
                0.,
            )
        };
        scene.rect(
            self.viewport,
            Color::hex(0).alpha(if t.light { 0.4 } else { 0.6 } * dim_alpha),
            self.viewport,
        );
        scene.rect(panel, t.surface.alpha(alpha), self.viewport);
        scene.stroke(panel, 1., t.line.alpha(alpha), self.viewport);
        for (i, (text, row)) in options.iter().zip(&rows).enumerate() {
            if closing.is_some() && i == selected && !t.reduce_motion {
                continue;
            }
            let value = row_text(*row).translate(
                (closed_value.x - source.x) * progress,
                (closed_value.y - source.y) * progress,
            );
            scene.text(
                value,
                text,
                TypeRole::Value,
                if i == selected && closing.is_none() {
                    t.fg.mix(t.accent, alpha)
                } else {
                    if i == selected { t.accent } else { t.fg }.alpha(alpha)
                },
                panel,
            );
        }
        if closing.is_none() && self.keyboard_focus && !rows.is_empty() {
            let position = p.candidate.sample(now).0.clamp(0., (rows.len() - 1) as f64);
            let focus = rows[position.floor() as usize]
                .lerp(rows[position.ceil() as usize], position.fract() as f32)
                .inset(4. * t.text_scale);
            scene.stroke(focus, 2., t.accent.alpha(alpha), panel);
        }
        if let Some(c) = closing.filter(|_| !t.reduce_motion)
            && let Some(text) = options.get(selected)
        {
            // Carry the chosen text into the closed value. The underlying copy is
            // suppressed until this lands, avoiding double labels and a final pop.
            let color = t.fg.mix(t.accent, c.opacity).mix(t.fg, progress);
            scene.text(
                source.lerp(closed_value, progress),
                text,
                TypeRole::Value,
                color,
                self.viewport,
            );
        }
    }
    fn paint_popup(&self, scene: &mut Scene, p: &Popup, t: &Theme) {
        if matches!(p.kind, Overlay::Picker { .. }) && self.viewport.w >= 720. {
            self.paint_desktop_picker(scene, p, t, None);
            return;
        }

        let age = self.clock.now() - p.at;
        let (b, rows) = self.popup_geometry(p);
        let is_menu = matches!(p.kind, Overlay::Menu { .. });
        let opacity = if is_menu {
            1.
        } else {
            (age / 0.16).clamp(0., 1.) as f32
        };
        if !is_menu {
            scene.rect(
                self.viewport,
                Color::hex(0).alpha(if t.light { 0.4 } else { 0.6 } * opacity),
                self.viewport,
            );
        }
        let start = scene.draws.len();
        scene.rect(b, t.surface, self.viewport);
        scene.stroke(b, 1., t.line, self.viewport);
        match &p.kind {
            Overlay::CommandPalette { query, commands } => {
                let left = Grid::new(self.viewport.w, self.viewport.h, t.text_scale).margin;
                scene.text(
                    Rect::new(left, 8., self.viewport.w - 2. * left, 16.),
                    "command",
                    TypeRole::Label,
                    t.fg2,
                    self.viewport,
                );
                let input = Rect::new(left, 32., self.viewport.w - left * 2., 40.);
                scene.stroke(input, 2., t.accent, self.viewport);
                scene.text(input.inset(8.), query, TypeRole::Body, t.fg, self.viewport);
                let labels = palette_matches(query, commands);
                for (i, (label, r)) in labels.iter().zip(rows).enumerate() {
                    scene.text(
                        r.inset(8.),
                        label,
                        TypeRole::Heading,
                        if i == p.selected { t.accent } else { t.fg },
                        b,
                    );
                }
            }
            Overlay::Dialog {
                title,
                message,
                actions,
            } => {
                let x = Grid::new(self.viewport.w, self.viewport.h, t.text_scale).margin;
                scene.text(
                    Rect::new(x, b.y + 24., b.w - x * 2., 32.),
                    title.to_lowercase(),
                    TypeRole::Heading,
                    t.fg,
                    self.viewport,
                );
                scene.text(
                    Rect::new(x, b.y + 72., b.w - x * 2., 48.),
                    message,
                    TypeRole::Body,
                    t.fg,
                    self.viewport,
                );
                for (i, (text, r)) in actions.iter().zip(rows).enumerate() {
                    scene.text(
                        r.inset(8.),
                        text,
                        TypeRole::Body,
                        if p.selected == i { t.accent } else { t.fg },
                        self.viewport,
                    );
                    if p.selected == i && self.keyboard_focus {
                        scene.stroke(r, 2., t.accent, self.viewport);
                    }
                }
            }
            Overlay::Menu { items, .. } => {
                for (i, (text, r)) in items.iter().zip(rows).enumerate() {
                    scene.text(
                        Rect::new(r.x + 12., r.y + 8., r.w - 24., 20.),
                        text,
                        TypeRole::Body,
                        if p.selected == i { t.accent } else { t.fg },
                        self.viewport,
                    );
                }
            }
            Overlay::Picker {
                options, selected, ..
            } => {
                for (i, (text, r)) in options.iter().zip(rows).enumerate() {
                    let from = scene.draws.len();
                    let compact = self.viewport.w < 720.;
                    let text_bounds = if compact {
                        r.inset(8. * t.text_scale)
                    } else {
                        Rect::new(
                            r.x + PICKER_TEXT_INSET * t.text_scale,
                            r.y + (r.h - 24. * t.text_scale) / 2.,
                            (r.w - 2. * PICKER_TEXT_INSET * t.text_scale).max(0.),
                            24. * t.text_scale,
                        )
                    };
                    scene.text(
                        text_bounds,
                        text,
                        if compact {
                            TypeRole::Heading
                        } else {
                            TypeRole::Value
                        },
                        if *selected == i { t.accent } else { t.fg },
                        b,
                    );
                    if self.keyboard_focus && p.selected == i {
                        scene.stroke(r.inset(4. * t.text_scale), 2., t.accent, b);
                    }
                    if self.viewport.w < 720. && !t.reduce_motion {
                        let f = Curve::OutExpo
                            .sample(((age - i as f64 * 0.03) / 0.22).clamp(0., 1.))
                            as f32;
                        scene.transform_since(
                            from,
                            Transform {
                                x: -40. * (1. - f),
                                opacity: f,
                                ..Default::default()
                            },
                            r.center(),
                        );
                    }
                }
            }
        }
        if !is_menu {
            let f = Curve::OutExpo
                .sample((age / if t.reduce_motion { 0.1 } else { 0.22 }).clamp(0., 1.))
                as f32;
            let transform = Transform {
                y: if t.reduce_motion
                    || matches!(p.kind, Overlay::Picker { .. }) && self.viewport.w >= 720.
                {
                    0.
                } else {
                    24. * (1. - f)
                },
                opacity: f,
                ..Default::default()
            };
            for draw in &mut scene.draws[start..] {
                draw.transform.y += transform.y;
                draw.transform.opacity *= transform.opacity;
            }
        }
    }
    fn paint_debug(&self, scene: &mut Scene, t: &Theme) {
        let b = self.viewport;
        if self.debug.grid {
            let g = Grid::new(b.w, b.h, t.text_scale);
            for i in 0..12 {
                let c = g.span(i, 1);
                scene.stroke(c, 1., t.accent.alpha(0.2), b);
            }
        }
        if self.debug.baselines {
            for y in (0..b.h as usize).step_by(4) {
                scene.rect(Rect::new(0., y as f32, b.w, 1.), t.accent.alpha(0.1), b);
            }
        }
        if self.debug.hit_areas {
            for n in &self.nodes {
                if n.w.interactive() {
                    scene.stroke(n.bounds, 1., t.accent.alpha(0.5), b);
                }
            }
        }
        if self.debug.timings {
            scene.rect(Rect::new(b.w - 240., 0., 240., 32.), t.surface, b);
            scene.text(
                Rect::new(b.w - 232., 8., 224., 20.),
                format!("{:.2} ms  {} nodes", self.frame_ms, self.nodes.len()),
                TypeRole::MonoValue,
                t.fg,
                b,
            );
        }
    }
    pub fn accessibility_tree(&self, scale: f64) -> accesskit::TreeUpdate {
        use accesskit::{Action, Node as ANode, NodeId, Role, Toggled, Tree, TreeId, TreeUpdate};
        let mut result = vec![];
        let mut children = vec![];
        if let Some(p) = &self.popup {
            let (_, rows) = self.popup_geometry(p);
            let labels = match &p.kind {
                Overlay::CommandPalette { query, commands } => palette_matches(query, commands),
                Overlay::Dialog { actions, .. } => actions.clone(),
                Overlay::Menu { items, .. } => items.clone(),
                Overlay::Picker { options, .. } => options.clone(),
            };
            for (i, (r, label)) in rows.into_iter().zip(labels).enumerate() {
                let id = NodeId(u64::MAX - i as u64);
                let mut n = ANode::new(Role::Button);
                n.set_label(label.clone());
                n.add_action(Action::Focus);
                n.add_action(Action::Click);
                n.set_bounds(accesskit::Rect::new(
                    r.x as f64 * scale,
                    r.y as f64 * scale,
                    r.right() as f64 * scale,
                    r.bottom() as f64 * scale,
                ));
                children.push(id);
                result.push((id, n));
            }
        } else {
            for item in &self.nodes {
                let w = &item.w;
                let s = &self.states[&w.id];
                if matches!(w.kind, Kind::Group | Kind::Custom) {
                    continue;
                }
                let role = match w.kind {
                    Kind::Toggle { .. } => Role::Switch,
                    Kind::Slider { .. } | Kind::Knob { .. } => Role::Slider,
                    Kind::Input {
                        multiline: true, ..
                    } => Role::MultilineTextInput,
                    Kind::Input { .. } => Role::TextInput,
                    Kind::Picker { .. } => Role::ComboBox,
                    Kind::Choice { .. } => Role::TabList,
                    Kind::ListItem { .. } => Role::ListBoxOption,
                    Kind::Meter { .. } => Role::Meter,
                    Kind::Progress { .. } => Role::ProgressIndicator,
                    Kind::Label { .. } => Role::Label,
                    _ => Role::Button,
                };
                let mut n = ANode::new(role);
                n.set_label(w.label.clone());
                let r = item.bounds;
                n.set_bounds(accesskit::Rect::new(
                    r.x as f64 * scale,
                    r.y as f64 * scale,
                    r.right() as f64 * scale,
                    r.bottom() as f64 * scale,
                ));
                if w.disabled {
                    n.set_disabled();
                } else if w.interactive() {
                    n.add_action(Action::Focus);
                    n.add_action(Action::Click);
                }
                if let Some((min, max, step, _)) = w.range() {
                    n.set_numeric_value(s.model);
                    n.set_min_numeric_value(min);
                    n.set_max_numeric_value(max);
                    n.set_numeric_value_step(step);
                    n.add_action(Action::Increment);
                    n.add_action(Action::Decrement);
                    n.add_action(Action::SetValue);
                }
                match &w.kind {
                    Kind::Toggle { .. } => n.set_toggled(if s.model >= 0.5 {
                        Toggled::True
                    } else {
                        Toggled::False
                    }),
                    Kind::Input { .. } => {
                        if let Some(e) = &s.editor {
                            n.set_value(e.value());
                            n.add_action(Action::SetValue);
                        }
                    }
                    Kind::Picker { options, .. } => {
                        n.set_value(options.get(s.model as usize).cloned().unwrap_or_default());
                    }
                    Kind::Choice { options, .. } => {
                        let mut tabs = vec![];
                        for (i, (label, r)) in
                            options.iter().zip(self.choice_spans(item)).enumerate()
                        {
                            let id = choice_accessibility_id(w.id, i);
                            let mut tab = ANode::new(Role::Tab);
                            tab.set_label(label.clone());
                            tab.set_selected(i == s.model as usize);
                            tab.set_bounds(accesskit::Rect::new(
                                r.x as f64 * scale,
                                r.y as f64 * scale,
                                r.right() as f64 * scale,
                                r.bottom() as f64 * scale,
                            ));
                            if w.disabled {
                                tab.set_disabled();
                            } else {
                                tab.add_action(Action::Focus);
                                tab.add_action(Action::Click);
                            }
                            tabs.push(id);
                            result.push((id, tab));
                        }
                        if let Some(r) = self.pivot_previous_rect(item) {
                            let id = pivot_previous_accessibility_id(w.id);
                            let mut previous = ANode::new(Role::Button);
                            previous.set_label(format!(
                                "previous: {}",
                                options[(s.model as usize) - 1]
                            ));
                            previous.set_bounds(accesskit::Rect::new(
                                r.x as f64 * scale,
                                r.y as f64 * scale,
                                r.right() as f64 * scale,
                                r.bottom() as f64 * scale,
                            ));
                            if w.disabled {
                                previous.set_disabled();
                            } else {
                                previous.add_action(Action::Click);
                            }
                            tabs.push(id);
                            result.push((id, previous));
                        }
                        n.set_children(tabs);
                    }
                    Kind::ListItem { selected, .. } => n.set_selected(*selected),
                    _ => {}
                }
                let id = NodeId(w.id.0);
                children.push(id);
                result.push((id, n));
            }
        }
        let mut root = ANode::new(Role::Window);
        root.set_label("Brockmann");
        root.set_children(children.clone());
        result.push((NodeId(0), root));
        let focus = if let Some(p) = &self.popup {
            children.get(p.selected).copied().unwrap_or(NodeId(0))
        } else {
            self.focus.map(|id| NodeId(id.0)).unwrap_or(NodeId(0))
        };
        TreeUpdate {
            nodes: result,
            tree: Some(Tree::new(NodeId(0))),
            tree_id: TreeId::ROOT,
            focus,
        }
    }
    pub fn accessibility_action(&mut self, request: accesskit::ActionRequest) {
        use accesskit::{Action, ActionData};
        if self.popup.is_some() {
            let i = (u64::MAX - request.target_node.0) as usize;
            let count = self
                .popup
                .as_ref()
                .map(|p| self.popup_geometry(p).1.len())
                .unwrap_or(0);
            if i < count {
                self.popup.as_mut().unwrap().selected = i;
                if request.action == Action::Click {
                    self.popup_input(Input::Key {
                        key: Key::Enter,
                        modifiers: Modifiers::default(),
                    });
                }
            }
            return;
        }
        let previous = self
            .nodes
            .iter()
            .filter(|n| n.w.interactive())
            .find(|n| {
                self.pivot_previous_rect(n).is_some()
                    && pivot_previous_accessibility_id(n.w.id) == request.target_node
            })
            .map(|n| n.w.id);
        if let Some(owner) = previous {
            if request.action == Action::Click {
                let index = (self.states[&owner].model as usize).saturating_sub(1);
                self.change(owner, Value::Selection(index), true);
            }
            return;
        }
        let choice = self
            .nodes
            .iter()
            .filter(|n| n.w.interactive())
            .find_map(|n| {
                let Kind::Choice { options, .. } = &n.w.kind else {
                    return None;
                };
                (0..options.len())
                    .find(|i| choice_accessibility_id(n.w.id, *i) == request.target_node)
                    .map(|i| (n.w.id, i))
            });
        if let Some((owner, index)) = choice {
            self.focus_widget(owner);
            if request.action == Action::Click {
                self.change(owner, Value::Selection(index), true);
            }
            return;
        }
        let id = Id(request.target_node.0);
        if !self.node(id).is_some_and(|n| n.w.interactive()) {
            return;
        }
        match request.action {
            Action::Focus => self.focus_widget(id),
            Action::Click => self.activate(id),
            Action::Increment | Action::Decrement => {
                if let Some((min, max, step, _)) = self.node(id).and_then(|n| n.w.range()) {
                    let direction = if request.action == Action::Increment {
                        1.
                    } else {
                        -1.
                    };
                    self.change(
                        id,
                        Value::Number((self.states[&id].model + step * direction).clamp(min, max)),
                        false,
                    );
                }
            }
            Action::SetValue => match request.data {
                Some(ActionData::NumericValue(v)) => self.accessibility_value(id, v),
                Some(ActionData::Value(v)) => {
                    if let Some(e) = self.states.get_mut(&id).and_then(|s| s.editor.as_mut()) {
                        e.select_all(&mut self.fonts);
                        e.insert(&mut self.fonts, &v, self.clock.now());
                        self.events.push(Event::Changed(id, Value::Text(e.value())));
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }
}
fn toggle_track(r: Rect) -> Rect {
    Rect::new(r.right() - 56., r.center().y - 10., 52., 20.)
}
fn input_rect(n: &Node) -> Rect {
    if matches!(
        n.w.kind,
        Kind::Input {
            multiline: true,
            ..
        }
    ) {
        n.bounds
    } else {
        Rect::new(n.bounds.x, n.bounds.y + 24., n.bounds.w, 40.)
    }
}
fn format_value(v: f64) -> String {
    if v.abs() >= 100. || v.fract().abs() < 0.005 {
        format!("{v:.0}")
    } else {
        format!("{v:.1}")
    }
}
fn hit_bounds(r: Rect, min: f32) -> Rect {
    let w = r.w.max(min);
    let h = r.h.max(min);
    Rect::new(r.center().x - w / 2., r.center().y - h / 2., w, h)
}

fn choice_accessibility_id(owner: Id, index: usize) -> accesskit::NodeId {
    accesskit::NodeId(Id::from(format!("brockmann-option-{}-{index}", owner.0).as_str()).0)
}

fn palette_matches(query: &str, commands: &[String]) -> Vec<String> {
    let query = query.to_lowercase();
    commands
        .iter()
        .filter(|c| c.to_lowercase().contains(&query))
        .cloned()
        .collect()
}

// Shared closed/open picker geometry keeps value text at the same position.
const PICKER_TEXT_INSET: f32 = 16.;
const PICKER_ROW_HEIGHT: f32 = 48.;
const PICKER_MENU_PADDING: f32 = 8.;
fn picker_value_rect(bounds: Rect, scale: f32) -> Rect {
    let inset = PICKER_TEXT_INSET * scale;
    Rect::new(
        bounds.x + inset,
        bounds.y + 32. * scale,
        (bounds.w - 2. * inset).max(0.),
        24. * scale,
    )
}

fn pivot_previous_accessibility_id(owner: Id) -> accesskit::NodeId {
    accesskit::NodeId(Id::from(format!("brockmann-pivot-previous-{}", owner.0).as_str()).0)
}
