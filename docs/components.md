# Component reference

All dimensions below are logical pixels at text scale 1. Device scaling and physical pixel snapping are the renderer's responsibility. Builder examples use stable keys. The application owns the values and consumes `Event::Changed` before rebuilding the view.

## Constructors and defaults

| Component | Constructor | Default dimensions / value event |
| --- | --- | --- |
| Toggle | `Widget::toggle(id, label, on)` | 72 px row; `Value::Bool` |
| Slider | `Widget::slider(id, label, value, min, max, default)` | 64 px row; `Value::Number` |
| Knob | `Widget::knob(id, label, value, min, max, default)` | 96×88 px layout; `Value::Number` |
| Button | `Widget::button(id, label, style)` | 40 px high; `Event::Click` |
| Text input | `Widget::input(id, label, value)` | 64 px including label; `Value::Text` |
| Picker | `Widget::picker(id, label, options, selected)` | 72 px row; `Value::Selection` |
| Choice | `Widget::choice(id, options, selected, false)` | 40 px row; `Value::Selection` |
| Pivot | `Widget::choice(id, options, selected, true)` | 56 px row; `Value::Selection` |
| Icon command | `Widget::icon_button(id, accessible_label, icon)` | 40×40 px; `Event::Click` |

`Widget::new(id, label, Kind::...)` exposes multiline input, list items, tiles, meters, progress and custom range settings. Use `.disabled(true)` for unavailable controls. Disabling a container also disables its descendants.

## Toggle

The label sits above “On” / “Off”; a 52×20 px track and 12×28 px thumb sit on the right. Press feedback widens the thumb. Pointer hover brightens the neutral outline. Keyboard focus adds the outside accent ring.

The slide is 180 ms with `Curve::OutExpo`. The fill is `track_width × normalized_progress` with no special endpoint branches. The outline mixes from its resting color to accent using that same progress. The word swap happens at the thumb's midpoint, with a short vertical cross-slide. The primary-foreground thumb remains visible independently of whether `on_accent` text is black or white. Disabled controls have no accent.

Label/row presses commit on pointer-down. Thumb presses commit on release, with drag/flick behavior. Space/Enter toggles; Left/Right sets an explicit value. Do not use the animated word as application state.

## Slider and knob

The simple builders use a step of `(max - min) / 100`. Construct the relevant `Kind` directly when you need a different step. Supply a finite range with `max > min` and keep the model value within it.

Sliders use horizontal dragging; knobs use vertical dragging over 200 px for full range. Shift applies 10% sensitivity. Sliders also support wheel input, arrows, PageUp/PageDown, Home/End and double-click reset. Knobs support double-click reset and a right-click MIDI learn action. These interactions capture the active control until release, cancellation or removal.

User values are immediate. Programmatic parameter changes are presented with a 240 ms value tween; double-click reset uses 160 ms. The application must smooth audio parameters separately. A UI tween does not provide an audio-rate ramp.

## Button and icon commands

`ButtonStyle::Primary` fills the rectangle with accent; use at most one primary command per view. `Secondary` has a 2 px outline. `Text` has no resting outline and gains an accent underline on hover/focus. Use explicit widths when command lengths must align across a group.

Button, tile and list-row press feedback uses the configured tilt. `Theme::tilt = false` disables it, as does reduced motion. Commands activate on pointer-down; keyboard activation uses Space/Enter.

Use words when they fit. Icons are limited to the embedded Phosphor set. `.expanded(open)` animates plus/caret expansion state on an icon command; ordinary arrows and carets nudge on press. Preserve accessible labels even if a tooltip is not needed for a sighted user.

## Text input

The ordinary input builder reserves a separate label above a 40 px field. For a multiline editor, construct `Kind::Input { value, multiline: true, error: None }` with a sufficient rectangle. An error string represents a real validation error and uses the danger token.

Inputs share Unicode shaping/editing, selection, IME composition, word/line movement, clipboard and undo/redo. Typing coalesces in 300 ms bursts. Long lines scroll horizontally to keep the caret visible. The caret is immediate and blinks at hard 530 ms boundaries; typing itself is not animated.

Keep the key stable while applying each text event. Replacing the key, removing the input or replacing its model text with a different value resets editor state/history. Device clipboard and native IME behavior need validation on each target desktop.

## Picker

The closed row has 16 px of horizontal content inset, a label above its current value, and a neutral 2 px hover outline. Keyboard focus is an outside accent ring, distinct from the hover outline. The inset is shared with desktop menu text; callers should not add leading spaces or draw another border.

On a desktop viewport at least 720 px wide:

1. The selected option's text anchors to the closed value's position.
2. Options use the same `Value` type role, 48 px rows, and 16 px horizontal inset.
3. The panel adds 8 px top/bottom padding and a flat surface with a hairline outline.
4. The panel reveals above/below the selected value over 220 ms, subject to window bounds. Its text stays anchored.
5. Long lists scroll inside the fixed panel. Wheel input scrolls; keyboard navigation reveals the candidate.

Below 720 px, the picker uses a compact full-page list with heading-size text. The selected value remains accent in both modes. The keyboard candidate has its own outline inset 4 px into the option row, away from the menu border and text; on desktop it glides between rows over 140 ms.

Opening emits no value change. Arrows move the candidate; Home/End jump to its endpoints. Enter/Space or a pointer selection commits `Value::Selection(index)` and closes. Escape or outside click closes without committing. Focus returns to the owner if it is still mounted.

On commit, the selected label continues from its menu row into the closed value while the panel collapses over 200 ms. Only one copy of that label is drawn during the handoff. The event is immediate; these animations affect presentation. Cancel collapses around the existing value, and a close during opening starts from the current reveal. Reduced motion uses a 100 ms opacity transition.

Keep options and their ordering stable while open. If backend choices change, resolve persistent device identities in the application before producing the next options array. The current popup is an opening-time snapshot of its options, not a live query of the backend.

## Choice and pivot

Use a segmented choice for short values; its underline travels between measured labels. Pivots use larger navigation titles. Both keep all labels in place when they fit, and only reveal necessary overflow when selecting an offscreen option. Earlier options remain clickable; there is no back chevron. Wheel, trackpad and pointer dragging pan an overflowing strip. Both report zero-based selection indices and support keyboard arrows and accessible tab options.

Changing a pivot does not automatically replace application content. Update your page model in `event`; call `ui.navigate(back)` before that update when you want the page transition. The demo shows this order.

## Lists, tiles and readouts

A `Kind::ListItem` exposes selected state and secondary text. The accent selection bar grows from the row center. Mouse drag and Alt+arrow emit `Event::Reorder { item, before }` within the same parent; `before` identifies the target item slot. Update the app order immediately. The captured row follows the pointer without tilt, neighbours reflow over 200 ms, and the row settles on release. Touch holds for 250 ms before dragging; normal swipes scroll. Tiles opt into this behavior with `.reorderable(true)`.

Tiles use flat fill/outline and text hierarchy. Compact tiles use separate value/label columns; larger tiles reserve distinct vertical slots. Set explicit heights for tiles in a horizontal row. `.live_tile(secondary, interval_seconds)` opts into a periodic flip; the interval is at least 0.5 s. A flip does not start while hovered, although one already in flight continues. Tile activation flashes the accent and decays over 300 ms.

Meters display instantaneous data with peak hold. `Kind::Meter { db, vertical }` expects dB; above 0 dB the overload segment uses danger. Progress uses an optional normalized value in `[0, 1]`; `None` produces five traveling squares. Use meters for data, rather than animating backend measurements through UI easing.

## Overlays and custom drawing

`Ui::show_dialog`, `context_menu`, `command_palette` and `notify` provide overlays. Dialog/menu/palette actions report `Event::Command(String)`; applications must route their own commands. A toast replaces the previous toast and never stacks. `.tooltip(text)` supplies delayed caption help; icon commands receive their accessible label as a default tooltip.

`Application::paint` receives the same `Scene` used by built-ins. Use its rectangle, stroke, text, arc and transform primitives with `ui.current_theme()` and `ui.clock.now()`. Custom rendering does not automatically add hit testing or accessible roles; provide those interactions deliberately. `animation::Panorama` and `animation::continuum` are controllers/helpers for application-owned page composition.

## Known scope

This is an initial toolkit. It does not open MIDI devices, implement a full code editor, provide a ready-made multi-pane scroll manager, or claim measured 144 Hz end-to-end input latency. Real clipboard, IME, screen-reader and compositor behavior needs platform testing. The embedded fonts do not cover every script. See the tests and demo before relying on a behavior in a shipped application.

## Colour picker

`Widget::color_picker(id, label, ColorPicker, selected)` presents named schemes as a ribbon or spaced swatches. `Orientation` chooses horizontal/vertical layout; `CompanionCount` chooses zero through three accent previews. It emits `Value::Selection`, supports pointer scrubbing, wheel, arrows, Home/End, and accessible radio options. Selection markers travel for 180 ms; reduced motion crossfades. See [the colour picker guide](color-picker.md) for complete examples, overflow and theme application.

The [component library app](showcase.md) demonstrates every component interactively.

## Interaction sound

Widgets use selective semantic feedback when global sound is enabled. Use `.sound(SoundFeedback::Silent)` to suppress a widget or `.sound(SoundFeedback::Cue(cue))` to override an activation cue. Global Off/Essential/Subtle modes apply throughout. Continuous controls, hover and text editing stay silent. Explicit results use `Ui::play_sound`; see [sound design and research](sound.md).
