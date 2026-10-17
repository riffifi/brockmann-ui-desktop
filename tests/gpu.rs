use brockmann::{
    icons::Icon,
    scene::{Primitive, Scene},
    theme::{Color, TypeRole},
    *,
};
fn pixel(bytes: &[u8], stride: usize, x: usize, y: usize) -> [u8; 4] {
    bytes[y * stride + x * 4..y * stride + x * 4 + 4]
        .try_into()
        .unwrap()
}
#[test]
fn offscreen_primitives_fonts_clipping_and_scale() {
    pollster::block_on(async {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::default());
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                force_fallback_adapter: true,
                ..Default::default()
            })
            .await
            .map_err(|_| ())
            .ok();
        let Some(adapter) = adapter else {
            eprintln!("No fallback GPU adapter: skipping offscreen test");
            return;
        };
        let (device, queue) = adapter.request_device(&Default::default()).await.unwrap();
        let mut renderer = Renderer::new(device, queue, wgpu::TextureFormat::Rgba8UnormSrgb);
        for scale in [1., 1.25, 1.5, 2.] {
            let width = (400. * scale) as u32;
            let height = (240. * scale) as u32;
            let extent = wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            };
            let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Brockmann acceptance"),
                size: extent,
                mip_level_count: 1,
                sample_count: 1,
                dimension: wgpu::TextureDimension::D2,
                format: renderer.format,
                usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                view_formats: &[],
            });
            let mut scene = Scene::default();
            let clip = Rect::new(0., 0., 400., 240.);
            scene.rect(Rect::new(24., 24., 52., 20.), Accent::Teal.color(), clip);
            scene.rect(
                Rect::new(24., 80., 20., 20.),
                Color::hex(0xffffff).alpha(0.5),
                clip,
            );
            scene.rect(
                Rect::new(100., 80., 80., 20.),
                Color::hex(0xffffff),
                Rect::new(100., 80., 40., 20.),
            );
            scene.text(
                Rect::new(24., 112., 360., 80.),
                "brockmann",
                TypeRole::Display,
                Color::hex(0xffffff),
                clip,
            );
            scene.arc(
                Rect::new(280., 24., 56., 56.),
                2.356,
                4.712,
                4.,
                Accent::Teal.color(),
                clip,
            );
            Icon::Plus.paint(
                &mut scene,
                Rect::new(340., 24., 24., 24.),
                Color::hex(0xffffff),
                clip,
            );
            scene.push(
                Primitive::Text {
                    bounds: Rect::new(24., 200., 100., 24.),
                    text: "weight".into(),
                    role: TypeRole::Body,
                    color: Color::hex(0xffffff),
                    weight: Some(100),
                },
                clip,
            );
            scene.push(
                Primitive::Text {
                    bounds: Rect::new(160., 200., 100., 24.),
                    text: "weight".into(),
                    role: TypeRole::Body,
                    color: Color::hex(0xffffff),
                    weight: Some(900),
                },
                clip,
            );
            renderer
                .render(
                    &scene,
                    &texture.create_view(&Default::default()),
                    brockmann::render::RenderOptions {
                        size: [width, height],
                        scale,
                        text_scale: 1.,
                        background: Color::hex(0),
                    },
                )
                .unwrap();
            let stride = (width * 4).div_ceil(256) * 256;
            let output = renderer.device.create_buffer(&wgpu::BufferDescriptor {
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
                    buffer: &output,
                    layout: wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(stride),
                        rows_per_image: Some(height),
                    },
                },
                extent,
            );
            renderer.queue.submit(Some(encoder.finish()));
            let (send, recv) = std::sync::mpsc::channel();
            output
                .slice(..)
                .map_async(wgpu::MapMode::Read, move |r| send.send(r).unwrap());
            renderer.device.poll(wgpu::PollType::Wait).unwrap();
            recv.recv().unwrap().unwrap();
            let bytes = output.slice(..).get_mapped_range();
            let p = |x: f32, y: f32| {
                pixel(
                    &bytes,
                    stride as usize,
                    (x * scale) as usize,
                    (y * scale) as usize,
                )
            };
            assert_eq!(p(30., 30.), [0, 171, 169, 255]);
            assert_eq!(p(23., 30.), [0, 0, 0, 255]);
            assert!(
                (p(30., 90.)[0] as i32 - 188).abs() <= 1,
                "linear coverage blend"
            );
            assert_eq!(p(120., 90.), [255, 255, 255, 255]);
            assert_eq!(p(150., 90.), [0, 0, 0, 255]);
            let ink = |x0: f32, y0: f32, x1: f32, y1: f32| {
                let mut total = 0u64;
                for y in (y0 * scale) as usize..(y1 * scale) as usize {
                    for x in (x0 * scale) as usize..(x1 * scale) as usize {
                        total += pixel(&bytes, stride as usize, x, y)[0] as u64;
                    }
                }
                total
            };
            assert!(
                ink(80., 112., 380., 192.) > 10000,
                "Geist title should render"
            );
            assert!(
                ink(160., 200., 260., 224.) > ink(24., 200., 124., 224.),
                "runtime variable weight must change raster coverage"
            );
            if std::env::var_os("BROCKMANN_WRITE_SNAPSHOTS").is_some() {
                std::fs::create_dir_all("target/screenshots").unwrap();
                let file =
                    std::fs::File::create(format!("target/screenshots/primitives-{scale}.png"))
                        .unwrap();
                let mut png = png::Encoder::new(file, width, height);
                png.set_color(png::ColorType::Rgba);
                png.set_depth(png::BitDepth::Eight);
                let mut writer = png.write_header().unwrap();
                let data: Vec<u8> = bytes
                    .chunks_exact(stride as usize)
                    .flat_map(|row| row[..width as usize * 4].iter().copied())
                    .collect();
                writer.write_image_data(&data).unwrap();
            }
        }
    });
}
