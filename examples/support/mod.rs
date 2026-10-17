use brockmann::{
    Application, Id, Input, Point, Rect, Renderer, Ui,
    render::RenderOptions,
    ui::{Key, Modifiers},
};
pub struct SnapshotView<'a> {
    pub viewport: Rect,
    pub scroll: f32,
    pub drag: Option<(&'a str, &'a str)>,
}
pub async fn snapshot(
    app: &mut impl Application,
    path: &str,
    scale: f32,
    open_picker: Option<&str>,
    hover_picker: Option<&str>,
    picker_phase: Option<&str>,
    view: Option<SnapshotView<'_>>,
) -> Result<(), Box<dyn std::error::Error>> {
    let view = view.unwrap_or(SnapshotView {
        viewport: Rect::new(0., 0., 1100., 800.),
        scroll: 0.,
        drag: None,
    });
    let logical = view.viewport;
    let scroll = view.scroll;
    let mut ui = Ui::default();
    app.configure(&mut ui);
    ui.reconcile(app.view(logical), logical);
    ui.advance(1.);
    if scroll > 0. {
        ui.scroll.offset = scroll.min(ui.scroll.max);
        ui.reconcile(app.view(logical), logical);
    }
    if let Some(key) = open_picker {
        let id = Id::from(key);
        if !ui
            .accessibility_tree(1.)
            .nodes
            .iter()
            .any(|(node, _)| node.0 == id.0)
        {
            return Err(format!("unknown picker key: {key}").into());
        }
        ui.focus_widget(id);
        ui.input(Input::Key {
            key: Key::Enter,
            modifiers: Modifiers::default(),
        });
        match picker_phase.unwrap_or("settled") {
            "opening" => ui.advance(0.025),
            "closing" => {
                ui.advance(0.25);
                ui.input(Input::Key {
                    key: Key::Escape,
                    modifiers: Modifiers::default(),
                });
                ui.advance(0.07);
            }
            "selecting" => {
                ui.advance(0.25);
                ui.input(Input::Key {
                    key: Key::Down,
                    modifiers: Modifiers::default(),
                });
                ui.input(Input::Key {
                    key: Key::Enter,
                    modifiers: Modifiers::default(),
                });
                for event in ui.drain_events().collect::<Vec<_>>() {
                    app.event(event, &mut ui);
                }
                ui.reconcile(app.view(logical), logical);
                ui.advance(0.07);
            }
            "settled" => ui.advance(0.25),
            other => return Err(format!("unknown picker phase: {other}").into()),
        }
    } else if let Some(key) = hover_picker {
        let id = Id::from(key);
        let tree = ui.accessibility_tree(1.);
        let bounds = tree
            .nodes
            .iter()
            .find(|(node, _)| node.0 == id.0)
            .and_then(|(_, node)| node.bounds())
            .ok_or_else(|| format!("unknown picker key: {key}"))?;
        ui.input(Input::Move(Point::new(
            bounds.x0 as f32 + 4.,
            bounds.y0 as f32 + 4.,
        )));
        ui.advance(0.25);
    }
    if let Some((from, to)) = view.drag {
        let start = ui
            .widget_geometry(Id::from(from))
            .ok_or("unknown drag source")?
            .0
            .center();
        let end = ui
            .widget_geometry(Id::from(to))
            .ok_or("unknown drag target")?
            .0
            .center();
        ui.input(Input::Down {
            position: start,
            button: brockmann::ui::PointerButton::Primary,
            touch: false,
        });
        ui.input(Input::Move(end));
        for event in ui.drain_events().collect::<Vec<_>>() {
            app.event(event, &mut ui);
        }
        ui.reconcile(app.view(logical), logical);
        ui.advance(0.08);
    }
    let mut scene = ui.scene();
    app.paint(&ui, &mut scene);
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = instance
        .request_adapter(&wgpu::RequestAdapterOptions {
            force_fallback_adapter: true,
            ..Default::default()
        })
        .await?;
    let (device, queue) = adapter.request_device(&Default::default()).await?;
    let mut renderer = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8UnormSrgb);
    let width = (logical.w * scale) as u32;
    let height = (logical.h * scale) as u32;
    let extent = wgpu::Extent3d {
        width,
        height,
        depth_or_array_layers: 1,
    };
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Demo snapshot"),
        size: extent,
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: renderer.format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    renderer.render(
        &scene,
        &texture.create_view(&Default::default()),
        RenderOptions {
            size: [width, height],
            scale,
            text_scale: 1.,
            background: ui.theme.bg,
        },
    )?;
    let stride = (width * 4).div_ceil(256) * 256;
    let buffer = renderer.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: (stride * height) as u64,
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    let mut encoder = renderer.device.create_command_encoder(&Default::default());
    encoder.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &texture,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(stride),
                rows_per_image: Some(height),
            },
        },
        extent,
    );
    renderer.queue.submit(Some(encoder.finish()));
    let (send, receive) = std::sync::mpsc::channel();
    buffer
        .slice(..)
        .map_async(wgpu::MapMode::Read, move |r| send.send(r).unwrap());
    renderer.device.poll(wgpu::PollType::Wait)?;
    receive.recv()??;
    let data = buffer.slice(..).get_mapped_range();
    let packed: Vec<u8> = data
        .chunks_exact(stride as usize)
        .flat_map(|row| row[..width as usize * 4].iter().copied())
        .collect();
    let file = std::fs::File::create(path)?;
    let mut encoder = png::Encoder::new(file, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder.write_header()?.write_image_data(&packed)?;
    println!("wrote {path}");
    Ok(())
}
