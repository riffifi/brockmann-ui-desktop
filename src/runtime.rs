//! winit lifecycle, AccessKit bridge and damage-driven frame pacing.
use crate::{
    Event, Point, Rect, Renderer, Ui, Widget,
    scene::Scene,
    theme::{Accent, Theme},
    ui::{Input, Key, Modifiers, PointerButton},
};
use accesskit_winit::{
    Adapter, Event as AccessibilityEvent, WindowEvent as AccessibilityWindowEvent,
};
use std::{
    error::Error,
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};
use winit::{
    application::ApplicationHandler,
    event::{ElementState, Ime, MouseButton, MouseScrollDelta, TouchPhase, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key as WKey, NamedKey},
    window::{CursorIcon, Window, WindowId},
};
/// Clone this handle onto input/device threads. Audio threads should use their own
/// lock-free channel and signal the UI from a non-real-time consumer.
#[derive(Clone)]
pub struct InputSender(EventLoopProxy<HostEvent>);
impl InputSender {
    pub fn send(&self, input: Input) -> bool {
        self.0.send_event(HostEvent::Input(input)).is_ok()
    }
}
#[derive(Debug)]
enum HostEvent {
    Accessibility(AccessibilityEvent),
    Input(Input),
}
impl From<AccessibilityEvent> for HostEvent {
    fn from(event: AccessibilityEvent) -> Self {
        Self::Accessibility(event)
    }
}
pub trait Application {
    fn configure(&mut self, _ui: &mut Ui) {}
    fn connected(&mut self, _input: InputSender) {}
    /// Non-blocking consumption of app-owned parameter channels, before view rebuild.
    fn update(&mut self, _ui: &mut Ui) {}

    fn title(&self) -> &str {
        "Brockmann"
    }
    fn view(&mut self, viewport: Rect) -> Widget;
    fn event(&mut self, event: Event, ui: &mut Ui);
    /// Custom widgets use precisely the same rendering and animation primitives.
    fn paint(&mut self, _ui: &Ui, _scene: &mut Scene) {}
}
pub fn run(app: impl Application + 'static) -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::<HostEvent>::with_user_event().build()?;
    let proxy = event_loop.create_proxy();
    let mut host = Host::new(app, proxy);
    event_loop.run_app(&mut host)?;
    if let Some(error) = host.error {
        return Err(error.into());
    }
    Ok(())
}
struct WindowState {
    renderer: Renderer,
    surface: wgpu::Surface<'static>,
    config: wgpu::SurfaceConfiguration,
    accessibility: Adapter,
    window: Arc<Window>,
}
struct Host<A> {
    app: A,
    ui: Ui,
    window: Option<WindowState>,
    proxy: EventLoopProxy<HostEvent>,
    last_tick: Instant,
    modifiers: Modifiers,
    composing: bool,
    error: Option<String>,
    hot_reload: HotReload,
    cursor: Point,
    redraw: bool,
    ui_wakeup: Option<Instant>,
    trace_frames: bool,
}
impl<A: Application> Host<A> {
    fn new(app: A, proxy: EventLoopProxy<HostEvent>) -> Self {
        let mut theme = Theme::default();
        apply_os_preferences(&mut theme);
        Self {
            app,
            ui: Ui::new(theme),
            window: None,
            proxy,
            last_tick: Instant::now(),
            modifiers: Modifiers::default(),
            composing: false,
            error: None,
            hot_reload: HotReload::default(),
            cursor: Point::default(),
            redraw: true,
            ui_wakeup: None,
            trace_frames: std::env::var_os("BROCKMANN_TRACE_FRAMES").is_some(),
        }
    }
    fn rebuild(&mut self) {
        let Some(w) = &self.window else { return };
        let scale = w.window.scale_factor() as f32;
        let viewport = Rect::new(
            0.,
            0.,
            w.config.width as f32 / scale,
            w.config.height as f32 / scale,
        );
        self.app.update(&mut self.ui);
        let mut tree = self.app.view(viewport);
        self.hot_reload.apply_layout(&mut tree);
        self.ui.reconcile(tree, viewport);
    }
    fn dispatch(&mut self, input: Input) {
        self.tick();
        let restore = self.ui.drag_restore_position();
        let release = matches!(input, Input::Up(_));
        self.ui.input(input);
        self.deliver();
        self.rebuild();
        self.redraw = true;
        if let Some(w) = &mut self.window {
            w.window.set_cursor_visible(!self.ui.dragging());
            if release && let Some(position) = restore {
                let _ = w
                    .window
                    .set_cursor_position(winit::dpi::LogicalPosition::new(position.x, position.y));
            }
            w.window.set_cursor(if self.ui.text_focused() {
                CursorIcon::Text
            } else {
                CursorIcon::Default
            });
            w.window.set_ime_allowed(self.ui.text_focused());
            if let Some(caret) = self.ui.ime_cursor_rect() {
                w.window.set_ime_cursor_area(
                    winit::dpi::LogicalPosition::new(caret.x, caret.y),
                    winit::dpi::LogicalSize::new(caret.w, caret.h),
                );
            }
            w.accessibility
                .update_if_active(|| self.ui.accessibility_tree(w.window.scale_factor()));
            w.window.request_redraw();
        }
    }
    fn deliver(&mut self) {
        let events = self.ui.drain_events().collect::<Vec<_>>();
        for e in events {
            self.app.event(e, &mut self.ui);
        }
    }
    fn tick(&mut self) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_tick).as_secs_f64();
        self.last_tick = now;
        self.ui.advance(dt);
    }
    fn fail(&mut self, event_loop: &ActiveEventLoop, message: String) {
        self.error = Some(message);
        event_loop.exit();
    }
}
impl<A: Application> ApplicationHandler<HostEvent> for Host<A> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }
        let window = match event_loop.create_window(
            Window::default_attributes()
                .with_title(self.app.title())
                .with_visible(false)
                .with_inner_size(winit::dpi::LogicalSize::new(1100., 800.)),
        ) {
            Ok(w) => Arc::new(w),
            Err(e) => {
                self.fail(event_loop, e.to_string());
                return;
            }
        };
        let accessibility = Adapter::with_event_loop_proxy(event_loop, &window, self.proxy.clone());
        let gpu = pollster::block_on(async {
            let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
            let surface = instance
                .create_surface(window.clone())
                .map_err(|e| e.to_string())?;
            let adapter = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    compatible_surface: Some(&surface),
                    power_preference: wgpu::PowerPreference::LowPower,
                    force_fallback_adapter: false,
                })
                .await
                .map_err(|e| e.to_string())?;
            let (device, queue) = adapter
                .request_device(&wgpu::DeviceDescriptor {
                    label: Some("Brockmann"),
                    ..Default::default()
                })
                .await
                .map_err(|e| e.to_string())?;
            let size = window.inner_size();
            let caps = surface.get_capabilities(&adapter);
            let format = caps
                .formats
                .iter()
                .copied()
                .find(|f| f.is_srgb())
                .ok_or("No sRGB surface format")?;
            let config = wgpu::SurfaceConfiguration {
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
                format,
                width: size.width.max(1),
                height: size.height.max(1),
                present_mode: wgpu::PresentMode::Fifo,
                desired_maximum_frame_latency: 1,
                alpha_mode: caps.alpha_modes[0],
                view_formats: vec![],
            };
            surface.configure(&device, &config);
            Ok::<_, String>((Renderer::new(device, queue, format), surface, config))
        });
        match gpu {
            Ok((renderer, surface, config)) => {
                self.window = Some(WindowState {
                    renderer,
                    surface,
                    config,
                    accessibility,
                    window: window.clone(),
                });
                self.app.configure(&mut self.ui);
                self.app.connected(InputSender(self.proxy.clone()));
                self.rebuild();
                window.set_visible(true);
                window.request_redraw();
            }
            Err(e) => self.fail(event_loop, e),
        }
    }
    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        let Some(w) = self.window.as_mut() else {
            return;
        };
        w.accessibility.process_event(&w.window, &event);
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(size) => {
                if size.width > 0 && size.height > 0 {
                    w.config.width = size.width;
                    w.config.height = size.height;
                    w.surface.configure(&w.renderer.device, &w.config);
                    self.rebuild();
                    self.redraw = true;
                    self.window.as_ref().unwrap().window.request_redraw();
                }
            }
            WindowEvent::ScaleFactorChanged { .. } => {
                self.rebuild();
                self.redraw = true;
                self.window.as_ref().unwrap().window.request_redraw();
            }
            WindowEvent::RedrawRequested => {
                let started = Instant::now();
                self.tick();
                self.deliver();
                self.rebuild();
                let mut scene = self.ui.scene();
                self.app.paint(&self.ui, &mut scene);
                let w = self.window.as_mut().unwrap();
                if w.window.inner_size().width == 0 || w.window.inner_size().height == 0 {
                    return;
                }
                match w.surface.get_current_texture() {
                    Ok(frame) => {
                        let view = frame.texture.create_view(&Default::default());
                        let scale = w.window.scale_factor() as f32;
                        let theme = self.ui.current_theme();
                        if let Err(e) = w.renderer.render(
                            &scene,
                            &view,
                            crate::render::RenderOptions {
                                size: [w.config.width, w.config.height],
                                scale,
                                text_scale: theme.text_scale,
                                background: theme.bg,
                            },
                        ) {
                            self.fail(event_loop, e);
                            return;
                        }
                        w.window.pre_present_notify();
                        frame.present();
                    }
                    Err(wgpu::SurfaceError::Lost | wgpu::SurfaceError::Outdated) => {
                        w.surface.configure(&w.renderer.device, &w.config);
                        w.window.request_redraw();
                    }
                    Err(wgpu::SurfaceError::OutOfMemory) => {
                        self.fail(event_loop, "GPU ran out of memory".into());
                        return;
                    }
                    Err(_) => {}
                }
                if let Some(w) = &mut self.window {
                    w.accessibility
                        .update_if_active(|| self.ui.accessibility_tree(w.window.scale_factor()));
                }
                self.ui.frame_ms = started.elapsed().as_secs_f32() * 1000.;
                if self.trace_frames {
                    eprintln!(
                        "brockmann frame at {:.3}s: {:.2} ms",
                        self.ui.clock.now(),
                        self.ui.frame_ms
                    );
                }
                self.redraw = false;
            }
            WindowEvent::CursorMoved { position, .. } => {
                let p = position.to_logical::<f32>(w.window.scale_factor());
                self.cursor = Point::new(p.x, p.y);
                self.dispatch(Input::Move(self.cursor));
            }
            WindowEvent::CursorLeft { .. } => self.dispatch(Input::Leave),
            WindowEvent::MouseInput { state, button, .. } => {
                let button = match button {
                    MouseButton::Left => PointerButton::Primary,
                    MouseButton::Right => PointerButton::Secondary,
                    _ => return,
                };
                self.dispatch(if state == ElementState::Pressed {
                    Input::Down {
                        position: self.cursor,
                        button,
                        touch: false,
                    }
                } else {
                    Input::Up(self.cursor)
                });
            }
            WindowEvent::MouseWheel { delta, .. } => {
                let delta = match delta {
                    MouseScrollDelta::LineDelta(_, y) => y,
                    MouseScrollDelta::PixelDelta(p) => p.y as f32 / 32.,
                };
                self.dispatch(Input::Wheel { delta });
            }
            WindowEvent::ModifiersChanged(m) => {
                let m = m.state();
                self.modifiers = Modifiers {
                    shift: m.shift_key(),
                    control: m.control_key(),
                    alt: m.alt_key(),
                    command: m.super_key(),
                };
                self.ui.modifiers = self.modifiers;
            }
            WindowEvent::KeyboardInput { event, .. } if event.state == ElementState::Pressed => {
                if let Some(key) = map_key(&event.logical_key) {
                    self.dispatch(Input::Key {
                        key,
                        modifiers: self.modifiers,
                    });
                }
                if !self.composing
                    && !self.modifiers.control
                    && !self.modifiers.command
                    && !self.modifiers.alt
                    && let Some(text) = event.text
                {
                    let text: String = text.chars().filter(|c| !c.is_control()).collect();
                    if !text.is_empty() {
                        self.dispatch(Input::Text(text));
                    }
                }
            }
            WindowEvent::Ime(Ime::Preedit(text, cursor)) => {
                self.composing = !text.is_empty();
                self.dispatch(Input::ImePreedit(text, cursor));
            }
            WindowEvent::Ime(Ime::Commit(text)) => {
                self.composing = false;
                self.dispatch(Input::ImeCommit(text));
            }
            WindowEvent::Ime(Ime::Disabled) => {
                self.composing = false;
                self.dispatch(Input::ImePreedit(String::new(), None));
            }
            WindowEvent::Touch(touch) => {
                let p = touch.location.to_logical::<f32>(w.window.scale_factor());
                let point = Point::new(p.x, p.y);
                self.dispatch(match touch.phase {
                    TouchPhase::Started => Input::Down {
                        position: point,
                        button: PointerButton::Primary,
                        touch: true,
                    },
                    TouchPhase::Moved => Input::Move(point),
                    TouchPhase::Ended => Input::Up(point),
                    TouchPhase::Cancelled => Input::Cancel,
                });
            }
            WindowEvent::Focused(f) => {
                if f {
                    let mut theme = self.ui.theme.clone();
                    apply_os_preferences(&mut theme);
                    self.ui.set_theme(theme);
                }
                self.dispatch(Input::Focus(f));
            }
            _ => {}
        }
    }
    fn user_event(&mut self, _: &ActiveEventLoop, event: HostEvent) {
        let event = match event {
            HostEvent::Input(input) => {
                self.dispatch(input);
                return;
            }
            HostEvent::Accessibility(event) => event,
        };
        match event.window_event {
            AccessibilityWindowEvent::InitialTreeRequested => {
                if let Some(w) = &mut self.window {
                    w.accessibility
                        .update_if_active(|| self.ui.accessibility_tree(w.window.scale_factor()));
                }
            }
            AccessibilityWindowEvent::ActionRequested(request) => {
                self.tick();
                self.ui.accessibility_action(request);
                self.deliver();
                self.rebuild();
                if let Some(w) = &self.window {
                    w.window.request_redraw();
                }
            }
            AccessibilityWindowEvent::AccessibilityDeactivated => {}
        }
    }
    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.hot_reload.poll(&mut self.ui) {
            self.rebuild();
            self.redraw = true;
        }
        if let Some(w) = &self.window {
            if self.ui.needs_frame() || self.redraw {
                w.window.request_redraw();
                event_loop.set_control_flow(ControlFlow::Wait);
            } else {
                let caret = self.ui.next_wakeup();
                self.ui_wakeup = caret
                    .map(|seconds| Instant::now() + Duration::from_secs_f64(seconds.max(0.001)));
                let timer = if cfg!(debug_assertions) {
                    Some(caret.unwrap_or(0.5).min(0.5))
                } else {
                    caret
                };
                if let Some(seconds) = timer {
                    event_loop.set_control_flow(ControlFlow::WaitUntil(
                        Instant::now() + Duration::from_secs_f64(seconds.max(0.001)),
                    ));
                } else {
                    event_loop.set_control_flow(ControlFlow::Wait);
                }
            }
        }
    }
    fn new_events(&mut self, _: &ActiveEventLoop, cause: winit::event::StartCause) {
        if matches!(cause, winit::event::StartCause::ResumeTimeReached { .. }) {
            self.tick();
            if self
                .ui_wakeup
                .is_some_and(|deadline| Instant::now() >= deadline)
            {
                self.redraw = true;
                self.ui_wakeup = None;
            }
        }
    }
}
fn map_key(key: &WKey) -> Option<Key> {
    Some(match key {
        WKey::Character(c) if c.as_str() == " " => Key::Space,
        WKey::Character(c) => Key::Character(c.to_string()),
        WKey::Named(k) => match k {
            NamedKey::Tab => Key::Tab,
            NamedKey::Space => Key::Space,
            NamedKey::Enter => Key::Enter,
            NamedKey::Escape => Key::Escape,
            NamedKey::ArrowLeft => Key::Left,
            NamedKey::ArrowRight => Key::Right,
            NamedKey::ArrowUp => Key::Up,
            NamedKey::ArrowDown => Key::Down,
            NamedKey::Home => Key::Home,
            NamedKey::End => Key::End,
            NamedKey::PageUp => Key::PageUp,
            NamedKey::PageDown => Key::PageDown,
            NamedKey::Backspace => Key::Backspace,
            NamedKey::Delete => Key::Delete,
            NamedKey::F9 => Key::F9,
            NamedKey::F10 => Key::F10,
            NamedKey::F11 => Key::F11,
            NamedKey::F12 => Key::F12,
            _ => return None,
        },
        _ => return None,
    })
}
/// Read native desktop preferences at startup and when returning to the app.
/// Environment overrides also support headless sessions and desktop environments
/// that do not expose text scaling or motion settings through a common API.
pub fn apply_os_preferences(theme: &mut Theme) {
    fn query(cmd: &str, args: &[&str]) -> Option<String> {
        let output = std::process::Command::new(cmd).args(args).output().ok()?;
        if !output.status.success() {
            return None;
        }
        Some(String::from_utf8_lossy(&output.stdout).trim().into())
    }
    #[cfg(target_os = "linux")]
    {
        if let Some(value) = query(
            "gsettings",
            &["get", "org.gnome.desktop.interface", "enable-animations"],
        ) {
            theme.reduce_motion = value == "false";
        }
        if let Some(value) = query(
            "gsettings",
            &["get", "org.gnome.desktop.interface", "text-scaling-factor"],
        ) && let Ok(n) = value.parse::<f32>()
        {
            theme.text_scale = n.clamp(0.5, 3.);
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Some(value) = query(
            "defaults",
            &["read", "com.apple.universalaccess", "reduceMotion"],
        ) {
            theme.reduce_motion = value == "1";
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(value) = query(
            "reg",
            &[
                "query",
                r"HKCU\Control Panel\Desktop\WindowMetrics",
                "/v",
                "MinAnimate",
            ],
        ) {
            theme.reduce_motion = value.split_whitespace().last() == Some("0");
        }
        if let Some(value) = query(
            "reg",
            &[
                "query",
                r"HKCU\Software\Microsoft\Accessibility",
                "/v",
                "TextScaleFactor",
            ],
        ) {
            if let Some(n) = value
                .split_whitespace()
                .last()
                .and_then(|s| u32::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            {
                theme.text_scale = (n as f32 / 100.).clamp(0.5, 3.);
            }
        }
    }
    if let Ok(value) = std::env::var("BROCKMANN_REDUCE_MOTION") {
        theme.reduce_motion = value == "1" || value == "true";
    }
    if let Ok(value) = std::env::var("BROCKMANN_TEXT_SCALE")
        && let Ok(n) = value.parse::<f32>()
        && n.is_finite()
    {
        theme.text_scale = n.clamp(0.5, 3.);
    }
}
#[derive(Default, serde::Deserialize)]
struct LocalConfig {
    light: Option<bool>,
    accent: Option<String>,
    reduce_motion: Option<bool>,
    tilt: Option<bool>,
    text_scale: Option<f32>,
    #[serde(default)]
    layout: std::collections::BTreeMap<String, [f32; 4]>,
}
struct HotReload {
    path: PathBuf,
    last_poll: Instant,
    modified: Option<std::time::SystemTime>,
    config: LocalConfig,
}
impl Default for HotReload {
    fn default() -> Self {
        Self {
            path: PathBuf::from("brockmann.local.toml"),
            last_poll: Instant::now() - Duration::from_secs(1),
            modified: None,
            config: LocalConfig::default(),
        }
    }
}
impl HotReload {
    fn poll(&mut self, ui: &mut Ui) -> bool {
        if !cfg!(debug_assertions) || self.last_poll.elapsed() < Duration::from_millis(500) {
            return false;
        }
        self.last_poll = Instant::now();
        let modified = std::fs::metadata(&self.path)
            .and_then(|m| m.modified())
            .ok();
        if modified == self.modified {
            return false;
        }
        self.modified = modified;
        let Some(source) = std::fs::read_to_string(&self.path).ok() else {
            return false;
        };
        match toml::from_str::<LocalConfig>(&source) {
            Ok(c) => {
                if c.text_scale.is_some_and(|v| !v.is_finite() || v <= 0.)
                    || c.layout
                        .values()
                        .any(|r| r.iter().any(|v| !v.is_finite()) || r[2] < 0. || r[3] < 0.)
                    || c.accent
                        .as_ref()
                        .is_some_and(|name| !Accent::ALL.iter().any(|a| a.name() == name))
                {
                    ui.notify("theme: invalid accent, scale or layout rectangle");
                    return true;
                }
                let accent = c
                    .accent
                    .as_ref()
                    .and_then(|name| Accent::ALL.into_iter().find(|a| a.name() == name))
                    .unwrap_or_else(|| {
                        Accent::ALL
                            .into_iter()
                            .find(|a| a.color() == ui.theme.accent)
                            .unwrap_or(Accent::Teal)
                    });
                let mut theme = Theme::new(c.light.unwrap_or(ui.theme.light), accent);
                theme.reduce_motion = c.reduce_motion.unwrap_or(ui.theme.reduce_motion);
                theme.tilt = c.tilt.unwrap_or(ui.theme.tilt);
                theme.text_scale = c.text_scale.unwrap_or(ui.theme.text_scale).clamp(0.5, 3.);
                ui.set_theme(theme);
                self.config = c;
                true
            }
            Err(e) => {
                ui.notify(format!("theme: {e}"));
                true
            }
        }
    }
    fn apply_layout(&self, w: &mut Widget) {
        if let Some(r) = self.config.layout.get(&w.label).or_else(|| {
            self.config
                .layout
                .iter()
                .find(|(key, _)| crate::Id::from(key.as_str()) == w.id)
                .map(|(_, v)| v)
        }) {
            w.bounds = Some(Rect::new(r[0], r[1], r[2].max(0.), r[3].max(0.)));
        }
        for c in &mut w.children {
            self.apply_layout(c);
        }
    }
}
