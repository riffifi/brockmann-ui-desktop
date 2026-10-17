//! Keyed view descriptions. The tree is diffed by stable application-chosen IDs.
use crate::{
    Rect,
    icons::Icon,
    layout::{Layout, Size},
    theme::TypeRole,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Id(pub u64);
impl From<&str> for Id {
    fn from(s: &str) -> Self {
        Self(s.bytes().fold(0xcbf29ce484222325, |a, b| {
            (a ^ b as u64).wrapping_mul(0x100000001b3)
        }))
    }
}
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum ButtonStyle {
    Primary,
    Secondary,
    Text,
}
#[derive(Clone, Debug)]
pub enum Kind {
    Group,
    Label {
        role: TypeRole,
    },
    Toggle {
        on: bool,
    },
    Slider {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
        bipolar: bool,
    },
    Knob {
        value: f64,
        min: f64,
        max: f64,
        step: f64,
        default: f64,
    },
    Button {
        style: ButtonStyle,
    },
    Input {
        value: String,
        multiline: bool,
        error: Option<String>,
    },
    ListItem {
        selected: bool,
        secondary: String,
    },
    Picker {
        options: Vec<String>,
        selected: usize,
    },
    Choice {
        options: Vec<String>,
        selected: usize,
        pivot: bool,
    },
    Tile {
        value: String,
        active: bool,
    },
    Meter {
        db: f32,
        vertical: bool,
    },
    Progress {
        value: Option<f32>,
    },
    IconButton {
        icon: Icon,
        expanded: Option<bool>,
    },
    ColorPicker {
        picker: crate::ColorPicker,
        selected: usize,
    },
    Custom,
}
#[derive(Clone, Debug)]
pub struct LiveTile {
    pub secondary: String,
    pub interval: f64,
}
#[derive(Clone, Debug)]
pub struct Widget {
    pub id: Id,
    pub label: String,
    pub kind: Kind,
    pub layout: Layout,
    pub size: Size,
    pub bounds: Option<Rect>,
    pub disabled: bool,
    /// One vertical scroll viewport may be mounted in a Ui.
    pub scrollable: bool,
    pub live_tile: Option<LiveTile>,
    pub tooltip: Option<String>,
    pub children: Vec<Widget>,
}
impl Widget {
    pub fn new(id: impl Into<Id>, label: impl Into<String>, kind: Kind) -> Self {
        let label = label.into();
        let tooltip = matches!(kind, Kind::IconButton { .. }).then(|| label.clone());
        Self {
            id: id.into(),
            label,
            kind,
            layout: Layout::Leaf,
            size: Size::default(),
            bounds: None,
            disabled: false,
            scrollable: false,
            live_tile: None,
            tooltip,
            children: vec![],
        }
    }
    pub fn label(id: impl Into<Id>, text: impl Into<String>, role: TypeRole) -> Self {
        Self::new(id, text, Kind::Label { role }).height(role.style().line)
    }
    pub fn column(id: impl Into<Id>, children: Vec<Self>) -> Self {
        let mut w = Self::new(id, "", Kind::Group);
        w.layout = Layout::Vertical {
            gap: 8.,
            padding: 0.,
        };
        w.children = children;
        w
    }
    pub fn row(id: impl Into<Id>, children: Vec<Self>) -> Self {
        let mut w = Self::column(id, children);
        w.layout = Layout::Horizontal {
            gap: 24.,
            padding: 0.,
        };
        w
    }
    /// A persistent boolean row with a right-aligned rectangular switch.
    ///
    /// Handle `Event::Changed(id, Value::Bool(on))` in the application and pass the
    /// new boolean back on the next view. The 180 ms visual transition follows
    /// that immediate change; it must not delay backend updates. Labels should
    /// describe positive settings, such as "Audio engine", rather than commands.
    pub fn toggle(id: impl Into<Id>, label: impl Into<String>, on: bool) -> Self {
        Self::new(id, label, Kind::Toggle { on }).height(72.)
    }
    pub fn button(id: impl Into<Id>, label: impl Into<String>, style: ButtonStyle) -> Self {
        let label = label.into();
        let width = crate::layout::rhythm(
            (label.chars().count() as f32 * if style == ButtonStyle::Text { 7.5 } else { 10. }
                + if style == ButtonStyle::Text { 0. } else { 40. })
            .max(40.),
        );
        Self::new(id, label, Kind::Button { style }).width(width)
    }
    pub fn slider(
        id: impl Into<Id>,
        label: impl Into<String>,
        value: f64,
        min: f64,
        max: f64,
        default: f64,
    ) -> Self {
        Self::new(
            id,
            label,
            Kind::Slider {
                value,
                min,
                max,
                default,
                step: (max - min) / 100.,
                bipolar: min < 0. && max > 0.,
            },
        )
        .height(64.)
    }
    pub fn knob(
        id: impl Into<Id>,
        label: impl Into<String>,
        value: f64,
        min: f64,
        max: f64,
        default: f64,
    ) -> Self {
        Self::new(
            id,
            label,
            Kind::Knob {
                value,
                min,
                max,
                default,
                step: (max - min) / 100.,
            },
        )
        .height(88.)
        .width(96.)
    }
    pub fn icon_button(id: impl Into<Id>, label: impl Into<String>, icon: Icon) -> Self {
        Self::new(
            id,
            label,
            Kind::IconButton {
                icon,
                expanded: None,
            },
        )
        .width(40.)
    }
    pub fn expanded(mut self, open: bool) -> Self {
        if let Kind::IconButton { expanded, .. } = &mut self.kind {
            *expanded = Some(open);
        }
        self
    }
    pub fn input(id: impl Into<Id>, label: impl Into<String>, value: impl Into<String>) -> Self {
        Self::new(
            id,
            label,
            Kind::Input {
                value: value.into(),
                multiline: false,
                error: None,
            },
        )
        .height(64.)
    }
    /// A named single selection. `selected` is a zero-based index into `options`.
    ///
    /// Desktop menus open over the value, with the selected option aligned to its
    /// closed text when window bounds permit. The label/value/menu share 16 px
    /// horizontal padding. Below 720 logical px the control uses a full-page list.
    /// Opening does not change the model: handle `Value::Selection` on commit.
    /// Keep the option ordering stable while the menu is open.
    pub fn picker(
        id: impl Into<Id>,
        label: impl Into<String>,
        options: Vec<String>,
        selected: usize,
    ) -> Self {
        Self::new(id, label, Kind::Picker { options, selected }).height(72.)
    }
    pub fn choice(id: impl Into<Id>, options: Vec<String>, selected: usize, pivot: bool) -> Self {
        Self::new(
            id,
            "",
            Kind::Choice {
                options,
                selected,
                pivot,
            },
        )
        .height(if pivot { 56. } else { 40. })
    }
    /// A ribbon or swatch palette. Changes emit `Value::Selection` immediately;
    /// the paired caret markers follow with an interruptible 180 ms transition.
    pub fn color_picker(
        id: impl Into<Id>,
        label: impl Into<String>,
        picker: crate::ColorPicker,
        selected: usize,
    ) -> Self {
        let selected = selected.min(picker.schemes.len().saturating_sub(1));
        let vertical = picker.orientation == crate::Orientation::Vertical;
        let empty = picker.schemes.is_empty();
        Self::new(id, label, Kind::ColorPicker { picker, selected })
            .height(if vertical { 360. } else { 96. })
            .disabled(empty)
    }
    pub fn at(mut self, bounds: Rect) -> Self {
        self.bounds = Some(bounds);
        self
    }
    pub fn height(mut self, h: f32) -> Self {
        self.size.height = h;
        self
    }
    pub fn width(mut self, w: f32) -> Self {
        self.size.width = Some(w);
        self
    }
    pub fn grow(mut self) -> Self {
        self.size.grow = 1.;
        self
    }
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }
    pub fn live_tile(mut self, secondary: impl Into<String>, interval: f64) -> Self {
        self.live_tile = Some(LiveTile {
            secondary: secondary.into(),
            interval: interval.max(0.5),
        });
        self
    }
    pub fn scrollable(mut self) -> Self {
        self.scrollable = true;
        self
    }
    pub fn tooltip(mut self, text: impl Into<String>) -> Self {
        self.tooltip = Some(text.into());
        self
    }
    pub fn gap(mut self, gap: f32) -> Self {
        match &mut self.layout {
            Layout::Horizontal { gap: g, .. } | Layout::Vertical { gap: g, .. } => *g = gap,
            _ => {}
        }
        self
    }
    pub fn interactive(&self) -> bool {
        !self.disabled
            && !matches!(
                self.kind,
                Kind::Group
                    | Kind::Label { .. }
                    | Kind::Meter { .. }
                    | Kind::Progress { .. }
                    | Kind::Custom
            )
    }
    pub fn range(&self) -> Option<(f64, f64, f64, f64)> {
        match self.kind {
            Kind::Slider {
                min,
                max,
                step,
                default,
                ..
            }
            | Kind::Knob {
                min,
                max,
                step,
                default,
                ..
            } => Some((min, max, step, default)),
            _ => None,
        }
    }
    pub fn value(&self) -> f64 {
        match self.kind {
            Kind::IconButton { expanded, .. } => u8::from(expanded.unwrap_or(false)) as f64,
            Kind::Toggle { on } => u8::from(on) as f64,
            Kind::ListItem { selected, .. } => u8::from(selected) as f64,
            Kind::Progress { value: Some(value) } => value.clamp(0., 1.) as f64,
            Kind::Slider { value, .. } | Kind::Knob { value, .. } => value,
            Kind::Choice { selected, .. }
            | Kind::Picker { selected, .. }
            | Kind::ColorPicker { selected, .. } => selected as f64,
            _ => 0.,
        }
    }
}
