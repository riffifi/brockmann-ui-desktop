use brockmann::{
    animation::{Curve, Scheduler, Tween},
    scene::Primitive,
    theme::{Color, TypeRole},
    ui::{Key, Modifiers, PointerButton, Value},
    widget::ButtonStyle,
    *,
};
const VIEW: Rect = Rect::new(0., 0., 640., 600.);
fn key(ui: &mut Ui, key: Key) {
    ui.input(Input::Key {
        key,
        modifiers: Modifiers::default(),
    });
}
fn down(ui: &mut Ui, x: f32, y: f32) {
    ui.input(Input::Down {
        position: Point::new(x, y),
        button: PointerButton::Primary,
        touch: false,
    });
}
fn events(ui: &mut Ui) -> Vec<Event> {
    ui.drain_events().collect()
}
#[test]
fn easing_is_monotonic_and_retarget_preserves_position_and_velocity() {
    for curve in [Curve::OutExpo, Curve::InExpo, Curve::InOut, Curve::Linear] {
        let mut previous = 0.;
        for i in 0..=1000 {
            let v = curve.sample(i as f64 / 1000.);
            assert!(v >= previous - 1e-8 && (-1e-8..=1.0000001).contains(&v));
            previous = v;
        }
    }
    let mut tween = Tween::new(0., 100., 0., 0.35, Curve::OutExpo);
    let before = tween.sample(0.12);
    tween.retarget(20., 0.12, 0.28, Curve::OutExpo);
    let after = tween.sample(0.12);
    assert!((before.0 - after.0).abs() < 1e-6);
    assert!((before.1 - after.1).abs() < 1e-5);
    assert_eq!(tween.sample(1.), (20., 0.));
}
#[test]
fn all_accents_choose_accessible_foreground() {
    for accent in Accent::ALL {
        for light in [false, true] {
            let theme = Theme::new(light, accent);
            assert!(theme.on_accent.contrast(theme.accent) >= 4.5, "{accent:?}");
            assert!(theme.fg.contrast(theme.bg) >= 4.5);
        }
    }
}
#[test]
fn snap_edges_after_scaling() {
    for scale in [1., 1.25, 1.5, 2.] {
        let r = Rect::new(4.4, 10.7, 52., 20.).snap(scale);
        assert_eq!(r.x.fract(), 0.);
        assert_eq!(r.right().fract(), 0.);
        assert_eq!(r.bottom().fract(), 0.);
    }
}
#[test]
fn toggle_commits_on_pointer_down_and_survives_reconcile() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::toggle("audio", "Audio", false).at(Rect::new(24., 24., 400., 72.)),
        VIEW,
    );
    down(&mut ui, 40., 40.);
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("audio"), Value::Bool(true))]
    );
    ui.reconcile(
        Widget::toggle("audio", "Audio", true).at(Rect::new(24., 24., 400., 72.)),
        VIEW,
    );
    ui.advance(0.2);
    let scene = ui.scene();
    assert!(
        scene
            .draws
            .iter()
            .any(|d| matches!(&d.primitive,Primitive::Text{text,..} if text=="On"))
    );
    ui.input(Input::Up(Point::new(40., 40.)));
    assert!(events(&mut ui).is_empty());
    ui.advance(1.);
    assert!(!ui.needs_frame());
}
#[test]
fn toggle_thumb_drag_captures_and_uses_midpoint() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::toggle("a", "Audio", false).at(Rect::new(0., 0., 200., 72.)),
        VIEW,
    );
    down(&mut ui, 156., 36.);
    assert!(events(&mut ui).is_empty());
    ui.advance(0.1);
    ui.input(Input::Move(Point::new(176., 36.)));
    ui.advance(0.05);
    assert!(
        ui.scene()
            .draws
            .iter()
            .any(|d| matches!(&d.primitive,Primitive::Text{text,..} if text=="On"))
    );
    ui.input(Input::Up(Point::new(250., 36.)));
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("a"), Value::Bool(true))]
    );
}
#[test]
fn disabled_widgets_do_not_receive_input_or_tab_focus() {
    let mut ui = Ui::default();
    let tree = Widget::column(
        "root",
        vec![
            Widget::toggle("a", "a", false).disabled(true),
            Widget::toggle("b", "b", false),
        ],
    )
    .at(VIEW);
    ui.reconcile(tree, VIEW);
    down(&mut ui, 20., 20.);
    assert!(events(&mut ui).is_empty());
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focus, Some(Id::from("b")));
    key(&mut ui, Key::Space);
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("b"), Value::Bool(true))]
    );
}
#[test]
fn slider_keyboard_and_capture_clamp_to_range() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::slider("level", "Level", 50., 0., 100., 25.).at(Rect::new(24., 24., 200., 64.)),
        VIEW,
    );
    ui.focus_widget(Id::from("level"));
    key(&mut ui, Key::End);
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("level"), Value::Number(100.))]
    );
    down(&mut ui, 124., 68.);
    events(&mut ui);
    ui.input(Input::Move(Point::new(-100., 68.)));
    assert!(ui.dragging());
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("level"), Value::Number(0.))]
    );
    ui.input(Input::Up(Point::new(-100., 68.)));
    assert!(!ui.dragging());
}
#[test]
fn picker_traps_focus_selects_and_restores_owner() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::picker("theme", "Theme", vec!["dark".into(), "light".into()], 0)
            .at(Rect::new(24., 24., 300., 72.)),
        VIEW,
    );
    ui.focus_widget(Id::from("theme"));
    key(&mut ui, Key::Enter);
    key(&mut ui, Key::Down);
    key(&mut ui, Key::Enter);
    let e = events(&mut ui);
    assert!(e.contains(&Event::Changed(Id::from("theme"), Value::Selection(1))));
    assert_eq!(ui.focus, Some(Id::from("theme")));
}
#[test]
fn midi_learning_maps_values_and_escape_cancels() {
    let mut ui = Ui::default();
    ui.reconcile(Widget::knob("tone", "Tone", 0., 0., 100., 0.), VIEW);
    ui.begin_midi_learn(Id::from("tone"));
    events(&mut ui);
    ui.input(Input::Midi { cc: 74, value: 127 });
    assert_eq!(
        events(&mut ui),
        vec![
            Event::MidiBound(Id::from("tone"), 74),
            Event::Changed(Id::from("tone"), Value::Number(100.))
        ]
    );
    ui.begin_midi_learn(Id::from("tone"));
    events(&mut ui);
    key(&mut ui, Key::Escape);
    ui.input(Input::Midi { cc: 20, value: 0 });
    assert!(events(&mut ui).is_empty());
}
#[test]
fn input_edits_unicode_undoes_and_reconciles_without_losing_history() {
    let mut ui = Ui::default();
    let id = Id::from("text");
    ui.reconcile(Widget::input("text", "Name", ""), VIEW);
    ui.focus_widget(id);
    ui.input(Input::Text("hé".into()));
    let mut value = String::new();
    for e in events(&mut ui) {
        if let Event::Changed(_, Value::Text(v)) = e {
            value = v;
        }
    }
    assert_eq!(value, "hé");
    ui.reconcile(Widget::input("text", "Name", value), VIEW);
    key(&mut ui, Key::Backspace);
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(id, Value::Text("h".into()))]
    );
    ui.input(Input::Key {
        key: Key::Character("z".into()),
        modifiers: Modifiers {
            control: true,
            ..Default::default()
        },
    });
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(id, Value::Text("hé".into()))]
    );
}
#[test]
fn reduced_motion_substitutes_transforms() {
    let t = Theme {
        reduce_motion: true,
        ..Default::default()
    };
    let mut ui = Ui::new(t);
    ui.reconcile(Widget::button("a", "save", ButtonStyle::Primary), VIEW);
    down(&mut ui, 10., 10.);
    ui.advance(0.08);
    assert!(ui.scene().draws.iter().all(|d| d.transform.rotate_x == 0.
        && d.transform.rotate_y == 0.
        && d.transform.scale == 1.));
    let tr = brockmann::animation::turnstile(0.5, false, false, true);
    assert_eq!(tr.x, 0.);
    assert_eq!(tr.rotate_y, 0.);
    assert_eq!(tr.opacity, 0.5);
}
#[test]
fn accesskit_exposes_names_values_states_and_bounds() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::toggle("a", "Audio engine", true).at(Rect::new(24., 24., 200., 72.)),
        VIEW,
    );
    let tree = ui.accessibility_tree(2.);
    let (_, node) = tree
        .nodes
        .iter()
        .find(|(id, _)| id.0 == Id::from("a").0)
        .unwrap();
    assert_eq!(node.label(), Some("Audio engine"));
    assert_eq!(node.toggled(), Some(accesskit::Toggled::True));
    assert_eq!(node.bounds().unwrap().x0, 48.);
}
#[test]
fn scheduler_has_deterministic_manual_clock() {
    let mut scheduler = Scheduler::default();
    scheduler.animate(1, 0., 1., 0.2, Curve::OutExpo);
    scheduler.clock.paused = true;
    scheduler.clock.advance(10.);
    assert!(scheduler.value(1, 0.).abs() < 1e-7);
    scheduler.clock.step(0.1);
    let value = scheduler.value(1, 0.);
    assert!(value > 0.5 && value < 1.);
    scheduler.clock.step(0.1);
    assert_eq!(scheduler.value(1, 0.), 1.);
    assert!(!scheduler.active());
}
#[test]
fn all_fonts_are_bundled_without_system_fallback() {
    let fs = brockmann::text::font_system();
    let families: Vec<_> = fs
        .db()
        .faces()
        .flat_map(|f| f.families.iter().map(|(name, _)| name.as_str()))
        .collect();
    assert!(families.contains(&"Geist"));
    assert!(families.contains(&"Geist Mono"));
    for cut in ["Circle", "Grid", "Line", "Square", "Triangle"] {
        assert!(
            families.contains(&format!("Geist Pixel {cut}").as_str()),
            "{families:?}"
        );
    }
    assert!(families.iter().all(|s| s.starts_with("Geist")));
    assert_eq!(TypeRole::Display.style().weight, 200);
    assert!(Color::hex(0xffffff).luminance() > 0.99);
}

#[test]
fn palette_filters_commands_and_dismisses_with_keyboard() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::button("commands", "commands", ButtonStyle::Text),
        VIEW,
    );
    ui.command_palette(vec![
        "settings".into(),
        "dark theme".into(),
        "drum synth".into(),
    ]);
    ui.input(Input::Text("drum".into()));
    key(&mut ui, Key::Enter);
    assert!(events(&mut ui).contains(&Event::Command("drum synth".into())));
    ui.advance(0.3);
    assert!(!ui.needs_frame());
}
#[test]
fn choice_options_are_individually_accessible() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::choice("tabs", vec!["tone".into(), "noise".into()], 0, false),
        VIEW,
    );
    let tree = ui.accessibility_tree(1.);
    let (id, _) = tree
        .nodes
        .iter()
        .find(|(_, n)| n.role() == accesskit::Role::Tab && n.label() == Some("noise"))
        .unwrap();
    ui.accessibility_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: *id,
        data: None,
    });
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("tabs"), Value::Selection(1))]
    );
}
#[test]
fn fine_drag_and_pointer_leave_preserve_continuity() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::slider("value", "Value", 50., 0., 100., 50.).at(Rect::new(0., 0., 200., 64.)),
        VIEW,
    );
    down(&mut ui, 100., 44.);
    events(&mut ui);
    ui.input(Input::Move(Point::new(120., 44.)));
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("value"), Value::Number(60.))]
    );
    ui.modifiers.shift = true;
    ui.input(Input::Move(Point::new(140., 44.)));
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("value"), Value::Number(61.))]
    );
    ui.input(Input::Leave);
    assert!(ui.dragging());
    assert!(events(&mut ui).is_empty());
}
#[test]
fn scrolling_moves_both_render_and_hit_bounds() {
    let tree = || {
        Widget::column(
            "scroll",
            vec![
                Widget::button("first", "first", ButtonStyle::Text).height(56.),
                Widget::button("second", "second", ButtonStyle::Text).height(56.),
                Widget::button("third", "third", ButtonStyle::Text).height(56.),
            ],
        )
        .scrollable()
        .at(Rect::new(24., 24., 200., 80.))
    };
    let mut ui = Ui::default();
    ui.reconcile(tree(), VIEW);
    assert_eq!(ui.scroll.max, 104.);
    ui.scroll.offset = 64.;
    ui.reconcile(tree(), VIEW);
    down(&mut ui, 50., 40.);
    assert_eq!(events(&mut ui), vec![Event::Click(Id::from("second"))]);
}
#[test]
fn disabling_a_container_disables_descendants() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::column("root", vec![Widget::toggle("child", "Audio", false)]).disabled(true),
        VIEW,
    );
    key(&mut ui, Key::Tab);
    assert_eq!(ui.focus, None);
    ui.accessibility_activate(Id::from("child"));
    assert!(events(&mut ui).is_empty());
}
#[test]
fn live_tiles_wait_for_interval_and_do_not_trigger_while_hovered() {
    let tile = || {
        Widget::new(
            "tile",
            "kit",
            brockmann::widget::Kind::Tile {
                value: "01".into(),
                active: false,
            },
        )
        .live_tile("02", 1.)
        .at(Rect::new(24., 24., 200., 200.))
    };
    let mut ui = Ui::default();
    ui.reconcile(tile(), VIEW);
    ui.advance(0.6);
    assert!(
        ui.scene()
            .draws
            .iter()
            .any(|d| matches!(&d.primitive,Primitive::Text{text,..} if text=="01"))
    );
    ui.input(Input::Move(Point::new(100., 100.)));
    ui.advance(0.7);
    assert!(
        ui.scene()
            .draws
            .iter()
            .any(|d| matches!(&d.primitive,Primitive::Text{text,..} if text=="01"))
    );
    ui.input(Input::Leave);
    ui.advance(0.01);
    ui.advance(0.25);
    assert!(
        ui.scene()
            .draws
            .iter()
            .any(|d| matches!(&d.primitive,Primitive::Text{text,..} if text=="02"))
    );
}
#[test]
fn sequences_stagger_and_keep_finished_values() {
    let mut s = Scheduler::default();
    s.stagger(&[(1, 0., 1.), (2, 0., 1.)], 0.2, 0.1, Curve::Linear);
    s.clock.step(0.1);
    assert!((s.value(1, 0.) - 0.5).abs() < 1e-6);
    assert_eq!(s.value(2, 0.), 0.);
    s.clock.step(0.3);
    s.finish();
    assert_eq!(s.value(1, 0.), 1.);
    assert_eq!(s.value(2, 0.), 1.);
    assert!(!s.active());
}
#[test]
fn panorama_snaps_and_disables_parallax_under_reduce_motion() {
    let mut p = brockmann::animation::Panorama::new(3, 400., 0.);
    p.drag(240., 0.1);
    assert_eq!(p.release(0., 0.1, false), 1);
    assert!(p.active(0.2));
    assert_eq!(p.content_offset(1.), -400.);
    assert_eq!(p.title_offset(1., false), -200.);
    assert_eq!(p.title_offset(1., true), 0.);
}
#[test]
fn title_measurement_uses_em_tracking() {
    let mut fonts = brockmann::text::font_system();
    let width = brockmann::text::measure(&mut fonts, "settings", TypeRole::Title, 1.);
    assert!((100. ..200.).contains(&width), "width={width}");
}
#[test]
fn pointer_opened_picker_releases_pressed_state() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::picker("theme", "Theme", vec!["dark".into(), "light".into()], 0)
            .at(Rect::new(24., 24., 200., 72.)),
        VIEW,
    );
    down(&mut ui, 40., 40.);
    key(&mut ui, Key::Escape);
    ui.advance(1.);
    assert!(!ui.needs_frame());
}

#[test]
fn toggle_thumb_stays_visible_when_accent_text_is_black() {
    let mut ui = Ui::default();
    assert_eq!(ui.theme.on_accent, Color::hex(0));
    ui.reconcile(
        Widget::toggle("toggle", "Audio", true).at(Rect::new(24., 24., 200., 72.)),
        VIEW,
    );
    assert!(ui.scene().draws.iter().any(|d|matches!(&d.primitive,Primitive::Rect{bounds,color,stroke} if bounds.w==12.&&bounds.h==28.&&*stroke==0.&&*color==ui.theme.fg)));
}
#[test]
fn long_single_line_input_scrolls_to_keep_caret_visible() {
    let mut fonts = brockmann::text::font_system();
    let mut editor = brockmann::text::TextEditor::new(&mut fonts, "", false);
    editor.resize(&mut fonts, 100., 24., 1.);
    editor.insert(
        &mut fonts,
        "a long input with enough characters to scroll",
        0.,
    );
    assert!(editor.horizontal_scroll() > 0.);
    let (x, _) = editor.cursor_position().unwrap();
    assert!((0. ..=102.).contains(&x), "caret x={x}");
}

fn text_draw<'a>(scene: &'a brockmann::scene::Scene, label: &str) -> &'a brockmann::scene::Draw {
    scene
        .draws
        .iter()
        .rev()
        .find(|d| matches!(&d.primitive,Primitive::Text{text,..} if text==label))
        .unwrap()
}
fn text_bounds(scene: &brockmann::scene::Scene, label: &str) -> Rect {
    let Primitive::Text { bounds, .. } = text_draw(scene, label).primitive else {
        unreachable!()
    };
    bounds
}
fn toggle_fill(ui: &Ui) -> f32 {
    ui.scene()
        .draws
        .iter()
        .find_map(|d| match &d.primitive {
            Primitive::Rect {
                bounds,
                color,
                stroke,
            } if bounds.h == 20. && *stroke == 0. && *color == ui.theme.accent => Some(bounds.w),
            _ => None,
        })
        .unwrap_or(0.)
}
#[test]
fn toggle_off_wipe_has_no_endpoint_caps_or_outline_snap() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::toggle("switch", "Audio", true).at(Rect::new(24., 24., 200., 72.)),
        VIEW,
    );
    ui.focus_widget(Id::from("switch"));
    key(&mut ui, Key::Left);
    assert_eq!(toggle_fill(&ui), 52.);
    ui.advance(0.000001);
    assert!(
        toggle_fill(&ui) > 51.99,
        "off must not drop a 12px cap as it begins"
    );
    ui.advance(0.179998);
    assert!(
        toggle_fill(&ui) < 0.01,
        "off must wipe through the final cap before completion"
    );
    ui.advance(0.000002);
    assert_eq!(toggle_fill(&ui), 0.);
    key(&mut ui, Key::Right);
    ui.advance(0.18);
    assert_eq!(toggle_fill(&ui), 52.);
    key(&mut ui, Key::Left);
    ui.advance(0.09);
    let outline = ui
        .scene()
        .draws
        .into_iter()
        .find_map(|d| match d.primitive {
            Primitive::Rect {
                bounds,
                color,
                stroke,
            } if bounds.w == 52. && bounds.h == 20. && stroke == 2. => Some(color),
            _ => None,
        })
        .unwrap();
    assert_ne!(
        outline, ui.theme.line_strong,
        "outline must still be blending during travel"
    );
    assert_ne!(outline, ui.theme.accent);
}
#[test]
fn picker_hover_outline_has_real_content_padding() {
    let bounds = Rect::new(48., 200., 400., 72.);
    let mut ui = Ui::new(Theme::new(true, Accent::Teal));
    ui.reconcile(
        Widget::picker("picker", "Theme", vec!["dark".into(), "light".into()], 1).at(bounds),
        Rect::new(0., 0., 1100., 800.),
    );
    ui.input(Input::Move(bounds.center()));
    let scene = ui.scene();
    assert!(
        scene
            .draws
            .iter()
            .any(|d| matches!(d.primitive,Primitive::Rect{bounds:r,stroke:2.,..} if r==bounds))
    );
    for label in ["Theme", "light"] {
        let text = text_bounds(&scene, label);
        assert_eq!(text.x - bounds.x, 16.);
        assert_eq!(bounds.right() - text.right(), 16.);
    }
}
#[test]
fn desktop_picker_opens_around_each_selected_value_without_sliding() {
    for scale in [1., 1.5] {
        for selected in 0..4 {
            let mut ui = Ui::new(Theme {
                text_scale: scale,
                ..Theme::default()
            });
            let view = Rect::new(0., 0., 1100., 900.);
            let options: Vec<String> = ["speakers", "headphones", "interface", "monitor"]
                .into_iter()
                .map(String::from)
                .collect();
            let label = options[selected].clone();
            ui.reconcile(
                Widget::picker("output", "Output", options, selected).at(Rect::new(
                    48.,
                    300.,
                    400.,
                    72. * scale,
                )),
                view,
            );
            let closed = text_bounds(&ui.scene(), &label);
            ui.focus_widget(Id::from("output"));
            key(&mut ui, Key::Enter);
            let first_frame = ui.scene();
            assert_eq!(text_bounds(&first_frame, &label), closed);
            assert_eq!(text_draw(&first_frame, &label).transform.y, 0.);
            ui.advance(0.25);
            let open = ui.scene();
            assert_eq!(text_bounds(&open, &label), closed);
            assert!(matches!(
                text_draw(&open, &label).primitive,
                Primitive::Text {
                    role: TypeRole::Value,
                    ..
                }
            ));
            // Hovering a different option must not move the panel or the selected text.
            let other = if selected == 0 {
                "headphones"
            } else {
                "speakers"
            };
            let point = text_bounds(&open, other).center();
            ui.input(Input::Move(point));
            assert_eq!(text_bounds(&ui.scene(), &label), closed);
            assert!(
                events(&mut ui).is_empty(),
                "opening/hovering may not commit a choice"
            );
        }
    }
}
#[test]
fn anchored_picker_pointer_selection_uses_the_drawn_row() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 1100., 800.);
    ui.reconcile(
        Widget::picker("theme", "Theme", vec!["dark".into(), "light".into()], 1)
            .at(Rect::new(48., 300., 400., 72.)),
        view,
    );
    ui.focus_widget(Id::from("theme"));
    key(&mut ui, Key::Enter);
    ui.advance(0.25);
    let point = text_bounds(&ui.scene(), "dark").center();
    down(&mut ui, point.x, point.y);
    assert!(events(&mut ui).contains(&Event::Changed(Id::from("theme"), Value::Selection(0))));
    assert_eq!(ui.focus, Some(Id::from("theme")));
}
#[test]
fn bounded_picker_scrolls_in_place_and_keyboard_reveals_candidate() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 900., 300.);
    ui.reconcile(
        Widget::picker(
            "device",
            "Output",
            (0..24).map(|i| format!("device {i}")).collect(),
            12,
        )
        .at(Rect::new(48., 160., 400., 72.)),
        view,
    );
    ui.focus_widget(Id::from("device"));
    key(&mut ui, Key::Enter);
    ui.advance(0.25);
    let scene = ui.scene();
    let row = text_draw(&scene, "device 12");
    let panel = row.clip;
    assert!(panel.y >= 8. && panel.bottom() <= view.bottom() - 8.);
    assert!(panel.contains(text_bounds(&scene, "device 12").center()));
    ui.input(Input::Wheel { delta: -2. });
    let scrolled = ui.scene();
    assert_eq!(text_draw(&scrolled, "device 12").clip, panel);
    assert!(text_bounds(&scrolled, "device 12").y < text_bounds(&scene, "device 12").y);
    key(&mut ui, Key::End);
    let end = ui.scene();
    assert!(panel.contains(text_bounds(&end, "device 23").center()));
    key(&mut ui, Key::Enter);
    assert!(events(&mut ui).contains(&Event::Changed(Id::from("device"), Value::Selection(23))));
}
#[test]
fn compact_picker_stays_a_bounded_full_page_list() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 400., 300.);
    ui.reconcile(
        Widget::picker(
            "output",
            "Output",
            vec!["speakers".into(), "headphones".into()],
            1,
        )
        .at(Rect::new(24., 160., 352., 72.)),
        view,
    );
    ui.focus_widget(Id::from("output"));
    key(&mut ui, Key::Enter);
    ui.advance(0.3);
    let scene = ui.scene();
    assert!(matches!(
        text_draw(&scene, "headphones").primitive,
        Primitive::Text {
            role: TypeRole::Heading,
            ..
        }
    ));
    let panel = text_draw(&scene, "headphones").clip;
    assert!(
        panel.x >= 24.
            && panel.right() <= view.right() - 24.
            && panel.bottom() <= view.bottom() - 24.
    );
    key(&mut ui, Key::Escape);
    assert!(
        !events(&mut ui)
            .iter()
            .any(|e| matches!(e, Event::Changed(..)))
    );
}

fn accessible_button_bounds(ui: &Ui, label: &str) -> Rect {
    let tree = ui.accessibility_tree(1.);
    let (_, node) = tree
        .nodes
        .iter()
        .find(|(_, n)| n.role() == accesskit::Role::Button && n.label() == Some(label))
        .unwrap();
    let b = node.bounds().unwrap();
    Rect::new(
        b.x0 as f32,
        b.y0 as f32,
        (b.x1 - b.x0) as f32,
        (b.y1 - b.y0) as f32,
    )
}
#[test]
fn screen_and_instrument_pivots_have_separate_mouse_previous_targets() {
    let view = Rect::new(0., 0., 1100., 800.);
    let mut root = Widget::column(
        "root",
        vec![
            Widget::choice(
                "pages",
                vec!["settings".into(), "drum synth".into(), "editor".into()],
                1,
                true,
            )
            .at(Rect::new(48., 64., 1052., 56.)),
            Widget::choice(
                "drums",
                vec!["kick".into(), "snare".into(), "hat".into()],
                2,
                true,
            )
            .at(Rect::new(48., 152., 1052., 56.)),
        ],
    );
    root.layout = brockmann::layout::Layout::Overlay;
    let mut ui = Ui::default();
    ui.reconcile(root, view);
    let screen_back = accessible_button_bounds(&ui, "previous: settings");
    assert!(screen_back.w >= 40. && screen_back.h >= 40.);
    assert!(screen_back.right() <= 48.);
    let point = screen_back.center();
    down(&mut ui, point.x, point.y);
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("pages"), Value::Selection(0))]
    );
    ui.input(Input::Up(point));
    let instrument_back = accessible_button_bounds(&ui, "previous: snare");
    let point = instrument_back.center();
    down(&mut ui, point.x, point.y);
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("drums"), Value::Selection(1))]
    );
}
#[test]
fn pivot_previous_keeps_title_axis_and_has_accessible_action_and_tooltip() {
    let mut ui = Ui::default();
    let bounds = Rect::new(48., 64., 800., 56.);
    ui.reconcile(
        Widget::choice(
            "pages",
            vec!["settings".into(), "synth".into(), "editor".into()],
            2,
            true,
        )
        .at(bounds),
        Rect::new(0., 0., 900., 600.),
    );
    assert_eq!(text_bounds(&ui.scene(), "editor").x, bounds.x);
    let back = accessible_button_bounds(&ui, "previous: synth");
    ui.input(Input::Move(back.center()));
    ui.advance(0.7);
    assert!(
        ui.scene()
            .draws
            .iter()
            .any(|d| matches!(&d.primitive,Primitive::Text{text,..} if text=="previous: synth"))
    );
    let tree = ui.accessibility_tree(1.);
    let (id, _) = tree
        .nodes
        .iter()
        .find(|(_, n)| n.label() == Some("previous: synth"))
        .unwrap();
    ui.accessibility_action(accesskit::ActionRequest {
        action: accesskit::Action::Click,
        target_tree: accesskit::TreeId::ROOT,
        target_node: *id,
        data: None,
    });
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("pages"), Value::Selection(1))]
    );
}
#[test]
fn compact_pivot_previous_uses_its_own_lane_without_hiding_active_title() {
    let mut ui = Ui::default();
    let bounds = Rect::new(24., 64., 376., 56.);
    ui.reconcile(
        Widget::choice(
            "tabs",
            vec!["kick".into(), "snare".into(), "hat".into()],
            1,
            true,
        )
        .at(bounds),
        Rect::new(0., 0., 400., 300.),
    );
    let back = accessible_button_bounds(&ui, "previous: kick");
    assert!(back.x >= bounds.x && back.right() <= bounds.right());
    assert!(back.w >= 40.);
    let scene = ui.scene();
    let title = text_draw(&scene, "snare");
    assert_eq!(text_bounds(&scene, "snare").x, bounds.x);
    assert!(title.clip.right() < back.x);
    let point = back.center();
    down(&mut ui, point.x, point.y);
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("tabs"), Value::Selection(0))]
    );
}
#[test]
fn first_and_disabled_pivots_do_not_respond_to_previous_mouse_control() {
    let view = Rect::new(0., 0., 900., 600.);
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::choice("tabs", vec!["kick".into(), "snare".into()], 0, true)
            .at(Rect::new(48., 64., 800., 56.)),
        view,
    );
    assert!(
        !ui.accessibility_tree(1.)
            .nodes
            .iter()
            .any(|(_, n)| n.label().is_some_and(|s| s.starts_with("previous:")))
    );
    ui.reconcile(
        Widget::choice("tabs", vec!["kick".into(), "snare".into()], 1, true)
            .disabled(true)
            .at(Rect::new(48., 64., 800., 56.)),
        view,
    );
    ui.advance(0.3);
    let point = accessible_button_bounds(&ui, "previous: kick").center();
    down(&mut ui, point.x, point.y);
    assert!(events(&mut ui).is_empty());
}
#[test]
fn desktop_picker_reveals_around_anchored_value_then_collapses_on_cancel() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 1100., 800.);
    ui.reconcile(
        Widget::picker(
            "output",
            "Output",
            vec!["speakers".into(), "headphones".into(), "interface".into()],
            1,
        )
        .at(Rect::new(48., 300., 400., 72.)),
        view,
    );
    let value = text_bounds(&ui.scene(), "headphones");
    ui.focus_widget(Id::from("output"));
    key(&mut ui, Key::Enter);
    let initial = ui.scene();
    let first_panel = text_draw(&initial, "headphones").clip;
    assert_eq!(text_bounds(&initial, "headphones"), value);
    ui.advance(0.08);
    let middle = ui.scene();
    let expanding = text_draw(&middle, "headphones").clip;
    assert!(expanding.h > first_panel.h);
    assert_eq!(text_bounds(&middle, "headphones"), value);
    ui.advance(0.2);
    let full = text_draw(&ui.scene(), "headphones").clip;
    assert!(full.h >= expanding.h);
    key(&mut ui, Key::Escape);
    let closing = ui.scene();
    assert_eq!(text_bounds(&closing, "headphones"), value);
    ui.advance(0.1);
    let shrinking = ui.scene();
    assert_eq!(text_bounds(&shrinking, "headphones"), value);
    assert!(text_draw(&shrinking, "speakers").clip.h < full.h);
    ui.advance(0.11);
    assert_eq!(text_bounds(&ui.scene(), "headphones"), value);
    assert!(
        !events(&mut ui)
            .iter()
            .any(|e| matches!(e, Event::Changed(..)))
    );
}
#[test]
fn choosing_an_option_carries_one_label_into_the_closed_value() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 1100., 800.);
    let options = vec!["speakers".into(), "headphones".into(), "interface".into()];
    let bounds = Rect::new(48., 300., 400., 72.);
    ui.reconcile(
        Widget::picker("output", "Output", options.clone(), 1).at(bounds),
        view,
    );
    let destination = text_bounds(&ui.scene(), "headphones");
    ui.focus_widget(Id::from("output"));
    key(&mut ui, Key::Enter);
    ui.advance(0.25);
    let source = text_bounds(&ui.scene(), "speakers");
    let point = source.center();
    down(&mut ui, point.x, point.y);
    assert!(events(&mut ui).contains(&Event::Changed(Id::from("output"), Value::Selection(0))));
    ui.reconcile(
        Widget::picker("output", "Output", options, 0).at(bounds),
        view,
    );
    for dt in [0., 0.04, 0.04, 0.04] {
        ui.advance(dt);
        let scene = ui.scene();
        let copies: Vec<_> = scene
            .draws
            .iter()
            .filter(|d| matches!(&d.primitive,Primitive::Text{text,..} if text=="speakers"))
            .collect();
        assert_eq!(copies.len(), 1, "no duplicate label while closing");
        let position = text_bounds(&scene, "speakers");
        assert!(position.y >= source.y && position.y <= destination.y);
    }
    ui.advance(0.09);
    assert_eq!(text_bounds(&ui.scene(), "speakers"), destination);
}
#[test]
fn reduced_motion_picker_crossfades_without_moving_selected_text() {
    let mut ui = Ui::new(Theme {
        reduce_motion: true,
        ..Default::default()
    });
    let view = Rect::new(0., 0., 1100., 800.);
    ui.reconcile(
        Widget::picker(
            "output",
            "Output",
            vec!["speakers".into(), "headphones".into()],
            1,
        )
        .at(Rect::new(48., 300., 400., 72.)),
        view,
    );
    ui.focus_widget(Id::from("output"));
    key(&mut ui, Key::Enter);
    let panel = text_draw(&ui.scene(), "headphones").clip;
    ui.advance(0.1);
    assert_eq!(text_draw(&ui.scene(), "headphones").clip, panel);
    key(&mut ui, Key::Escape);
    ui.advance(0.05);
    assert!(ui.scene().draws.iter().all(|d| d.transform.x == 0.
        && d.transform.y == 0.
        && d.transform.rotate_x == 0.
        && d.transform.rotate_y == 0.));
}

#[test]
fn picker_keyboard_focus_glides_even_after_menu_has_been_idle() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 1100., 800.);
    ui.reconcile(
        Widget::picker(
            "output",
            "Output",
            vec!["speakers".into(), "headphones".into(), "interface".into()],
            0,
        )
        .at(Rect::new(48., 300., 400., 72.)),
        view,
    );
    ui.focus_widget(Id::from("output"));
    key(&mut ui, Key::Enter);
    ui.advance(1.);
    assert!(!ui.needs_frame());
    let focus_rect = |ui: &Ui| {
        ui.scene()
            .draws
            .into_iter()
            .find_map(|d| match d.primitive {
                Primitive::Rect {
                    bounds,
                    color,
                    stroke,
                } if stroke == 2. && color == ui.theme.accent && bounds.h == 40. => Some(bounds),
                _ => None,
            })
            .unwrap()
    };
    key(&mut ui, Key::Down);
    let start = focus_rect(&ui);
    assert!(ui.needs_frame());
    ui.advance(0.06);
    let mid = focus_rect(&ui);
    assert!(mid.y > start.y && mid.y < start.y + 48.);
    ui.advance(0.09);
    let end = focus_rect(&ui);
    assert!((end.y - start.y - 48.).abs() < 0.01);
    assert!(!ui.needs_frame());
}
#[test]
fn closing_during_opening_keeps_current_panel_and_dim_layer() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 1100., 800.);
    ui.reconcile(
        Widget::picker(
            "output",
            "Output",
            vec!["speakers".into(), "headphones".into(), "interface".into()],
            1,
        )
        .at(Rect::new(48., 300., 400., 72.)),
        view,
    );
    ui.focus_widget(Id::from("output"));
    key(&mut ui, Key::Enter);
    ui.advance(0.03);
    let surface = |ui: &Ui| {
        ui.scene()
            .draws
            .into_iter()
            .find_map(|d| match d.primitive {
                Primitive::Rect {
                    bounds,
                    color,
                    stroke,
                } if stroke == 0. && color.0[..3] == ui.theme.surface.0[..3] => {
                    Some((bounds, color.0[3]))
                }
                _ => None,
            })
            .unwrap()
    };
    let dim = |ui: &Ui| {
        ui.scene()
            .draws
            .into_iter()
            .find_map(|d| match d.primitive {
                Primitive::Rect {
                    bounds,
                    color,
                    stroke,
                } if bounds == view && stroke == 0. => Some(color.0[3]),
                _ => None,
            })
            .unwrap()
    };
    let before = surface(&ui);
    let dim_before = dim(&ui);
    key(&mut ui, Key::Escape);
    let after = surface(&ui);
    assert_eq!(before, after);
    assert_eq!(dim(&ui), dim_before);
    ui.advance(0.05);
    assert!(surface(&ui).0.h < before.0.h);
    assert!(dim(&ui) < dim_before);
}

#[test]
fn touching_a_visible_pivot_chevron_does_not_move_its_target() {
    let mut ui = Ui::default();
    let view = Rect::new(0., 0., 1100., 800.);
    ui.reconcile(
        Widget::choice(
            "tabs",
            vec!["kick".into(), "snare".into(), "hat".into()],
            1,
            true,
        )
        .at(Rect::new(48., 64., 1052., 56.)),
        view,
    );
    let visible = accessible_button_bounds(&ui, "previous: kick");
    ui.input(Input::Down {
        position: visible.center(),
        button: PointerButton::Primary,
        touch: true,
    });
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("tabs"), Value::Selection(0))]
    );
}

#[test]
fn color_picker_hit_targets_and_selection_match_in_all_variants() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        for variant in [ColorPickerVariant::Ribbon, ColorPickerVariant::Swatches] {
            for count in [
                CompanionCount::None,
                CompanionCount::One,
                CompanionCount::Two,
                CompanionCount::Three,
            ] {
                let bounds = Rect::new(
                    24.,
                    24.,
                    if orientation == Orientation::Vertical {
                        160.
                    } else {
                        480.
                    },
                    if orientation == Orientation::Vertical {
                        480.
                    } else {
                        96.
                    },
                );
                let picker = ColorPicker::metro()
                    .orientation(orientation)
                    .variant(variant)
                    .companions(count);
                let mut ui = Ui::default();
                ui.reconcile(
                    Widget::color_picker("palette", "Scheme", picker.clone(), 0).at(bounds),
                    VIEW,
                );
                ui.advance(1.);
                let cell = picker.cell_bounds(bounds, 0., 3);
                assert_eq!(
                    if orientation == Orientation::Vertical {
                        cell.h
                    } else {
                        cell.w
                    },
                    48.
                );
                down(&mut ui, cell.center().x, cell.center().y);
                assert_eq!(
                    events(&mut ui),
                    vec![Event::Changed(Id::from("palette"), Value::Selection(3))]
                );
                // Selection is owned by the app and survives a keyed view rebuild.
                ui.reconcile(
                    Widget::color_picker("palette", "Scheme", picker, 3).at(bounds),
                    VIEW,
                );
                ui.input(Input::Up(cell.center()));
                ui.advance(0.3);
                let tree = ui.accessibility_tree(1.);
                assert_eq!(
                    tree.nodes
                        .iter()
                        .filter(|(_, n)| n.role() == accesskit::Role::RadioButton
                            && n.toggled() == Some(accesskit::Toggled::True))
                        .count(),
                    1
                );
                assert!(tree.nodes.iter().any(|(_, n)| {
                    n.label().is_some_and(|s| s.starts_with("emerald #008A00"))
                        && n.toggled() == Some(accesskit::Toggled::True)
                }));
            }
        }
    }
}
#[test]
fn color_picker_overflow_keyboard_wheel_and_assistive_selection_reveal_options() {
    for orientation in [Orientation::Horizontal, Orientation::Vertical] {
        let bounds = Rect::new(24., 24., 160., 168.);
        let picker = ColorPicker::metro()
            .orientation(orientation)
            .companions(CompanionCount::Three);
        let mut ui = Ui::default();
        ui.reconcile(
            Widget::color_picker("palette", "Scheme", picker.clone(), 0).at(bounds),
            VIEW,
        );
        ui.advance(1.);
        ui.focus_widget(Id::from("palette"));
        key(&mut ui, Key::End);
        assert_eq!(
            events(&mut ui),
            vec![Event::Changed(Id::from("palette"), Value::Selection(15))]
        );
        ui.advance(0.3);
        let r = picker.cell_bounds(bounds, 15., 15);
        assert!(bounds.contains(r.center()));
        let tree = ui.accessibility_tree(1.);
        let (id, node) = tree
            .nodes
            .iter()
            .find(|(_, n)| n.label().is_some_and(|s| s.starts_with("yellow #E3C800")))
            .unwrap();
        assert_eq!(node.role(), accesskit::Role::RadioButton);
        assert!(node.label().unwrap().contains("companion #FFF29B"));
        ui.accessibility_action(accesskit::ActionRequest {
            action: accesskit::Action::Focus,
            target_tree: accesskit::TreeId::ROOT,
            target_node: *id,
            data: None,
        });
        assert_eq!(
            events(&mut ui),
            vec![Event::Changed(Id::from("palette"), Value::Selection(0))]
        );
        ui.advance(0.3);
        ui.input(Input::Move(picker.cell_bounds(bounds, 0., 0).center()));
        ui.input(Input::Wheel { delta: -1. });
        assert_eq!(
            events(&mut ui),
            vec![Event::Changed(Id::from("palette"), Value::Selection(1))]
        );
        key(&mut ui, Key::Home);
        events(&mut ui);
        key(&mut ui, Key::Left);
        assert!(events(&mut ui).is_empty());
    }
}
#[test]
fn color_picker_drag_selects_without_repeated_events_and_disabled_is_inert() {
    let bounds = Rect::new(24., 24., 480., 96.);
    let picker = ColorPicker::metro();
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::color_picker("palette", "Scheme", picker.clone(), 0).at(bounds),
        VIEW,
    );
    ui.advance(1.);
    let start = picker.cell_bounds(bounds, 0., 0).center();
    down(&mut ui, start.x, start.y);
    assert!(events(&mut ui).is_empty());
    let next = picker.cell_bounds(bounds, 0., 2).center();
    ui.input(Input::Move(next));
    assert_eq!(
        events(&mut ui),
        vec![Event::Changed(Id::from("palette"), Value::Selection(2))]
    );
    ui.input(Input::Move(next));
    assert!(events(&mut ui).is_empty());
    ui.input(Input::Up(next));
    ui.reconcile(
        Widget::color_picker("palette", "Scheme", picker, 2)
            .disabled(true)
            .at(bounds),
        VIEW,
    );
    down(&mut ui, start.x, start.y);
    key(&mut ui, Key::End);
    assert!(events(&mut ui).is_empty());
    let tree = ui.accessibility_tree(1.);
    assert!(
        tree.nodes
            .iter()
            .filter(|(_, n)| n.role() == accesskit::Role::RadioButton)
            .all(|(_, n)| n.is_disabled() && !n.supports_action(accesskit::Action::Click))
    );
}
#[test]
fn color_picker_marker_retargets_without_a_position_jump_and_settles() {
    let bounds = Rect::new(0., 0., 768., 96.);
    let picker = ColorPicker::metro();
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::color_picker("palette", "Scheme", picker.clone(), 0).at(bounds),
        bounds,
    );
    ui.advance(1.);
    ui.focus_widget(Id::from("palette"));
    key(&mut ui, Key::End);
    ui.advance(0.04);
    let markers = |ui: &Ui| {
        ui.scene()
            .draws
            .into_iter()
            .filter_map(|d| {
                if let Primitive::Triangle { points, .. } = d.primitive {
                    Some(points)
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
    };
    let before = markers(&ui);
    key(&mut ui, Key::Home);
    assert_eq!(before, markers(&ui));
    assert!(ui.needs_frame());
    ui.advance(1.);
    let settled = markers(&ui);
    let mut initial = Ui::default();
    initial.reconcile(
        Widget::color_picker("palette", "Scheme", picker, 0).at(bounds),
        bounds,
    );
    initial.advance(1.);
    assert_eq!(settled, markers(&initial));
}
#[test]
fn color_picker_handles_empty_custom_and_oversized_selection_and_theme_contrast() {
    let mut ui = Ui::default();
    ui.reconcile(
        Widget::color_picker("empty", "Empty", ColorPicker::new(vec![]), usize::MAX),
        VIEW,
    );
    assert!(!ui.scene().draws.is_empty());
    assert!(events(&mut ui).is_empty());
    let schemes = ColorPicker::metro().schemes;
    for scheme in &schemes {
        for light in [false, true] {
            let theme = Theme::new(light, Accent::Teal).with_accent(scheme.primary);
            assert!(theme.on_accent.contrast(theme.accent) >= 4.5);
        }
    }
    let custom = ColorPicker::new(vec![
        ColorScheme::new("paper", Color::hex(0xffffff)).companions(&[Color::hex(0x202020)]),
    ])
    .companions(CompanionCount::Three);
    ui.reconcile(
        Widget::color_picker("custom", "Custom", custom, usize::MAX),
        VIEW,
    );
    let tree = ui.accessibility_tree(1.);
    assert!(tree.nodes.iter().any(
        |(_, n)| n.label() == Some("paper #FFFFFF, companion #202020")
            && n.toggled() == Some(accesskit::Toggled::True)
    ));
}

#[test]
fn reduced_motion_color_picker_crossfades_stationary_markers() {
    let bounds = Rect::new(0., 0., 768., 96.);
    let theme = Theme {
        reduce_motion: true,
        ..Default::default()
    };
    let mut ui = Ui::new(theme);
    ui.reconcile(
        Widget::color_picker("palette", "Scheme", ColorPicker::metro(), 0).at(bounds),
        bounds,
    );
    ui.advance(1.);
    ui.focus_widget(Id::from("palette"));
    key(&mut ui, Key::End);
    ui.advance(0.04);
    let a = ui
        .scene()
        .draws
        .into_iter()
        .filter_map(|d| {
            if let Primitive::Triangle { points, .. } = d.primitive {
                Some((points, d.transform.opacity))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    ui.advance(0.02);
    let b = ui
        .scene()
        .draws
        .into_iter()
        .filter_map(|d| {
            if let Primitive::Triangle { points, .. } = d.primitive {
                Some((points, d.transform.opacity))
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    assert!(!a.is_empty());
    assert_eq!(
        a.iter().map(|(p, _)| p).collect::<Vec<_>>(),
        b.iter().map(|(p, _)| p).collect::<Vec<_>>()
    );
    assert_ne!(a[0].1, b[0].1);
    ui.advance(0.05);
    assert!(
        ui.scene()
            .draws
            .iter()
            .filter(|d| matches!(d.primitive, Primitive::Triangle { .. }))
            .all(|d| d.transform.opacity == 1.)
    );
}
