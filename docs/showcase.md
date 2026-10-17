# The component library app

Run the separate interactive catalogue:

```sh
cargo run --example showcase
```

It uses the same public widgets, fonts, renderer and host as an application built with the library. Its source is `examples/showcase.rs`; `examples/demo.rs` remains the smaller application-oriented example.

| Page | What to try |
| --- | --- |
| colour | Horizontal/vertical ribbon and swatches; zero to three companions; custom and disabled palettes; apply a selected scheme |
| controls | Switches, three button styles, sliders, bipolar values, knobs, MIDI learn, editable and error inputs, dropdowns, tabs, pivots, every icon |
| typography | Every type role, Geist Sans/Mono and all five Pixel cuts; a live 100–900 weight slider and animated weight presets |
| tiles | Static and live tiles, horizontal/vertical meters, determinate/indeterminate progress, selectable and reorderable lists |
| overlays | Dialog, context menu, filtered command palette, notification, tooltip, aligned dropdown |
| motion | Interruptible switches and value changes, colour selection, page transitions, reduced-motion control and motion debug shortcuts |
| layout | Four pixel rhythm, differently sized tiles, stack/row composition and the responsive twelve-column grid |

Each page scrolls independently on entry. The top pivot includes the library's previous-page mouse affordance when an earlier title is clipped. Resize the window to inspect compact layouts. The **Dark / light** action changes the surface while keeping the selected UI accent.

```sh
cargo run --example showcase -- --page typography
cargo run --example showcase -- --light --vertical --swatches --companions 3
cargo run --example showcase -- --page controls --snapshot target/controls.png --scale 2
cargo run --example showcase -- --snapshot target/colour.png
```

`--page` accepts the names in the table. `--companions` accepts 0–3. Snapshot mode renders a deterministic 1100 × 800 logical pixel viewport using an offscreen GPU adapter; `--scale` accepts 0.5–4. Use `--width` (320–2560), `--height` (320–1600), and `--scroll` (logical pixels) to capture compact layouts or content farther down a page. The live app supports scrolling and interaction.

For motion inspection, use Ctrl/Cmd + Shift + S to slow the clock, Ctrl/Cmd + Shift + Space to pause it, and Ctrl/Cmd + Shift + . to step one frame. F9–F12 toggle grid, baselines, hit areas and timing diagnostics.

The catalogue includes a small custom text specimen. It obtains its resolved rectangle and clipping area with `Ui::widget_geometry`, paints `Primitive::Text` with a changing weight, and returns `true` from `Application::needs_frame` only while that tween is active. This is also the pattern for application-owned animations: sample the shared `ui.clock`, respect motion preferences, and stop requesting frames at rest. Keep blocking work out of `paint`, `view` and `event`.

The synth demo and MIDI controls demonstrate UI and parameter binding; neither example supplies an audio engine. To exercise MIDI learn without hardware, applications can send `Input::Midi` through the host's `InputSender`.
