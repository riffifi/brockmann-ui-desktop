# Brockmann

A native Rust UI toolkit implementing the Metro / Swiss design and motion specification in this repository. Uses wgpu, winit, cosmic-text and AccessKit. Geist fonts and the permitted Phosphor icons are embedded; it needs no runtime asset downloads or system fonts.

```sh
cargo run --example demo
cargo run --example demo -- --page synth
cargo run --example demo -- --page editor
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
```

Read the [application guide](docs/guide.md) for a complete example, state ownership, layout, switches, anchored pickers and worker integration. The [design guidelines](docs/design-guidelines.md) cover control choice, spacing, type, motion and review. The [component reference](docs/components.md) records dimensions, events, keyboard behavior and current limits. The guide's Rust examples are checked by `cargo test --doc`.

The demo contains the settings screen, an interactive drum synth **UI stub** (no audio engine), and an editable code pane. Linux needs a Wayland or X11 session and Vulkan or OpenGL support. Windows uses wgpu's native backend; macOS uses Metal. Rust 2024 edition is required. Native execution has been checked on Linux; macOS and Windows require platform testing.

## Application model

Implement `Application::view` to return keyed `Widget` descriptions and `Application::event` to update your own state. `run` hosts the window, input, accessibility adapter and renderer. `Application::connected` supplies an `InputSender` for MIDI/device threads; `Input::Refresh` wakes the host and `Application::update` can nonblockingly read application-owned channels. Audio threads should publish to the app’s lock-free channel, with a non-real-time consumer signaling the UI.

```rust
use brockmann::{Application, Event, Id, Rect, Ui, Widget, run};
use brockmann::ui::Value;

#[derive(Default)]
struct Settings { audio: bool }
impl Application for Settings {
    fn view(&mut self, viewport: Rect) -> Widget {
        Widget::toggle("audio", "Audio engine", self.audio)
            .at(Rect::new(48., 144., (viewport.w - 96.).max(80.), 72.))
    }
    fn event(&mut self, event: Event, _: &mut Ui) {
        if let Event::Changed(id, Value::Bool(on)) = event {
            if id == Id::from("audio") { self.audio = on; }
        }
    }
}
fn main() -> Result<(), Box<dyn std::error::Error>> { run(Settings::default()) }
```

Keys must be unique and stable across view rebuilds. Zero is reserved for the accessibility root. Removed keys release focus, dragging, text history and animation state. Disabled containers disable their descendants. Applications consume changes before the next view reconciliation.

`Widget::column`, `row`, `Layout::Tiles`, `Grid::span` and `Grid::tile` provide composition. `.at` uses absolute logical coordinates; stack dimensions, gaps and padding follow theme text scaling. `.scrollable()` creates a clipped vertical viewport (one per `Ui`). Custom applications can supply a separate `Ui` for each independent scroll surface. The renderer snaps edges after display scaling, with hairlines fixed at one physical pixel.

## Components and input

Built-ins include toggles, sliders, knobs, three button styles, single and multiline text inputs, list items, anchored/full-screen pickers, pivots, segmented choices, tiles, meters, progress, and the closed icon set. `Ui` exposes dialogs, context menus, replacement toasts, delayed tooltips and MIDI learn. Build less common components with `Widget::new` and `Kind`.

- Tab / Shift+Tab moves focus; arrows operate controls and choice groups. Pivots also show a mouse-accessible previous chevron when earlier sections are cropped. Sliders support PageUp/PageDown, Home/End, wheel, Shift for fine control and double-click reset.
- Toggle rows activate on pointer-down. Thumb drags commit on release using midpoint or flick direction. State words change during travel.
- Knobs use vertical dragging, Shift for fine adjustment, double-click reset, and a right-click MIDI learn menu. Feed controller messages as `Input::Midi { cc, value }` when embedding `Ui` in your own host. The library does not open MIDI devices or own audio channels.
- Text fields use shared Unicode shaping/editing, selection, word and line motion, IME composition, clipboard and undo/redo. Typing bursts coalesce for 300 ms. The editor demo shares this implementation.
- Escape cancels MIDI learn or dismisses a popup. List reorder emits `Event::Reorder`; the application owns list order. Alt+Up/Down provides keyboard reorder.
- AccessKit exports names, bounds, values, disabled/selected/toggled states and actions. Popups isolate the accessibility tree while open.

All rendering properties share the `Ui::clock` time source. `Tween::retarget` preserves position and velocity; curves are monotonic cubic Béziers without springs. Page turnstiles, feathering, press tilt, value tweens and focus movement are interruptible. Live tiles can opt into `.live_tile(secondary_value, interval_seconds)`; a flip does not start while hovered. `animation::Panorama` provides horizontal drag/flick snapping and a half-speed title offset. `animation::continuum` exposes the shared-element geometry interpolation for app-owned list-to-detail navigation. `Application::paint` accepts the same `Scene` primitives used by built-ins. A text primitive can override weight at runtime. All five Pixel cuts have explicit family aliases (`Geist Pixel Square`, `Circle`, `Grid`, `Line`, `Triangle`).

The host sleeps when nothing changes. Active animations use FIFO presentation at display refresh; focused text carets use timed wakeups between hard blink edges. Debug theme polling wakes the CPU every 500 ms without submitting GPU work unless configuration changed. OS motion/text preferences are read at startup and on focus (GNOME settings on Linux, native preferences on Windows/macOS). `BROCKMANN_REDUCE_MOTION=1` and `BROCKMANN_TEXT_SCALE=1.5` override desktop preferences.

## Debugging and snapshots

`BROCKMANN_TRACE_FRAMES=1 cargo run --example demo` logs actual rendered frames for idle/damage checks.

F9: columns; F10: baselines; F11: hit areas; F12: frame timing. Ctrl+Shift+S toggles 10× slow motion. Set `ui.clock.paused` and call `ui.clock.step(1.0 / 144.0)` to frame-step deterministically.

In debug builds, `brockmann.local.toml` hot reloads theme and explicit layout bounds. Invalid files produce a toast and preserve the previous configuration. Example:

```toml
light = false
accent = "teal"
reduce_motion = false
tilt = true
text_scale = 1.0

[layout]
settings = [320, 160, 640, 480]
```

Generate screenshots using a headless GPU adapter:

```sh
cargo run --example demo -- --snapshot target/settings.png
cargo run --example demo -- --light --hover-picker theme --snapshot target/picker-hover.png
cargo run --example demo -- --light --open-picker theme --snapshot target/picker-open.png
cargo run --example demo -- --open-picker accent --picker-phase selecting --snapshot target/picker-selecting.png
cargo run --example demo -- --page synth --instrument snare --snapshot target/synth-snare.png
cargo run --example demo -- --page synth --scale 2 --snapshot target/synth.png
BROCKMANN_WRITE_SNAPSHOTS=1 cargo test --test gpu
```

The GPU test checks sRGB output with linear coverage blending, clipping, runtime variable weight, and 100/125/150/200% scaling. It skips explicitly if no fallback adapter exists. Behavioral tests cover animation continuity, tokens, focus, disabled state, toggle drag/activation, range input, picker focus, Unicode history, font isolation, reduced motion and MIDI learning.

## Current boundaries

This is an initial toolkit implementation. The provided spec has no reference screenshot, so visual comparison is against its geometry and tokens. A full production code editor and audio synthesis are application concerns. Native screen-reader/IME testing on all three operating systems, cursor restoration under Wayland compositor restrictions, and measured 144 Hz latency still need hardware validation. Optional live-tile flipping and panorama parallax are not enabled in the demo. Clipboard access depends on the desktop clipboard service. Geist-only fallback intentionally cannot cover scripts absent from the bundled fonts.

Fonts retain their SIL OFL license in `assets/fonts/OFL.txt`. Icon sources retain the MIT license in `assets/icons/LICENSE`. See `LICENSE` for toolkit code.

Run the complete interactive component catalogue with `cargo run --example showcase`. It includes colour ribbons and swatches, all controls, typography, overlays and motion examples. See [the showcase guide](docs/showcase.md) and [colour picker usage](docs/color-picker.md).

Optional, research-informed UI sound uses a quiet synthesized click/pop family with selective routing and user-controlled mute/volume. Audition it with `cargo run --example showcase -- --page sound --sound`; see [sound design and research](docs/sound.md). The default `sound` feature adds CPAL output; `--no-default-features` keeps synthesis/export and removes native playback.
