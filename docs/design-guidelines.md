# Design guidelines

Use Brockmann to make content and actions legible with type, alignment and space. The visual language is Metro interaction with Swiss composition: flat rectangles, a strong left axis, a small type palette, and one accent.

## Choose the control from the task

| User intent | Control | Example |
| --- | --- | --- |
| Enable an independent persistent setting | Toggle | Audio engine, velocity sensitivity |
| Execute an action once | Button | save, load, export |
| Choose among 2–4 short values frequently | Segmented choice | mono / stereo |
| Choose a value from a longer named set | Picker | Output device, accent |
| Navigate between sections | Pivot | kick / snare / master |
| Adjust a continuous value with room for precision | Slider | level, pan |
| Adjust a value in a dense parameter panel | Knob | tone, decay |
| Display a measurement | Meter/readout | output dB, BPM |

Avoid negative switch labels such as “Disable audio”; “Off” becomes ambiguous. Avoid switches for destructive commands. Opening a picker or focusing a control should not change the application's value. Keep unavailable controls visible when their state is useful, with a disabled appearance.

## Compose before decorating

Start with a 12-column grid. Use outer margins of 24 px below 720 px, 48 px from 720 through 1280 px, and 96 px above that. Use multiples of four for spacing. Prefer 8 px between related rows and 24–48 px between larger groups.

Align page/section bounds to the grid. Repeated settings should share row bounds and control positions. A field's **internal padding** is a separate alignment rule: picker label, value and menu options all share a 16 px inset. Align the outer field bounds with neighboring rows, then preserve that inset consistently.

Do not compensate for borders with spaces in a string. Do not paint a separate selection rectangle around a built-in picker. The library provides the hover outline, keyboard focus ring and menu focus outline, with room between strokes and text.

Use an asymmetric composition: title on the main left edge, a narrower content column, and deliberate empty space. Fit dense modules to the 96 px tile grid with 8 px gaps. Separate editor panes with a hairline, rather than enclosing every area in a card.

## Use type as hierarchy

| Role | Best use |
| --- | --- |
| `Display` / `Title` | One large, light, lowercase page or section heading |
| `Heading` | Group headings and compact picker choices |
| `Value` | Switch words, field values, desktop picker choices |
| `Body` | Commands and ordinary text |
| `Label` / `Caption` | Categories, secondary information, status |
| `MonoValue` / `MonoLarge` | Parameters, units and aligned numeric data |
| `PixelDisplay` | A rare, large data readout |

Keep large text light. Try to stay within four sizes on a screen and two weights inside a component. Lowercase titles and commands; reserve uppercase for tiny category tags. Use meaningful, short labels. Empty-state copy should explain an action or an actual condition.

## Give color a job

Use `fg`, `fg2` and `fg3` for the primary, secondary and disabled hierarchy. Use accent for a committed selection, active state, a value fill or keyboard focus. Reserve danger for real destructive confirmations and overload/error states.

Avoid accent-filled decoration or large accent backgrounds. Use `on_accent` for text on a filled accent surface. Selected text on neutral surfaces uses `Theme::accent_text()`, which adjusts the shade for at least 4.5:1 contrast while retaining the original colour for fills and palette samples. The switch thumb uses the primary foreground color as a geometric indicator; it is deliberately independent of `on_accent` text contrast.

When switching dark/light themes or accents, preserve the current motion and text-scale preferences. Test both themes and at least a bright accent and a dark accent.

## Make motion continuous at both ends

Input commits domain state immediately. Motion explains the state change afterward. Never delay a command or audio update until a UI animation finishes.

Every animated value needs sensible geometry at exactly 0 and exactly 1, and immediately next to those values. A track that is 52 px at rest but 40 px at the first animation sample produces a visible jump, even if the easing curve is smooth. The switch now wipes continuously across the full track and returns its outline smoothly to rest.

Preserve the established thumb travel and duration when fixing a discontinuity. Retarget an active tween from its current state; do not restart it from the last model value. Use the shared clock, test rapid reversals, and verify reduced motion. Avoid springs, overshoot and secondary flourishes.

For desktop pickers, keep the selected option's text stationary as the menu appears. The list reveals around its selection over 220 ms. On commit, carry the chosen text into the closed value during a 200 ms collapse; keep the application event immediate. A dialog may slide; an anchored picker should preserve the pointer's relationship to the selected text.

## Separate selection, hover and focus

A committed selection is application state. Hover identifies the pointer target. Keyboard focus identifies the next keyboard action. They can coexist and should remain distinguishable.

A closed picker has a neutral outline on hover. Its text sits inside that outline with 16 px of horizontal space. Keyboard focus has an accent ring outside the control. In the open menu, accent text marks the committed option; the keyboard candidate has its own inset outline. Escape cancels navigation without committing the candidate.

Keep fitting pivot titles visible in their original positions. Only pan when needed to reveal overflow. Let users click previous titles directly and pan a narrow strip with a trackpad, wheel or pointer drag.

Do not make the menu follow the hovered row, or recentre it whenever an arrow key moves. Clamp the panel to the window and scroll its contents when necessary. Keep option order stable, especially while a menu is open. For dynamically discovered devices, preserve identity in the application and map that identity to the current option index deliberately.

## Work across input methods and sizes

Test the complete task using only a keyboard. Tab order should follow reading order. Every icon-only command needs a meaningful label; `Widget::icon_button` uses it for both accessibility and its default tooltip.

Ordinary pointer hit areas are at least 40×40 px; touch areas are at least 48×48. The dense legacy colour ribbon is a deliberate exception; offer swatches for touch use. Preserve those targets even when a glyph is smaller. Respect reduced motion and enlarged text. Stack-based layout scales its spacing; explicit rectangles need application-level reflow. A scroll viewport should reveal content rather than forcing smaller type.

Desktop pickers anchor to the selected value when space permits. Compact pickers use a full-page list. At window edges and with very long lists, containment and usable scrolling take precedence over exact anchoring.

## Review an actual screen

Before accepting a view, check:

- Shared edges, consistent four-pixel spacing, and readable type hierarchy.
- Text inset from outlines in light and dark themes, including on hover and keyboard focus.
- Switches turning On **and** Off, from both the label and the thumb, including quick reversals.
- Picker opening with the first, middle and last option selected; selection text should stay put when the panel fits.
- Picker Escape, outside click, wheel scroll, arrow navigation and edge clamping.
- 100%, 125%, 150% and 200% display scale; enlarged text and reduced motion.
- Disabled controls, accessible names and an idle window that stops submitting frames.

Remove decoration that does not help a decision or action. Keep empty space when it clarifies structure.

## Colour scheme content

Colour ribbons are content selectors: their multicolour samples do not relax the single-accent rule for UI chrome. Use paired carets outside samples, named schemes and hex values so selection never relies on hue alone. Companion squares preview application-defined palette roles; keep those meanings stable and show only the roles the user will apply. The legacy ribbon deliberately uses dense 24 px columns; use 48 px swatches when touch target size matters. Use the ribbon for a compact spectrum and swatches for separation, with the same keyboard and accessibility behavior in either orientation. See [colour picker guidelines](color-picker.md).

## Sound as confirmation

Use a small, quiet family of tactile sounds for committed local state and deliberate transitions. Do not add sounds to hover, typing, scrolling, MIDI or every continuous value change. An activation click confirms input, while completion/failure requires an explicit domain result and a visible explanation. Keep sound off by default, provide volume and essential-only modes, and preserve user choice independently of motion preferences. [Sound design and research](sound.md) documents the acoustic specification, evidence and listening checklist.

For reorderable surfaces, keep press tilt subtle and remove it during capture. Move the surface with the pointer, animate displaced siblings, and settle on release. Do not use a large perspective tilt as a substitute for movement feedback.
