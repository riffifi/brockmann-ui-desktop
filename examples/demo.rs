mod support;
use brockmann::{
    layout::Grid,
    theme::TypeRole,
    ui::Value,
    widget::{ButtonStyle, Kind},
    *,
};
struct Demo {
    page: usize,
    synth: usize,
    light: bool,
    accent: usize,
    enabled: bool,
    motion: bool,
    feedback: bool,
    tilt: bool,
    velocity: bool,
    name: String,
    values: [f64; 4],
    kit: usize,
    code: String,
    editor_file: usize,
    kit_code: String,
}
impl Default for Demo {
    fn default() -> Self {
        Self {
            page:0, synth:0, light:false, accent:0, enabled:true,
            motion:true, feedback:false, tilt:true, velocity:true, name:"studio".into(),
            values:[60.,420.,24.,12.], kit:0, editor_file:0,
            kit_code: "pub struct Kit {\n    pub name: String,\n}\n".into(),
            code:"fn main() {\n    let kit = Kit::new(\"studio\");\n    kit.kick().tone(60.0).decay(420.0);\n    kit.play();\n}\n".into(),
        }
    }
}
fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|s| s.to_string()).collect()
}
impl Application for Demo {
    fn configure(&mut self, ui: &mut Ui) {
        self.motion = !ui.theme.reduce_motion;
        self.tilt = ui.theme.tilt;
        let mut theme = Theme::new(self.light, Accent::ALL[self.accent]);
        theme.reduce_motion = ui.theme.reduce_motion;
        theme.text_scale = ui.theme.text_scale;
        theme.tilt = ui.theme.tilt;
        ui.set_theme(theme);
        ui.sound.mode = if self.feedback {
            SoundMode::Subtle
        } else {
            SoundMode::Off
        };
    }
    fn title(&self) -> &str {
        "Brockmann"
    }
    fn view(&mut self, v: Rect) -> Widget {
        let grid = Grid::new(v.w, v.h, 1.);
        let margin = grid.margin;
        let content_w = (v.w - 2. * margin).min(640.);
        let content_x = if v.w > 1000. {
            grid.span(3, 6).x
        } else {
            margin
        };
        let mut children = vec![
            Widget::label("app", "BROCKMANN", TypeRole::Label).at(Rect::new(
                margin,
                24.,
                v.w - margin * 2.,
                16.,
            )),
            Widget::choice(
                "pages",
                strings(&["settings", "drum synth", "editor"]),
                self.page,
                true,
            )
            .at(Rect::new(margin, 64., v.w - margin, 56.)),
        ];
        match self.page {
            0 => {
                children.push(
                    Widget::column(
                        "settings",
                        vec![
                            Widget::toggle("enabled", "Audio engine", self.enabled),
                            Widget::toggle("velocity", "Velocity sensitivity", self.velocity),
                            Widget::toggle("feedback", "Interaction sounds", self.feedback)
                                .sound(SoundFeedback::Silent),
                            Widget::toggle("motion", "Motion", self.motion),
                            Widget::toggle("tilt", "Press tilt", self.tilt),
                            Widget::picker(
                                "theme",
                                "Theme",
                                strings(&["dark", "light"]),
                                usize::from(self.light),
                            ),
                            Widget::picker(
                                "accent",
                                "Accent",
                                Accent::ALL.iter().map(|a| a.name().into()).collect(),
                                self.accent,
                            ),
                        ],
                    )
                    .scrollable()
                    .at(Rect::new(
                        content_x,
                        160.,
                        content_w.min(v.w - margin - content_x),
                        (v.h - 192.).clamp(80., 480.),
                    )),
                );
            }
            1 => {
                children.push(
                    Widget::choice(
                        "instruments",
                        strings(&["kick", "snare", "hat", "toms", "cymbals", "master"]),
                        self.synth,
                        true,
                    )
                    .at(Rect::new(margin, 152., v.w - margin, 56.)),
                );
                children.push(
                    Widget::new(
                        "hit",
                        "velocity",
                        Kind::Tile {
                            value: "127".into(),
                            active: false,
                        },
                    )
                    .at(Rect::new(margin, 240., 200., 200.)),
                );
                let x = if v.w < 720. { margin } else { margin + 248. };
                let y = if v.w < 720. { 472. } else { 240. };
                let knobs = [
                    ("tone", "tone", 20., 200.),
                    ("decay", "decay", 10., 1000.),
                    ("noise", "noise", 0., 100.),
                    ("drive", "drive", 0., 100.),
                ];
                for (i, (id, label, min, max)) in knobs.into_iter().enumerate() {
                    children.push(
                        Widget::knob(id, label, self.values[i], min, max, self.values[i]).at(
                            Rect::new(
                                x + (i % 2) as f32 * 104.,
                                y + (i / 2) as f32 * 104.,
                                96.,
                                88.,
                            ),
                        ),
                    );
                }
                children.push(
                    Widget::slider("level", "level", self.values[3], 0., 100., 12.).at(Rect::new(
                        x,
                        y + 240.,
                        200.,
                        64.,
                    )),
                );
                children.push(
                    Widget::new(
                        "meter",
                        "Output level",
                        Kind::Meter {
                            db: -12.,
                            vertical: true,
                        },
                    )
                    .at(Rect::new(v.w - margin, 240., 4., 320.)),
                );
                children.push(
                    Widget::row(
                        "appbar",
                        vec![
                            Widget::button("learn", "learn midi", ButtonStyle::Text).width(104.),
                            Widget::button("presets", "presets", ButtonStyle::Text).width(80.),
                            Widget::button("settings", "settings", ButtonStyle::Text).width(80.),
                        ],
                    )
                    .at(Rect::new(margin, v.h - 56., v.w - margin * 2., 40.)),
                );
            }
            _ => {
                children.push(
                    Widget::choice(
                        "files",
                        strings(&["main.rs", "kit.rs"]),
                        self.editor_file,
                        false,
                    )
                    .at(Rect::new(margin, 152., v.w - margin * 2., 40.)),
                );
                children.push(
                    Widget::new(
                        "code",
                        "main.rs",
                        Kind::Input {
                            value: if self.editor_file == 0 {
                                self.code.clone()
                            } else {
                                self.kit_code.clone()
                            },
                            multiline: true,
                            error: None,
                        },
                    )
                    .at(Rect::new(
                        margin + 40.,
                        216.,
                        v.w - margin * 2. - 40.,
                        (v.h - 272.).max(80.),
                    )),
                );
                children.push(
                    Widget::button("commands", "commands", ButtonStyle::Text).at(Rect::new(
                        v.w - margin - 100.,
                        v.h - 48.,
                        100.,
                        40.,
                    )),
                );
                children.push(
                    Widget::label("status", "rust    utf-8", TypeRole::Caption).at(Rect::new(
                        margin,
                        v.h - 32.,
                        v.w - margin * 2.,
                        16.,
                    )),
                );
            }
        }
        let mut root = Widget::column("root", children);
        root.layout = brockmann::layout::Layout::Overlay;
        root.at(v)
    }
    fn paint(&mut self, ui: &Ui, scene: &mut brockmann::scene::Scene) {
        if self.page == 2 {
            let v = ui.viewport;
            let margin = Grid::new(v.w, v.h, 1.).margin;
            let theme = ui.current_theme();
            let clip = Rect::new(margin, 216., v.w - 2. * margin, (v.h - 272.).max(0.));
            let code = if self.editor_file == 0 {
                &self.code
            } else {
                &self.kit_code
            };
            for i in 0..code.lines().count().max(1) {
                scene.text(
                    Rect::new(margin, 224. + i as f32 * 22., 24., 22.),
                    format!("{}", i + 1),
                    TypeRole::MonoValue,
                    theme.fg3,
                    clip,
                );
            }
            scene.rect(Rect::new(margin + 32., 216., 1., clip.h), theme.line, v);
        }
    }
    fn event(&mut self, event: Event, ui: &mut Ui) {
        match event {
            Event::Changed(id, value) => {
                match value {
                    Value::Selection(n) if id == Id::from("pages") => {
                        ui.navigate(n < self.page);
                        self.page = n;
                    }
                    Value::Selection(n) if id == Id::from("instruments") => {
                        self.synth = n;
                    }
                    Value::Selection(n) if id == Id::from("theme") => self.light = n == 1,
                    Value::Selection(n) if id == Id::from("accent") => self.accent = n,
                    Value::Bool(v) if id == Id::from("enabled") => self.enabled = v,
                    Value::Bool(v) if id == Id::from("velocity") => self.velocity = v,
                    Value::Bool(v) if id == Id::from("feedback") => self.feedback = v,
                    Value::Bool(v) if id == Id::from("motion") => self.motion = v,
                    Value::Bool(v) if id == Id::from("tilt") => self.tilt = v,
                    Value::Number(v) => {
                        for (i, key) in ["tone", "decay", "noise", "drive"].iter().enumerate() {
                            if id == Id::from(*key) {
                                self.values[i] = v;
                            }
                        }
                        if id == Id::from("level") {
                            self.values[3] = v;
                        }
                    }
                    Value::Text(v) if id == Id::from("name") => self.name = v,
                    Value::Text(v) if id == Id::from("code") => {
                        if self.editor_file == 0 {
                            self.code = v
                        } else {
                            self.kit_code = v
                        }
                    }
                    Value::Selection(v) if id == Id::from("files") => self.editor_file = v,
                    _ => {}
                }
                let mut theme = Theme::new(self.light, Accent::ALL[self.accent.min(6)]);
                theme.text_scale = ui.theme.text_scale;
                theme.reduce_motion = !self.motion;
                theme.tilt = self.tilt;
                ui.set_theme(theme);
                ui.sound.mode = if self.feedback {
                    SoundMode::Subtle
                } else {
                    SoundMode::Off
                };
            }
            Event::Click(id) if id == Id::from("commands") => ui.command_palette(strings(&[
                "settings",
                "drum synth",
                "editor",
                "light theme",
                "dark theme",
            ])),
            Event::Command(c) if ["settings", "drum synth", "editor"].contains(&c.as_str()) => {
                let next = match c.as_str() {
                    "settings" => 0,
                    "drum synth" => 1,
                    _ => 2,
                };
                ui.navigate(next < self.page);
                self.page = next;
            }
            Event::Command(c) if c == "light theme" || c == "dark theme" => {
                self.light = c == "light theme";
                let mut theme = Theme::new(self.light, Accent::ALL[self.accent]);
                theme.text_scale = ui.theme.text_scale;
                theme.reduce_motion = !self.motion;
                theme.tilt = self.tilt;
                ui.set_theme(theme);
            }
            Event::Click(id) if id == Id::from("settings") => {
                ui.navigate(true);
                self.page = 0;
            }
            Event::Click(id) if id == Id::from("learn") => ui.begin_midi_learn(Id::from("tone")),
            Event::Click(id) if id == Id::from("presets") => {
                ui.show_dialog("presets", "studio", strings(&["load", "cancel"]))
            }
            Event::Command(command) if command == "load" => {
                self.kit += 1;
                self.values = [48. + self.kit as f64 * 4., 360., 18., 8.];
                ui.notify("preset loaded");
            }
            Event::MidiBound(_, cc) => ui.notify(format!("CC {cc}")),
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
    let mut demo = Demo {
        light: args.iter().any(|arg| arg == "--light"),
        ..Demo::default()
    };
    if let Some(page) = option("--page") {
        demo.page = match page.as_str() {
            "settings" => 0,
            "synth" => 1,
            "editor" => 2,
            _ => page.parse::<usize>()?.min(2),
        };
    }
    if let Some(instrument) = option("--instrument") {
        demo.synth = match instrument.as_str() {
            "kick" => 0,
            "snare" => 1,
            "hat" => 2,
            "toms" => 3,
            "cymbals" => 4,
            "master" => 5,
            _ => instrument.parse::<usize>()?.min(5),
        };
    }
    if let Some(path) = option("--snapshot") {
        let scale = option("--scale")
            .map(|s| s.parse::<f32>())
            .transpose()?
            .unwrap_or(1.);
        if !scale.is_finite() || !(0.5..=4.).contains(&scale) {
            return Err("scale must be between 0.5 and 4".into());
        }
        pollster::block_on(support::snapshot(
            &mut demo,
            path,
            scale,
            option("--open-picker").map(String::as_str),
            option("--hover-picker").map(String::as_str),
            option("--picker-phase").map(String::as_str),
            None,
        ))
    } else {
        run(demo)
    }
}
