use brockmann::{
    sound::{SoundGate, synthesize},
    ui::{Key, Modifiers, PointerButton, Value},
    widget::ButtonStyle,
    *,
};
const VIEW: Rect = Rect::new(0., 0., 768., 600.);
fn enabled() -> Ui {
    let mut ui = Ui::default();
    ui.sound = SoundSettings {
        mode: SoundMode::Subtle,
        volume: 0.35,
    };
    ui
}
fn key(ui: &mut Ui, key: Key) {
    ui.input(Input::Key {
        key,
        modifiers: Modifiers::default(),
    });
}
fn click(ui: &mut Ui, point: Point) {
    ui.input(Input::Down {
        position: point,
        button: PointerButton::Primary,
        touch: false,
    });
}
#[test]
fn pcm_has_bounded_level_zero_endpoints_no_dc_and_reproducible_design() {
    for rate in [8000, 44100, 48000, 96000, 192000] {
        for cue in SoundCue::ALL {
            let pcm = synthesize(cue, rate).unwrap();
            assert_eq!(
                pcm.len(),
                (rate as f64 * cue.duration_ms() as f64 / 1000.).round() as usize
            );
            assert!(pcm.iter().all(|s| s.is_finite() && s.abs() <= 0.200001));
            assert_eq!((pcm[0], pcm[pcm.len() - 1]), (0., 0.));
            assert!(
                pcm[1].abs() < 0.008 && pcm[pcm.len() - 2].abs() < 0.008,
                "{cue:?}"
            );
            assert!(
                (pcm.iter().sum::<f32>() / pcm.len() as f32).abs() < 0.00002,
                "{cue:?}"
            );
            assert!(pcm.iter().map(|s| s * s).sum::<f32>() > 0.01);
            assert_eq!(pcm, synthesize(cue, rate).unwrap());
        }
    }
    assert!(synthesize(SoundCue::Select, 0).is_err());
    assert!(synthesize(SoundCue::Select, 384000).is_err());
}
#[test]
fn admission_drops_repeats_and_respects_global_mute_and_essential_mode() {
    let settings = SoundSettings {
        mode: SoundMode::Subtle,
        volume: 0.35,
    };
    let mut gate = SoundGate::default();
    assert!(gate.admit(SoundCue::Select, settings, 0.));
    assert!(!gate.admit(SoundCue::Select, settings, 0.099));
    assert!(!gate.admit(SoundCue::ToggleOn, settings, 0.044));
    assert!(gate.admit(SoundCue::ToggleOn, settings, 0.05));
    assert!(gate.admit(SoundCue::ToggleOff, settings, 0.10));
    assert!(!gate.admit(SoundCue::Confirm, SoundSettings::default(), 1.));
    let essential = SoundSettings {
        mode: SoundMode::Essential,
        ..settings
    };
    assert!(!gate.admit(SoundCue::Open, essential, 2.));
    assert!(gate.admit(SoundCue::Confirm, essential, 2.));
    assert!(!gate.admit(SoundCue::Reject, settings, f64::NAN));
    // A quick asynchronous failure must remain audible after an activation click.
    let mut outcomes = SoundGate::default();
    assert!(outcomes.admit(SoundCue::Activate, settings, 0.));
    assert!(outcomes.admit(SoundCue::Reject, settings, 0.015));
    assert!(!outcomes.admit(SoundCue::Reject, settings, 0.020));
    assert_eq!(
        SoundSettings {
            volume: f32::NAN,
            ..settings
        }
        .gain(),
        0.
    );
    assert_eq!(
        SoundSettings {
            volume: 2.,
            ..settings
        }
        .gain(),
        1.
    );
}
#[test]
fn only_real_switch_changes_emit_cues_and_programmatic_values_are_silent() {
    let mut ui = enabled();
    ui.reconcile(Widget::toggle("switch", "Engine", false), VIEW);
    ui.advance(1.);
    assert_eq!(ui.take_sound(), None);
    ui.focus_widget(Id::from("switch"));
    key(&mut ui, Key::Right);
    assert_eq!(ui.take_sound(), Some(SoundCue::ToggleOn));
    key(&mut ui, Key::Right);
    assert_eq!(ui.take_sound(), None);
    key(&mut ui, Key::Left);
    assert_eq!(ui.take_sound(), Some(SoundCue::ToggleOff));
    ui.reconcile(Widget::toggle("switch", "Engine", true), VIEW);
    assert_eq!(ui.take_sound(), None);
}
#[test]
fn continuous_editing_hover_wheel_midi_and_notifications_stay_silent() {
    let mut ui = enabled();
    ui.reconcile(Widget::slider("slider", "Level", 50., 0., 100., 50.), VIEW);
    click(&mut ui, Point::new(400., 40.));
    ui.input(Input::Move(Point::new(500., 40.)));
    ui.input(Input::Up(Point::new(500., 40.)));
    ui.input(Input::Wheel { delta: 1. });
    ui.input(Input::Midi { cc: 1, value: 127 });
    ui.notify("A notification alone is not an outcome");
    assert_eq!(ui.take_sound(), None);
    ui.reconcile(Widget::input("text", "Name", ""), VIEW);
    ui.focus_widget(Id::from("text"));
    ui.input(Input::Text("hello".into()));
    assert_eq!(ui.take_sound(), None);
}
#[test]
fn layer_and_selection_coalesce_and_widget_silence_is_respected() {
    let mut ui = enabled();
    ui.reconcile(
        Widget::picker("picker", "Output", vec!["one".into(), "two".into()], 0),
        VIEW,
    );
    ui.focus_widget(Id::from("picker"));
    key(&mut ui, Key::Enter);
    assert_eq!(ui.take_sound(), Some(SoundCue::Open));
    key(&mut ui, Key::Down);
    assert_eq!(ui.take_sound(), None);
    key(&mut ui, Key::Enter);
    assert_eq!(ui.take_sound(), Some(SoundCue::Select));
    ui.reconcile(
        Widget::picker("picker", "Output", vec!["one".into(), "two".into()], 1)
            .sound(SoundFeedback::Silent),
        VIEW,
    );
    ui.focus_widget(Id::from("picker"));
    key(&mut ui, Key::Enter);
    key(&mut ui, Key::Escape);
    assert_eq!(ui.take_sound(), None);
    ui.show_dialog("Problem", "Explain it", vec!["close".into()]);
    ui.play_sound(SoundCue::Reject);
    assert_eq!(ui.take_sound(), Some(SoundCue::Reject));
    ui.play_sound(SoundCue::Confirm);
    ui.sound.mode = SoundMode::Off;
    assert_eq!(ui.take_sound(), None);
    ui.sound.mode = SoundMode::Subtle;
    assert_eq!(ui.take_sound(), None);
}
#[test]
fn primary_actions_acknowledge_activation_without_claiming_success() {
    let mut ui = enabled();
    ui.reconcile(
        Widget::button("primary", "Save", ButtonStyle::Primary),
        VIEW,
    );
    ui.accessibility_activate(Id::from("primary"));
    assert_eq!(ui.take_sound(), Some(SoundCue::Activate));
    ui.reconcile(
        Widget::button("secondary", "Edit", ButtonStyle::Secondary),
        VIEW,
    );
    ui.accessibility_activate(Id::from("secondary"));
    assert_eq!(ui.take_sound(), None);
    ui.reconcile(
        Widget::button("secondary", "Edit", ButtonStyle::Secondary)
            .sound(SoundFeedback::Cue(SoundCue::Activate)),
        VIEW,
    );
    ui.accessibility_activate(Id::from("secondary"));
    assert_eq!(ui.take_sound(), Some(SoundCue::Activate));
}
#[test]
fn palette_scrubbing_is_silent_until_release_and_wheel_does_not_chatter() {
    let mut ui = enabled();
    let picker = ColorPicker::metro();
    let bounds = Rect::new(0., 0., 768., 96.);
    ui.reconcile(
        Widget::color_picker("palette", "Scheme", picker.clone(), 0).at(bounds),
        VIEW,
    );
    ui.advance(1.);
    click(&mut ui, picker.cell_bounds(bounds, 0., 1).center());
    assert_eq!(ui.take_sound(), Some(SoundCue::Select));
    let point = picker.cell_bounds(bounds, 0., 3).center();
    ui.input(Input::Move(point));
    assert_eq!(ui.take_sound(), None);
    ui.input(Input::Up(point));
    assert_eq!(ui.take_sound(), Some(SoundCue::Select));
    ui.input(Input::Wheel { delta: -1. });
    assert_eq!(ui.take_sound(), None);
    assert!(
        ui.drain_events()
            .any(|e| matches!(e, Event::Changed(_, Value::Selection(3))))
    );
}
#[test]
fn wav_export_round_trips_header_and_finite_pcm_without_a_device() {
    let path = std::env::temp_dir().join(format!("brockmann-cue-test-{}.wav", std::process::id()));
    let samples = synthesize(SoundCue::Confirm, 48000).unwrap();
    brockmann::sound::write_wav(&path, &samples, 48000).unwrap();
    let bytes = std::fs::read(&path).unwrap();
    std::fs::remove_file(path).unwrap();
    assert_eq!(&bytes[..4], b"RIFF");
    assert_eq!(&bytes[8..12], b"WAVE");
    assert_eq!(u32::from_le_bytes(bytes[24..28].try_into().unwrap()), 48000);
    assert_eq!(bytes.len(), 44 + samples.len() * 2);
    assert_eq!(i16::from_le_bytes(bytes[44..46].try_into().unwrap()), 0);
}
