//! Interactive component catalogue. Run `cargo run --example showcase`.
mod support;
use brockmann::{
    animation::{Curve, Tween},
    color_picker::hex,
    icons::Icon,
    layout::{Grid, Layout},
    scene::{Primitive, Scene},
    theme::{Color, TypeRole},
    ui::Value,
    widget::{ButtonStyle, Kind},
    *,
};
const PAGES: [&str; 7] = [
    "colour",
    "controls",
    "typography",
    "tiles",
    "overlays",
    "motion",
    "layout",
];
fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}
fn heading(id: &str, text: &str) -> Widget {
    Widget::label(id, text, TypeRole::Heading).height(40.)
}
fn note(id: &str, text: &str) -> Widget {
    Widget::label(id, text, TypeRole::Body).height(32.)
}
fn button(id: &str, text: &str) -> Widget {
    Widget::button(id, text, ButtonStyle::Secondary)
}
struct Showcase {
    page: usize,
    light: bool,
    reduced: bool,
    selected: usize,
    companions: usize,
    vertical: bool,
    swatches: bool,
    toggled: bool,
    amount: f64,
    knob: f64,
    choice: usize,
    pivot: usize,
    custom: usize,
    notes: String,
    invalid: String,
    output: usize,
    text: String,
    weight: Tween,
    weight_target: f64,
    running: bool,
    item: usize,
    order: Vec<usize>,
    status: String,
}
impl Default for Showcase {
    fn default() -> Self {
        Self {
            page: 0,
            light: false,
            reduced: false,
            selected: 4,
            companions: 2,
            vertical: false,
            swatches: false,
            toggled: true,
            amount: 64.,
            knob: 0.25,
            choice: 0,
            pivot: 0,
            custom: 0,
            notes: "A place for longer notes.\nSelect, copy and edit this text.".into(),
            invalid: "fast".into(),
            output: 0,
            text: "Brockmann studio".into(),
            weight: Tween::new(300., 300., 0., 0., Curve::Linear),
            weight_target: 300.,
            running: false,
            item: 0,
            order: vec![0, 1, 2],
            status: "Ready / every control is interactive".into(),
        }
    }
}
impl Showcase {
    fn count(&self) -> CompanionCount {
        [
            CompanionCount::None,
            CompanionCount::One,
            CompanionCount::Two,
            CompanionCount::Three,
        ][self.companions]
    }
    fn picker(&self) -> ColorPicker {
        ColorPicker::metro()
            .companions(self.count())
            .orientation(if self.vertical {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            .variant(if self.swatches {
                ColorPickerVariant::Swatches
            } else {
                ColorPickerVariant::Ribbon
            })
    }
    fn theme(&self, ui: &mut Ui) {
        let color = ColorPicker::metro().schemes[self.selected].primary;
        let mut theme = Theme::new(self.light, Accent::Teal).with_accent(color);
        theme.text_scale = ui.theme.text_scale;
        theme.reduce_motion = self.reduced;
        theme.tilt = ui.theme.tilt;
        ui.set_theme(theme);
    }
    fn colour(&self) -> Vec<Widget> {
        let mut rows = vec![
            heading("colour-heading", "a ribbon of possibilities"),
            note(
                "colour-note",
                "Choose a scheme. The carets follow; apply it to change the interface.",
            ),
            Widget::choice(
                "companion-count",
                strings(&["none", "one accent", "two accents", "three accents"]),
                self.companions,
                false,
            ),
            Widget::row(
                "colour-options",
                vec![
                    Widget::toggle("vertical", "Vertical", self.vertical).grow(),
                    Widget::toggle("swatches", "Swatches", self.swatches).grow(),
                ],
            )
            .height(72.),
            Widget::color_picker("colour-main", "Scheme", self.picker(), self.selected)
                .height(if self.vertical { 360. } else { 96. }),
        ];
        let scheme = &ColorPicker::metro().schemes[self.selected];
        rows.push(note(
            "scheme-note",
            &format!(
                "{} / {} / companions are palette data",
                scheme.name,
                hex(scheme.primary)
            ),
        ));
        rows.push(
            Widget::row(
                "apply-row",
                vec![
                    Widget::button("apply-colour", "Apply to interface", ButtonStyle::Primary),
                    button("copy-colour", "Show colour values"),
                ],
            )
            .height(48.),
        );
        rows.push(heading("variants-heading", "four companion densities"));
        for (i, count) in [
            CompanionCount::None,
            CompanionCount::One,
            CompanionCount::Two,
            CompanionCount::Three,
        ]
        .into_iter()
        .enumerate()
        {
            rows.push(Widget::color_picker(
                Id::from(format!("density-{i}").as_str()),
                [
                    "Primary only",
                    "One companion",
                    "Two companions",
                    "Three companions",
                ][i],
                ColorPicker::metro().companions(count),
                self.selected,
            ));
        }
        rows.push(heading(
            "vertical-heading",
            "vertical / ribbon and swatches",
        ));
        rows.push(note(
            "vertical-note",
            "Wheel or use arrows to reveal more colours in a short strip.",
        ));
        rows.push(
            Widget::row(
                "vertical-variants",
                vec![
                    Widget::color_picker(
                        "vertical-ribbon",
                        "Ribbon",
                        ColorPicker::metro()
                            .orientation(Orientation::Vertical)
                            .companions(CompanionCount::Three),
                        self.selected,
                    )
                    .grow(),
                    Widget::color_picker(
                        "vertical-swatches",
                        "Swatches",
                        ColorPicker::metro()
                            .orientation(Orientation::Vertical)
                            .variant(ColorPickerVariant::Swatches)
                            .companions(CompanionCount::Three),
                        self.selected,
                    )
                    .grow(),
                ],
            )
            .height(312.),
        );
        rows.push(
            Widget::color_picker(
                "disabled-palette",
                "Disabled palette",
                ColorPicker::metro().companions(CompanionCount::One),
                self.selected,
            )
            .disabled(true),
        );
        rows.push(heading("custom-heading", "your own schemes"));
        rows.push(Widget::color_picker(
            "custom-palette",
            "Paper & ink",
            ColorPicker::new(vec![
                ColorScheme::new("newsprint", Color::hex(0xe6ddc6))
                    .companions(&[Color::hex(0x202020), Color::hex(0xb5432e)]),
                ColorScheme::new("blueprint", Color::hex(0x194b80))
                    .companions(&[Color::hex(0xeaf0eb)]),
                ColorScheme::new("graphite", Color::hex(0x34363a)),
            ])
            .variant(ColorPickerVariant::Swatches)
            .companions(CompanionCount::Two),
            self.custom,
        ));
        rows
    }
    fn controls(&self) -> Vec<Widget> {
        let mut rows = vec![
            heading("controls-heading", "the everyday elements"),
            Widget::toggle("toggle", "Audio engine", self.toggled),
            Widget::toggle("disabled-toggle", "Disabled setting", true).disabled(true),
            Widget::row(
                "buttons",
                vec![
                    Widget::button("primary", "Primary", ButtonStyle::Primary),
                    button("secondary", "Secondary"),
                    Widget::button("text-button", "Text action", ButtonStyle::Text),
                ],
            )
            .height(48.),
            button("disabled-button", "Unavailable").disabled(true),
            Widget::slider("amount", "Output / 0–100", self.amount, 0., 100., 50.),
            Widget::slider("bipolar", "Pan / −1 to +1", self.knob, -1., 1., 0.),
            Widget::row(
                "knobs",
                vec![
                    Widget::knob("knob", "pan", self.knob, -1., 1., 0.),
                    Widget::knob("disabled-knob", "disabled", 0.6, 0., 1., 0.5).disabled(true),
                    button("learn", "MIDI learn"),
                ],
            )
            .height(96.),
            Widget::input("name", "Session name", &self.text),
            Widget::new(
                "invalid-input",
                "Tempo",
                Kind::Input {
                    value: self.invalid.clone(),
                    multiline: false,
                    error: Some("Enter a number from 20 to 300".into()),
                },
            )
            .height(88.),
            Widget::input("disabled-input", "Read only example", "studio").disabled(true),
            Widget::new(
                "multiline",
                "Notes",
                Kind::Input {
                    value: self.notes.clone(),
                    multiline: true,
                    error: None,
                },
            )
            .height(120.),
            Widget::picker(
                "output",
                "Audio output",
                strings(&[
                    "Built-in audio",
                    "Studio monitors",
                    "Headphones",
                    "USB interface",
                    "Network output",
                ]),
                self.output,
            ),
            Widget::choice(
                "tabs",
                strings(&["Session", "Routing", "History"]),
                self.choice,
                false,
            ),
            Widget::choice(
                "sample-pivot",
                strings(&["kick", "snare", "hat", "master"]),
                self.pivot,
                true,
            ),
            heading("icons-heading", "the complete icon set"),
        ];
        let icons = [
            ("More", Icon::DotsThree),
            ("More vertical", Icon::DotsThreeVertical),
            ("Add", Icon::Plus),
            ("Subtract", Icon::Minus),
            ("Close", Icon::X),
            ("Confirm", Icon::Check),
            ("Caret up", Icon::CaretUp),
            ("Caret down", Icon::CaretDown),
            ("Caret left", Icon::CaretLeft),
            ("Caret right", Icon::CaretRight),
            ("Up", Icon::ArrowUp),
            ("Down", Icon::ArrowDown),
            ("Left", Icon::ArrowLeft),
            ("Right", Icon::ArrowRight),
            ("Play", Icon::Play),
            ("Stop", Icon::Stop),
            ("Record", Icon::Record),
        ];
        for (row, chunk) in icons.chunks(6).enumerate() {
            rows.push(
                Widget::row(
                    Id::from(format!("icons-{row}").as_str()),
                    chunk
                        .iter()
                        .map(|(name, icon)| {
                            Widget::icon_button(
                                Id::from(format!("icon-{name}").as_str()),
                                *name,
                                *icon,
                            )
                        })
                        .collect(),
                )
                .gap(8.)
                .height(48.),
            );
        }
        rows
    }
    fn typography(&self) -> Vec<Widget> {
        let mut rows = vec![
            heading("type-heading", "geist / three voices"),
            note(
                "type-note",
                "Sans for hierarchy. Mono for values. Pixel for a deliberate display moment.",
            ),
            Widget::slider(
                "weight",
                "Variable weight / 100–900",
                self.weight_target,
                100.,
                900.,
                300.,
            ),
            Widget::new("weight-specimen", "Variable Geist specimen", Kind::Custom).height(88.),
            Widget::row(
                "weight-actions",
                vec![
                    button("weight-light", "Animate to 200"),
                    button("weight-bold", "Animate to 800"),
                ],
            )
            .height(48.),
        ];
        let roles = [
            TypeRole::Display,
            TypeRole::Title,
            TypeRole::Heading,
            TypeRole::Value,
            TypeRole::Body,
            TypeRole::Label,
            TypeRole::Caption,
            TypeRole::MonoValue,
            TypeRole::MonoLarge,
            TypeRole::PixelDisplay,
            TypeRole::PixelCircle,
            TypeRole::PixelGrid,
            TypeRole::PixelLine,
            TypeRole::PixelTriangle,
        ];
        for (i, role) in roles.into_iter().enumerate() {
            let style = role.style();
            rows.push(
                Widget::label(
                    Id::from(format!("type-info-{i}").as_str()),
                    format!(
                        "{role:?} / {} / {} px / weight {}",
                        style.family, style.size, style.weight
                    ),
                    TypeRole::Caption,
                )
                .height(24.),
            );
            rows.push(
                Widget::label(
                    Id::from(format!("type-{i}").as_str()),
                    if i >= 9 {
                        "Brockmann 08"
                    } else if i >= 7 {
                        "120.00 / −6.0 dB"
                    } else {
                        "form follows feeling"
                    },
                    role,
                )
                .height(style.line + 16.),
            );
        }
        rows
    }
    fn tiles(&self) -> Vec<Widget> {
        let mut rows = vec![
            heading("tiles-heading", "live surfaces / clear signals"),
            note(
                "tiles-note",
                "Tiles animate on activation. Live content flips without moving its neighbours.",
            ),
            Widget::row(
                "tile-row",
                vec![
                    Widget::new(
                        "tile",
                        "velocity",
                        Kind::Tile {
                            value: "127".into(),
                            active: self.toggled,
                        },
                    )
                    .width(192.),
                    Widget::new(
                        "live-tile",
                        "session",
                        Kind::Tile {
                            value: "studio".into(),
                            active: true,
                        },
                    )
                    .live_tile("take 08 / recording", 3.)
                    .width(192.),
                ],
            )
            .height(192.),
            heading("meter-heading", "levels and progress"),
            Widget::new(
                "horizontal-meter",
                "Output / −12 dB",
                Kind::Meter {
                    db: -12.,
                    vertical: false,
                },
            )
            .height(32.),
            Widget::new(
                "progress",
                "Export",
                Kind::Progress {
                    value: Some((self.amount / 100.) as f32),
                },
            )
            .height(24.),
            Widget::slider("amount", "Determinate progress", self.amount, 0., 100., 0.),
            Widget::toggle("running", "Indeterminate activity", self.running),
        ];
        if self.running {
            rows.push(
                Widget::new("indeterminate", "Working", Kind::Progress { value: None }).height(32.),
            );
        }
        rows.push(
            Widget::new(
                "vertical-meter",
                "Peak / −6 dB",
                Kind::Meter {
                    db: -6.,
                    vertical: true,
                },
            )
            .width(8.)
            .height(120.),
        );
        rows.push(heading("list-heading", "list selection / drag to reorder"));
        for i in &self.order {
            rows.push(
                Widget::new(
                    Id::from(format!("list-{i}").as_str()),
                    ["Studio session", "Live set", "Sketchbook"][*i],
                    Kind::ListItem {
                        selected: self.item == *i,
                        secondary: [
                            "12 tracks / 120 BPM",
                            "8 tracks / 128 BPM",
                            "3 tracks / 90 BPM",
                        ][*i]
                            .into(),
                    },
                )
                .height(72.),
            );
        }
        rows
    }
    fn overlays(&self) -> Vec<Widget> {
        vec![
            heading("overlay-heading", "layers with a purpose"),
            note(
                "overlay-note",
                "Open each layer to inspect focus, keyboard navigation and entry / exit motion.",
            ),
            button("dialog", "Open dialog"),
            button("menu", "Open context menu"),
            button("commands", "Open command palette"),
            button("toast", "Show notification"),
            Widget::icon_button(
                "tooltip",
                "A named action / tooltip after a short hover",
                Icon::DotsThree,
            ),
            Widget::picker(
                "output",
                "Aligned dropdown",
                strings(&[
                    "Built-in audio",
                    "Studio monitors",
                    "Headphones",
                    "USB interface",
                    "Network output",
                ]),
                self.output,
            ),
            Widget::label(
                "overlay-help",
                "Escape dismisses a layer. Tab moves focus. Enter confirms an option.",
                TypeRole::Body,
            )
            .height(40.),
        ]
    }
    fn motion(&self) -> Vec<Widget> {
        vec![
            heading("motion-heading", "motion you can interrupt"),
            note(
                "motion-note",
                "Retarget a control during its transition. The next motion starts where it is.",
            ),
            Widget::toggle("reduce-motion", "Reduced motion", self.reduced),
            Widget::toggle("toggle", "Switch / 180 ms", self.toggled),
            Widget::slider(
                "amount",
                "Programmatic value / 240 ms",
                self.amount,
                0.,
                100.,
                50.,
            ),
            Widget::row(
                "presets",
                vec![button("low", "Set 15"), button("high", "Set 85")],
            )
            .height(48.),
            Widget::color_picker(
                "motion-palette",
                "Selection / 180 ms",
                ColorPicker::metro().companions(CompanionCount::Three),
                self.selected,
            ),
            button("navigate", "Replay page transition"),
            button("toast", "Replay notification"),
            note(
                "slow-help",
                "Ctrl/Cmd + Shift + S: slow motion. Ctrl/Cmd + Shift + Space: pause.",
            ),
            note(
                "step-help",
                "Ctrl/Cmd + Shift + .: advance one frame while paused.",
            ),
            note(
                "debug-help",
                "F9: grid. F10: baselines. F11: hit areas. F12: timings.",
            ),
        ]
    }
    fn layout(&self) -> Vec<Widget> {
        vec![
            heading("layout-heading", "a four pixel rhythm"),
            note(
                "layout-note",
                "Twelve columns / 16 px gutters / margins adapt at 720 and 1280 px.",
            ),
            Widget::label("layout-display", "space is structure", TypeRole::Title).height(64.),
            Widget::row(
                "layout-row",
                vec![
                    Widget::new(
                        "layout-tile-1",
                        "one unit",
                        Kind::Tile {
                            value: "96".into(),
                            active: true,
                        },
                    )
                    .width(96.),
                    Widget::new(
                        "layout-tile-2",
                        "two units",
                        Kind::Tile {
                            value: "200".into(),
                            active: false,
                        },
                    )
                    .width(200.),
                ],
            )
            .height(96.),
            Widget::column(
                "layout-stack",
                vec![
                    Widget::label("stack-heading", "A vertical stack", TypeRole::Heading),
                    Widget::label(
                        "stack-body",
                        "Related content shares an axis and a deliberate gap.",
                        TypeRole::Body,
                    ),
                    button("layout-action", "Aligned action"),
                ],
            )
            .gap(16.)
            .height(136.),
            note(
                "layout-guide",
                "Resize this window to see the catalogue reflow. F9 reveals the grid.",
            ),
            note(
                "layout-guidelines",
                "Use whitespace for grouping; reserve borders for interaction and structure.",
            ),
        ]
    }
}
impl Application for Showcase {
    fn title(&self) -> &str {
        "Brockmann / component library"
    }
    fn configure(&mut self, ui: &mut Ui) {
        self.reduced |= ui.theme.reduce_motion;
        self.theme(ui);
    }
    fn needs_frame(&self, ui: &Ui) -> bool {
        self.page == 2 && self.weight.active(ui.clock.now()) && !ui.clock.paused
    }
    fn view(&mut self, v: Rect) -> Widget {
        let margin = Grid::new(v.w, v.h, 1.).margin;
        let mut rows = match self.page {
            0 => self.colour(),
            1 => self.controls(),
            2 => self.typography(),
            3 => self.tiles(),
            4 => self.overlays(),
            5 => self.motion(),
            _ => self.layout(),
        };
        let available = (v.w - margin * 2.).clamp(0., 960.);
        for row in &mut rows {
            if let Layout::Horizontal { gap, padding } = row.layout {
                let minimum = row
                    .children
                    .iter()
                    .map(|w| {
                        w.size
                            .width
                            .unwrap_or(if matches!(w.kind, Kind::ColorPicker { .. }) {
                                128.
                            } else {
                                200.
                            })
                    })
                    .sum::<f32>()
                    + gap * row.children.len().saturating_sub(1) as f32
                    + padding * 2.;
                if minimum > available {
                    row.layout = Layout::Vertical { gap, padding };
                    row.size.height = row.children.iter().map(|w| w.size.height).sum::<f32>()
                        + gap * row.children.len().saturating_sub(1) as f32
                        + padding * 2.;
                    for child in &mut row.children {
                        child.size.grow = 0.;
                    }
                }
            }
        }
        let mut root =
            Widget::column(
                "showcase-root",
                vec![
                    Widget::label("brand", "BROCKMANN / COMPONENT LIBRARY", TypeRole::Label).at(
                        Rect::new(margin, 24., (v.w - margin * 2. - 120.).max(120.), 20.),
                    ),
                    Widget::button("theme", "Dark / light", ButtonStyle::Text).at(Rect::new(
                        (v.w - margin - 104.).max(margin),
                        20.,
                        104.,
                        32.,
                    )),
                    Widget::choice("showcase-pages", strings(&PAGES), self.page, true)
                        .at(Rect::new(margin, 64., (v.w - margin).max(0.), 56.)),
                    Widget::column(Id::from(format!("page-{}", self.page).as_str()), rows)
                        .gap(16.)
                        .scrollable()
                        .at(Rect::new(
                            margin,
                            152.,
                            (v.w - margin * 2.).clamp(0., 960.),
                            (v.h - 216.).max(64.),
                        )),
                    Widget::label("status", &self.status, TypeRole::Caption).at(Rect::new(
                        margin,
                        (v.h - 40.).max(0.),
                        (v.w - margin * 2.).max(0.),
                        24.,
                    )),
                ],
            );
        root.layout = Layout::Overlay;
        root.at(v)
    }
    fn paint(&mut self, ui: &Ui, scene: &mut Scene) {
        if self.page == 2
            && let Some((bounds, clip)) = ui.widget_geometry(Id::from("weight-specimen"))
        {
            scene.push(
                Primitive::Text {
                    bounds,
                    text: "variable by nature".into(),
                    role: TypeRole::Display,
                    color: ui.current_theme().fg,
                    weight: Some(
                        self.weight
                            .sample(ui.clock.now())
                            .0
                            .round()
                            .clamp(100., 900.) as u16,
                    ),
                },
                clip,
            );
        }
    }
    fn event(&mut self, event: Event, ui: &mut Ui) {
        match event {
            Event::Changed(id, value) => match value {
                Value::Selection(n) if id == Id::from("showcase-pages") => {
                    ui.navigate(n < self.page);
                    self.page = n;
                }
                Value::Selection(n) if id == Id::from("companion-count") => self.companions = n,
                Value::Selection(n) if id == Id::from("output") => self.output = n,
                Value::Selection(n) if id == Id::from("tabs") => self.choice = n,
                Value::Selection(n) if id == Id::from("sample-pivot") => self.pivot = n,
                Value::Selection(n) if id == Id::from("custom-palette") => self.custom = n,
                Value::Selection(n) => {
                    self.selected = n;
                    self.status = ColorPicker::metro().schemes[n].description();
                }
                Value::Bool(b) if id == Id::from("vertical") => self.vertical = b,
                Value::Bool(b) if id == Id::from("swatches") => self.swatches = b,
                Value::Bool(b) if id == Id::from("reduce-motion") => {
                    self.reduced = b;
                    self.theme(ui);
                }
                Value::Bool(b) if id == Id::from("running") => self.running = b,
                Value::Bool(b) => self.toggled = b,
                Value::Number(n) if id == Id::from("weight") => {
                    self.weight_target = n;
                    self.weight.retarget(
                        n,
                        ui.clock.now(),
                        if self.reduced { 0.1 } else { 0.18 },
                        Curve::OutExpo,
                    );
                }
                Value::Number(n) if id == Id::from("amount") => self.amount = n,
                Value::Number(n) => self.knob = n,
                Value::Text(text) if id == Id::from("invalid-input") => self.invalid = text,
                Value::Text(text) if id == Id::from("multiline") => self.notes = text,
                Value::Text(text) => self.text = text,
            },
            Event::Click(id) => match id {
                id if id == Id::from("theme") => {
                    self.light = !self.light;
                    self.theme(ui);
                }
                id if id == Id::from("apply-colour") => {
                    self.theme(ui);
                    ui.notify("Scheme applied to the interface");
                }
                id if id == Id::from("copy-colour") => {
                    let picker = ColorPicker::metro();
                    let scheme = &picker.schemes[self.selected];
                    let mut values = scheme.description();
                    for c in scheme.companions.iter().take(self.companions).flatten() {
                        values.push_str(&format!(" / {}", hex(*c)));
                    }
                    ui.show_dialog("colour values", values, strings(&["close"]));
                }
                id if id == Id::from("dialog") => ui.show_dialog(
                    "save session?",
                    "Your changes are ready. Choose an action to continue.",
                    strings(&["save", "cancel"]),
                ),
                id if id == Id::from("menu") => ui.context_menu(
                    Point::new(ui.viewport.w / 2., 240.),
                    strings(&["duplicate", "rename", "delete"]),
                ),
                id if id == Id::from("commands") => ui.command_palette(strings(&[
                    "colour",
                    "controls",
                    "typography",
                    "tiles",
                    "overlays",
                    "motion",
                    "layout",
                ])),
                id if id == Id::from("toast") => {
                    ui.notify("Session saved / changes are up to date")
                }
                id if id == Id::from("learn") => ui.begin_midi_learn(Id::from("knob")),
                id if id == Id::from("low") => self.amount = 15.,
                id if id == Id::from("high") => self.amount = 85.,
                id if id == Id::from("navigate") => ui.navigate(false),
                id if id == Id::from("weight-light") || id == Id::from("weight-bold") => {
                    self.weight_target = if id == Id::from("weight-light") {
                        200.
                    } else {
                        800.
                    };
                    self.weight.retarget(
                        self.weight_target,
                        ui.clock.now(),
                        if self.reduced { 0.1 } else { 0.8 },
                        Curve::InOut,
                    );
                }
                _ => {
                    for i in 0..3 {
                        if id == Id::from(format!("list-{i}").as_str()) {
                            self.item = i;
                        }
                    }
                    self.status = "Action activated".into();
                }
            },
            Event::Command(command) => {
                if let Some(page) = PAGES.iter().position(|p| *p == command) {
                    ui.navigate(page < self.page);
                    self.page = page;
                } else {
                    self.status = format!("Command / {command}");
                    ui.notify(&self.status);
                }
            }
            Event::Reorder { item, before } => {
                let from = self
                    .order
                    .iter()
                    .position(|i| item == Id::from(format!("list-{i}").as_str()));
                let to = self
                    .order
                    .iter()
                    .position(|i| before == Id::from(format!("list-{i}").as_str()));
                if let (Some(from), Some(to)) = (from, to) {
                    let item = self.order.remove(from);
                    self.order.insert(to, item);
                }
            }
            Event::MidiBound(_, cc) => ui.notify(format!("Bound to CC {cc}")),
            _ => {}
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    let option = |flag: &str| {
        args.iter()
            .position(|s| s == flag)
            .and_then(|i| args.get(i + 1))
    };
    let mut app = Showcase {
        light: args.iter().any(|s| s == "--light"),
        ..Default::default()
    };
    if let Some(page) = option("--page") {
        app.page = PAGES
            .iter()
            .position(|p| p == page)
            .ok_or("unknown showcase page")?;
    }
    app.vertical = args.iter().any(|s| s == "--vertical");
    app.swatches = args.iter().any(|s| s == "--swatches");
    if let Some(count) = option("--companions") {
        app.companions = count.parse::<usize>()?;
        if app.companions > 3 {
            return Err("companions must be between 0 and 3".into());
        }
    }
    if let Some(path) = option("--snapshot") {
        let scale = option("--scale")
            .map(|s| s.parse::<f32>())
            .transpose()?
            .unwrap_or(1.);
        if !scale.is_finite() || !(0.5..=4.).contains(&scale) {
            return Err("scale must be between 0.5 and 4".into());
        }
        let width = option("--width")
            .map(|s| s.parse::<f32>())
            .transpose()?
            .unwrap_or(1100.);
        let height = option("--height")
            .map(|s| s.parse::<f32>())
            .transpose()?
            .unwrap_or(800.);
        let scroll = option("--scroll")
            .map(|s| s.parse::<f32>())
            .transpose()?
            .unwrap_or(0.);
        if !width.is_finite()
            || !height.is_finite()
            || !(320.0..=2560.0).contains(&width)
            || !(320.0..=1600.0).contains(&height)
            || !scroll.is_finite()
            || scroll < 0.
        {
            return Err("invalid snapshot size or scroll position".into());
        }
        let snapshot_view = Rect::new(0., 0., width, height);
        pollster::block_on(support::snapshot(
            &mut app,
            path,
            scale,
            None,
            None,
            None,
            Some((snapshot_view, scroll)),
        ))
    } else {
        run(app)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn every_catalogue_page_mounts_and_paints_at_compact_and_desktop_sizes() {
        for width in [400., 1100., 1600.] {
            for page in 0..PAGES.len() {
                let mut app = Showcase {
                    page,
                    selected: 15,
                    companions: 3,
                    vertical: true,
                    swatches: true,
                    ..Default::default()
                };
                let mut ui = Ui::default();
                app.configure(&mut ui);
                let bounds = Rect::new(0., 0., width, 800.);
                ui.reconcile(app.view(bounds), bounds);
                ui.advance(1.);
                ui.scroll.offset = ui.scroll.max;
                ui.reconcile(app.view(bounds), bounds);
                let mut scene = ui.scene();
                app.paint(&ui, &mut scene);
                assert!(!scene.draws.is_empty());
                assert!(scene.draws.iter().all(|d| d.clip.x.is_finite()
                    && d.clip.y.is_finite()
                    && d.clip.w >= 0.
                    && d.clip.h >= 0.));
                assert!(
                    ui.accessibility_tree(1.)
                        .nodes
                        .iter()
                        .any(|(_, n)| n.label() == Some("BROCKMANN / COMPONENT LIBRARY"))
                );
            }
        }
    }
    #[test]
    fn catalogue_selections_are_independent_and_weight_animation_stops_at_rest() {
        let mut app = Showcase::default();
        let mut ui = Ui::default();
        for (name, index) in [
            ("sample-pivot", 3),
            ("tabs", 2),
            ("custom-palette", 1),
            ("colour-main", 15),
        ] {
            app.event(
                Event::Changed(Id::from(name), Value::Selection(index)),
                &mut ui,
            );
        }
        assert_eq!(
            (app.pivot, app.choice, app.custom, app.selected),
            (3, 2, 1, 15)
        );
        app.page = 2;
        app.event(Event::Click(Id::from("weight-bold")), &mut ui);
        assert!(app.needs_frame(&ui));
        ui.advance(1.);
        assert!(!app.needs_frame(&ui));
        assert_eq!(app.weight.sample(ui.clock.now()).0, 800.);
    }
}
