# Colour schemes and the ribbon picker

The Windows 8 inspired ribbon presents a sequence of named colour schemes. Each scheme has one primary colour and zero to three companion colours. A continuous ribbon resembles the reference; spaced swatches give individual schemes more separation. Both work horizontally and vertically, with the same interaction model.

The small squares preview companions. This is Brockmann's interpretation of the reference: applications supply their meaning and values. They might represent a background, a secondary accent and a highlight. Choosing a scheme selects the whole set. A picker with no companions still selects a primary colour; “none” does not mean a transparent colour or no selection.

## Add a picker

```rust
use brockmann::{ColorPicker, CompanionCount, Widget};

let palette = ColorPicker::metro().companions(CompanionCount::Two);
let picker = Widget::color_picker("scheme", "Colour scheme", palette, 4);
```

`metro()` contains sixteen named schemes with authored light, deep and contrasting companions. It is a Brockmann palette, not an import of Windows settings. The zero-based selection index is controlled by the application, just like a dropdown. Keep IDs and scheme ordering stable while the control is mounted.

```rust
use brockmann::{ColorPicker, Event, Id};
use brockmann::ui::Value;

let event = Event::Changed(Id::from("scheme"), Value::Selection(4));
let mut selected = 0;
if let Event::Changed(id, Value::Selection(index)) = event {
    if id == Id::from("scheme") {
        selected = index;
        let palette = ColorPicker::metro();
        let scheme = palette.selected(selected).unwrap();
        // Apply scheme.primary and the enabled companions to your domain model.
        assert_eq!(scheme.name, "teal");
    }
}
assert_eq!(selected, 4);
```

Pass the updated index to the next view. Changes commit immediately; the paired selection carets follow with an interruptible 180 ms transition. Programmatic model changes use the toolkit's 240 ms value transition. The shared reduced-motion path uses a 100 ms crossfade in place of travel. The component becomes idle when the transition finishes.

## Layout and variants

```rust
use brockmann::{ColorPicker, ColorPickerVariant, CompanionCount, Orientation, Widget};

let vertical = ColorPicker::metro()
    .orientation(Orientation::Vertical)
    .variant(ColorPickerVariant::Swatches)
    .companions(CompanionCount::Three);
let picker = Widget::color_picker("scheme-vertical", "Scheme", vertical, 4)
    .width(240.0)
    .height(360.0);
```

| Option | Use |
| --- | --- |
| `Orientation::Horizontal` | Settings pages, broad panels and compact tool areas |
| `Orientation::Vertical` | Side panels and adjacent content previews |
| `ColorPickerVariant::Ribbon` | A continuous band, closest to the reference |
| `ColorPickerVariant::Swatches` | Separate blocks with neutral hairlines |
| `CompanionCount::None / One / Two / Three` | How many companion slots to show |

Horizontal controls default to 96 logical pixels high. Vertical controls default to 360; allow at least 128 pixels of width for three companions and markers, and more for a full header. The header includes the label, selected name and primary hex value. Leave this height intact; avoid squeezing the control down to the visual colour band's height.

Each scheme occupies a 48 pixel hit target along the strip. Sixteen schemes need 768 pixels to be visible together. In a smaller viewport the strip follows the selection, keeping the active option visible. Users can reach the rest with the wheel, keyboard or assistive actions. Pointer dragging scrubs visible schemes; it does not implement inertial scrolling. Choose a smaller custom palette for a compact touch panel where seeing every option at once matters.

The carets sit outside the colour samples, so selection remains visible for light, dark and low-contrast samples without changing their appearance. A normal keyboard focus outline surrounds the control. Colour values are content: they stay the same in light and dark themes. Disabled samples are muted.

## Supply your own schemes

```rust
use brockmann::{ColorPicker, ColorPickerVariant, ColorScheme, CompanionCount, Widget};
use brockmann::theme::Color;

let palette = ColorPicker::new(vec![
    ColorScheme::new("newsprint", Color::hex(0xe6ddc6))
        .companions(&[Color::hex(0x202020), Color::hex(0xb5432e)]),
    ColorScheme::new("blueprint", Color::hex(0x194b80))
        .companions(&[Color::hex(0xeaf0eb)]),
    ColorScheme::new("graphite", Color::hex(0x34363a)),
])
.variant(ColorPickerVariant::Swatches)
.companions(CompanionCount::Two);
let picker = Widget::color_picker("paper", "Paper & ink", palette, 0);
```

`ColorScheme::companions` accepts at most three colours. Missing slots stay empty; the control never invents extra colours. Each slot is independent, so a companion can be a different hue rather than a tint. Use meaningful, unique names, and keep slot meanings consistent throughout a palette. Avoid visually identical schemes unless their names and companions distinguish them.

An empty palette renders a named, disabled empty state. Construction clamps an out-of-range selected index to the last available scheme. `selected(index)` also clamps and returns `None` for an empty palette. `color_picker::hex` formats an sRGB colour for display.

## Apply a theme colour deliberately

A colour picker can edit document, instrument or visualisation colours without recolouring the interface. The showcase previews selections and applies them to the chrome only when **Apply to interface** is pressed.

```rust
use brockmann::{Accent, Theme};
use brockmann::theme::Color;

let theme = Theme::new(false, Accent::Teal).with_accent(Color::hex(0xf0a30a));
assert!(theme.on_accent.contrast(theme.accent) >= 4.5);
```

`with_accent` sets the primary UI accent, its pressed shade and a black or white foreground with sufficient contrast. Use opaque sRGB colours. When replacing a theme in an application, preserve `text_scale`, `reduce_motion` and `tilt` preferences before calling `Ui::set_theme`.

Keep the interface's single accent principle: companions are palette data, not a reason to give unrelated controls different highlight colours. Apply extra colours only to content that benefits from them, such as a chart or artwork. If your product needs transparency or an explicit “no colour” option, model and label that choice separately.

## Input and accessibility

- Click or tap a scheme to select it. Drag across the visible strip to scrub.
- Tab focuses the picker; arrow keys move one scheme. Home and End select the first and last.
- Wheel over the picker moves one scheme per wheel event and reveals overflow.
- Hover shows the scheme name and hex value in a tooltip.
- AccessKit exposes a radio group with a named radio option for every scheme, its primary and visible companion values, and the checked state. Focusing or activating an offscreen option selects and reveals it.
- Disabled pickers ignore input and expose disabled options.

Do not rely on colour recognition alone. Keep names and values visible, preserve keyboard access, and give schemes useful names rather than labels such as “colour 1”.
