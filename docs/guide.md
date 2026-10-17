# Building with Brockmann

Brockmann is a native Rust toolkit for flat, typographic desktop interfaces. Start with `Application`, return a keyed widget tree from `view`, and handle domain changes in `event`. The host supplies the window, renderer, animation clock, input routing and AccessKit adapter.

The settings page in `examples/demo.rs` is the smallest complete composition to study. The synth page demonstrates parameter controls; the editor page demonstrates shared text editing. The synth is a UI stub, with no audio engine.

## Run and integrate

From this checkout:

```sh
cargo run --example demo
cargo run --example showcase
cargo run --example demo -- --page synth
cargo run --example demo -- --page editor
cargo doc --no-deps --open
```

Use a local dependency from an application alongside the checkout:

```toml
[dependencies]
brockmann = { path = "../brockmann" }
```

The toolkit embeds Geist Sans, Mono, all five Pixel cuts, and its permitted Phosphor icons. Applications do not need to discover system fonts or ship separate font files. Linux requires a graphical session and a supported GPU backend. Offscreen snapshots also work with a software adapter.

## A complete settings view

```no_run
use brockmann::{Application, Event, Id, Rect, Ui, Widget, run};
use brockmann::layout::Grid;
use brockmann::ui::Value;

struct Settings {
    audio: bool,
    output: usize,
}

impl Application for Settings {
    fn title(&self) -> &str { "studio settings" }

    fn view(&mut self, viewport: Rect) -> Widget {
        let grid = Grid::new(viewport.w, viewport.h, 1.0);
        Widget::column("settings", vec![
            Widget::toggle("audio", "Audio engine", self.audio),
            Widget::picker(
                "output", "Output",
                vec!["speakers".into(), "headphones".into()], self.output,
            ),
        ])
        .gap(8.0)
        .scrollable()
        .at(Rect::new(grid.margin, 144.0, grid.bounds.w, viewport.h - 168.0))
    }

    fn event(&mut self, event: Event, _ui: &mut Ui) {
        match event {
            Event::Changed(id, Value::Bool(on)) if id == Id::from("audio") => {
                self.audio = on;
                // Publish to an application-owned parameter channel here.
            }
            Event::Changed(id, Value::Selection(index)) if id == Id::from("output") => {
                self.output = index;
            }
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    run(Settings { audio: true, output: 0 })
}
```

The application owns `audio` and `output`. A widget describes their current values; it is not the source of truth. Apply `Changed` events to your model before the next `view`. Otherwise reconciliation sees the old value and animates the control back to it.

## Identity and lifecycle

Give each widget a stable, unique key. Labels may change; keys should survive those changes. Use a record's identity for reorderable lists, rather than its current row number. Reusing a key preserves focus, text history, drag state and animation. Removing it releases that state. ID zero is reserved for the accessibility root; duplicate keys panic during reconciliation.

`view` is called repeatedly, including during animation. Keep it quick and free of side effects: do not load presets, send commands, open devices or start threads there. Use these hooks instead:

| Hook | Responsibility |
| --- | --- |
| `configure(&mut Ui)` | Initial application preferences; retain the host's OS motion/text preferences |
| `connected(InputSender)` | Save or share the handle used to wake the UI from device/worker threads |
| `update(&mut Ui)` | Nonblocking consumption of application-owned messages before the next view |
| `event(Event, &mut Ui)` | Apply actions and value changes to the domain model |
| `paint(&Ui, &mut Scene)` | Add application-specific rendering primitives |

Use `Event::Click` for commands and `Event::Changed` for values. Picker and choice values are zero-based option indices. Handle `Event::Command` for dialog, context-menu and palette commands. `Dismissed` reports popup closure; it does not imply that a value changed.

## Compose with stacks and the grid

All public rectangles use logical pixels. Let the renderer handle device scaling and physical edge snapping. Do not multiply `.at` rectangles by `Window::scale_factor()` yourself.

Use `Widget::column` and `row` for ordinary composition. A column uses each child's height; a row uses its width. `.grow()` distributes the remaining space along the stack's main axis. `.at` sets an **absolute** rectangle, including on a nested child; it is not an offset relative to its parent. Prefer stack placement for repeated rows.

```rust
use brockmann::{Rect, Widget};
use brockmann::widget::ButtonStyle;

let actions = Widget::row("actions", vec![
    Widget::button("save", "save", ButtonStyle::Primary).width(120.0),
    Widget::button("cancel", "cancel", ButtonStyle::Text).width(80.0),
]).gap(24.0).at(Rect::new(48.0, 480.0, 400.0, 40.0));
```

`Grid::span(start, count)` uses zero-based columns. `span(3, 6)` starts on the fourth column. `Grid::tile` uses 96 px cells with 8 px gaps: a 2×2 tile is 200×200. Align component bounds to the grid; align repeated text consistently within those bounds.

Stack sizes, gaps and padding follow `Theme::text_scale`. Explicit `.at` rectangles are app-owned: leave enough room or reflow them when supporting larger text. A scrollable column clips overflowing children and moves hit targets with them. One scroll viewport can be mounted per `Ui`; independent scroll surfaces need separate UI instances or application-owned layout/scrolling.

## Switches: immediate values, continuous visuals

```rust
use brockmann::Widget;
let audio = Widget::toggle("audio", "Audio engine", true);
```

Use a switch for an independent, persistent boolean. Prefer a positive label such as “Audio engine” or “Velocity sensitivity”; the library supplies “On” / “Off”. Use a button for a one-time action, and a segmented choice or picker for mutually exclusive values.

A switch row is 72 px high. Clicking its label or empty row area toggles on pointer-down. Pressing the thumb starts a captured interaction: a click toggles on release, while a drag commits to the nearer side, or the flick direction above 600 logical px/s. Space/Enter toggles; Left sets Off and Right sets On.

The value event is immediate when the interaction commits. The visual thumb then travels for 180 ms on the shared clock, while the word changes at the midpoint. Do not delay applying the model until the word or animation finishes. Backend smoothing, audio ramps and asynchronous device state belong to the application.

The track fill uses the same normalized progress across its full width, including both endpoints. The outline blends back to its resting color during travel. This avoids the disappearing end caps that made turning off look abrupt. Thumb timing and travel are the same in both directions. New targets interrupt the current tween; reduced motion substitutes a 100 ms crossfade.

For visual review, test **both** directions, rapid reversals, label clicks, thumb clicks and drags. Also test an On switch in light mode: the foreground-colored thumb must stay visible even when accent text uses black for contrast.

## Pickers: the selected option stays in context

```rust
use brockmann::Widget;
let output = Widget::picker(
    "output", "Output",
    vec!["speakers".into(), "headphones".into(), "interface".into()],
    1,
);
```

Use a picker for a named, single selection. Keep option order stable and `selected` within the options array. For two to four short, frequently changed options, prefer `Widget::choice(..., false)` so all options remain visible. Empty or unavailable choices should be represented deliberately, for example with a disabled control and an actual current value.

The closed picker reserves **16 px of horizontal inset** for its label and value, multiplied by text scale. Its hover outline surrounds that inset; text does not touch the left stroke. The keyboard focus ring is separate and sits outside the control with a gap. Do not insert spaces into labels or add your own second border to achieve padding.

On a regular desktop viewport (720 logical px and wider), the menu opens **over the field**, placing the selected option's text at the closed value's position. Options before it extend upward and options after it extend downward. Open and closed values use the same type role and horizontal inset. The panel reveals above and below that anchor over 220 ms, so the selected option does not slide away from the pointer. Arrow-key focus glides between rows over 140 ms. On selection, the chosen label moves into the closed value as the surrounding panel collapses over 200 ms. The model still commits immediately; visual settling does not delay the backend. Cancellation collapses around the unchanged value. Reduced motion uses a 100 ms crossfade.

The panel moves only as needed to stay inside the window. Long lists scroll within that fixed panel; wheel scrolling is supported and keyboard navigation reveals its candidate. Hovering a row does not move the menu. Near a window edge, keeping the panel usable takes precedence over perfect text anchoring.

Below 720 px, the picker uses a compact full-page list at the left margin. In either mode, the committed selection uses accent text. Keyboard navigation has a separate inset focus outline. Enter/Space commits the candidate; Escape or a click outside dismisses without changing the model. Opening the menu alone never emits a selection change.

## Mouse navigation in pivots

A pivot keeps its active title on the main left axis and crops earlier titles. Whenever an earlier section exists, Brockmann adds a quiet previous chevron with a 40 px pointer target (48 px for touch), a named accessibility action and a tooltip naming the previous section. It lives in the left gutter when that gutter is clear. On narrow or crowded layouts it occupies a separate lane at the right edge, leaving the title axis intact.

This is automatic for `Widget::choice(..., true)` and applies independently to nested pivots such as the screen selector and synth instrument selector. Clicking the chevron emits the same `Value::Selection` as keyboard Left; route it through your normal page model. It disappears at the first section. Ordinary segmented choices do not gain the chevron.

## Themes and preferences

Update themes through `Ui::set_theme`, which crossfades their color tokens. Preserve motion, text scale and tilt preferences when only changing colors:

```rust
use brockmann::{Accent, Theme, Ui};

fn set_light_theme(ui: &mut Ui, light: bool) {
    let mut theme = Theme::new(light, Accent::Teal);
    theme.text_scale = ui.theme.text_scale;
    theme.reduce_motion = ui.theme.reduce_motion;
    theme.tilt = ui.theme.tilt;
    ui.set_theme(theme);
}
```

Use theme tokens in custom drawing. `on_accent` is chosen for readable text and can be black; do not assume it is white. Numeric data belongs in Geist Mono. Pixel cuts are for occasional large readouts. The renderer supports Geist-only fallback, so scripts absent from the bundled fonts will not gain system-font coverage.

The host reads OS preferences at startup and on focus. `BROCKMANN_REDUCE_MOTION=1` and `BROCKMANN_TEXT_SCALE=1.5` provide explicit overrides. Debug builds also watch `brockmann.local.toml`; copy `brockmann.local.example.toml` as a starting point. Invalid configuration leaves the prior configuration active and displays a toast.

## Parameters, workers and MIDI

Continuous controls report user changes immediately. Apps pass preset/reset values back through `view`, and the UI interpolates their presentation. Keep audio smoothing separate from visual smoothing.

`InputSender::send` wakes the event loop and returns false if it has closed. This handle is intended for ordinary worker/device threads, **not** a real-time audio callback. Audio code should use an app-owned lock-free channel or atomic parameter store, with a non-real-time consumer waking the UI.

```no_run
use brockmann::{Application, Event, Input, InputSender, Rect, Ui, Widget};

struct DeviceApp;
impl Application for DeviceApp {
    fn connected(&mut self, input: InputSender) {
        // A MIDI/device worker can send actual messages using this handle.
        // This illustrative worker sends one controller value.
        std::thread::spawn(move || {
            input.send(Input::Midi { cc: 74, value: 64 });
        });
    }
    fn view(&mut self, _: Rect) -> Widget {
        Widget::knob("tone", "tone", 60.0, 20.0, 200.0, 60.0)
    }
    fn event(&mut self, _: Event, _: &mut Ui) {}
}
```

Right-click a parameter to enter MIDI learn, or call `ui.begin_midi_learn(id)`. The next CC binds to it; Escape cancels learning. The toolkit handles visual learning state and mapped values. Device discovery, persistence of bindings and the audio channel belong to the application. Store and reapply a complete model in a real app; the illustrative `DeviceApp` above omits that domain model.

## Debug and verify

Use F9 for columns, F10 for baselines, F11 for hit areas and F12 for frame timing. Ctrl+Shift+S slows animation tenfold. `Ui::clock` can also be paused and frame-stepped for deterministic review.

```sh
cargo test --all-targets
cargo test --doc
cargo clippy --all-targets -- -D warnings
cargo fmt --check
BROCKMANN_TRACE_FRAMES=1 cargo run --example demo
```

The host renders during animation and sleeps between changes. A focused text caret has timed blink wakeups. Debug configuration polling wakes the CPU without submitting frames when nothing changed. Performance and screen-reader/IME behavior still need validation on your target OS and hardware; a headless render test does not establish input latency.

Read `docs/design-guidelines.md` in the source checkout for composition and interaction guidance, and `docs/components.md` for the control reference and review checklist.

## Colour schemes and the component library

`Widget::color_picker` adds horizontal or vertical ribbons and swatches with zero to three companion accents per scheme. Changes use `Value::Selection`; applications retain ownership of the palette and selected index. See [`color_picker`](crate::color_picker) for the public types and `docs/color-picker.md` in the checkout for examples and interaction guidelines.

Run `cargo run --example showcase` to explore every widget, type role, all five Pixel fonts, variable font weights, overlays, palette variants and motion. See `docs/showcase.md` for page selection and snapshot options.

## Optional interaction sound

Sound stays off until the application sets `ui.sound.mode` to `SoundMode::Essential` or `SoundMode::Subtle`. The host provides asynchronous device setup, a small synthesized click/pop family and semantic routing, while continuous controls and editing stay silent. Explicit success/failure cues come from `Ui::play_sound`, after the result is known. See [`sound`](crate::sound) for research, mappings, configuration and audio-engine integration. Audition the family with `cargo run --example showcase -- --page sound --sound`.
