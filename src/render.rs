//! Instanced quads, analytic arcs, tessellated icons and grayscale glyph coverage.
//! All primitives, including glyphs, share transforms, clipping and painter order.
use crate::{
    scene::{Primitive, Scene},
    theme::Color,
};
use cosmic_text::{
    Attrs, Buffer, CacheKey, Family, FontSystem, Metrics, Shaping, SwashCache, SwashContent, Weight,
};
use std::{collections::HashMap, sync::Arc};
use wgpu::util::DeviceExt;
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Instance {
    rect: [f32; 4],
    color: [f32; 4],
    params: [f32; 4],
    clip: [f32; 4],
    movement: [f32; 4],
    rotation: [f32; 4],
    origin: [f32; 4],
    uv: [f32; 4],
    extra: [f32; 4],
}
#[derive(Clone, Copy)]
struct Glyph {
    uv: [f32; 4],
    left: i32,
    top: i32,
    width: u32,
    height: u32,
}
const ATLAS: u32 = 2048;
#[derive(Clone, Copy, Debug)]
pub struct RenderOptions {
    pub size: [u32; 2],
    pub scale: f32,
    pub text_scale: f32,
    pub background: Color,
}
pub struct Renderer {
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub format: wgpu::TextureFormat,
    pipeline: wgpu::RenderPipeline,
    layout: wgpu::BindGroupLayout,
    screen: wgpu::Buffer,
    atlas: wgpu::Texture,
    sampler: wgpu::Sampler,
    font_system: FontSystem,
    swash: SwashCache,
    glyphs: HashMap<CacheKey, Glyph>,
    buffers: HashMap<String, Arc<Buffer>>,
    pen: (u32, u32, u32),
    instances: wgpu::Buffer,
    capacity: usize,
    bind: wgpu::BindGroup,
    pub last_instances: usize,
}
impl Renderer {
    pub fn new(device: wgpu::Device, queue: wgpu::Queue, format: wgpu::TextureFormat) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Brockmann primitives"),
            source: wgpu::ShaderSource::Wgsl(include_str!("primitives.wgsl").into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Brockmann scene"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX_FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 3,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: None,
            bind_group_layouts: &[&layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Brockmann flat primitives"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vs"),
                compilation_options: Default::default(),
                buffers: &[],
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fs"),
                compilation_options: Default::default(),
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
            }),
            primitive: wgpu::PrimitiveState::default(),
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
            cache: None,
        });
        let atlas = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Geist coverage atlas"),
            size: wgpu::Extent3d {
                width: ATLAS,
                height: ATLAS,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let screen = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Viewport"),
            contents: bytemuck::cast_slice(&[1f32; 4]),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });
        let capacity = 256;
        let instances = Self::instance_buffer(&device, capacity);
        let bind = Self::bind(&device, &layout, &instances, &screen, &atlas, &sampler);
        Self {
            device,
            queue,
            format,
            pipeline,
            layout,
            screen,
            atlas,
            sampler,
            font_system: crate::text::font_system(),
            swash: SwashCache::new(),
            glyphs: HashMap::new(),
            buffers: HashMap::new(),
            pen: (1, 1, 0),
            instances,
            capacity,
            bind,
            last_instances: 0,
        }
    }
    fn instance_buffer(device: &wgpu::Device, n: usize) -> wgpu::Buffer {
        device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Instances"),
            size: (n * std::mem::size_of::<Instance>()) as u64,
            usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        })
    }
    fn bind(
        d: &wgpu::Device,
        l: &wgpu::BindGroupLayout,
        b: &wgpu::Buffer,
        s: &wgpu::Buffer,
        a: &wgpu::Texture,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        d.create_bind_group(&wgpu::BindGroupDescriptor {
            label: None,
            layout: l,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: b.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: s.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(
                        &a.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }
    fn glyph(&mut self, key: CacheKey) -> Result<Option<Glyph>, String> {
        if let Some(g) = self.glyphs.get(&key) {
            return Ok(Some(*g));
        }
        let Some(image) = self.swash.get_image(&mut self.font_system, key).as_ref() else {
            return Ok(None);
        };
        let w = image.placement.width;
        let h = image.placement.height;
        if w == 0 || h == 0 {
            return Ok(None);
        }
        if w + 2 >= ATLAS || h + 2 >= ATLAS {
            return Err("Glyph exceeds coverage atlas".into());
        }
        if self.pen.0 + w + 1 >= ATLAS {
            self.pen.0 = 1;
            self.pen.1 += self.pen.2 + 2;
            self.pen.2 = 0;
        }
        if self.pen.1 + h + 1 >= ATLAS {
            return Err("Glyph atlas full".into());
        }
        let (x, y, _) = self.pen;
        let data = match image.content {
            SwashContent::Mask => image.data.clone(),
            SwashContent::Color => image.data.chunks_exact(4).map(|c| c[3]).collect(),
            SwashContent::SubpixelMask => image
                .data
                .chunks_exact(4)
                .map(|c| ((c[0] as u32 + c[1] as u32 + c[2] as u32) / 3) as u8)
                .collect(),
        };
        self.queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.atlas,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            &data,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(w),
                rows_per_image: Some(h),
            },
            wgpu::Extent3d {
                width: w,
                height: h,
                depth_or_array_layers: 1,
            },
        );
        let g = Glyph {
            uv: [
                x as f32 / ATLAS as f32,
                y as f32 / ATLAS as f32,
                (x + w) as f32 / ATLAS as f32,
                (y + h) as f32 / ATLAS as f32,
            ],
            left: image.placement.left,
            top: image.placement.top,
            width: w,
            height: h,
        };
        self.pen.0 += w + 2;
        self.pen.2 = self.pen.2.max(h);
        self.glyphs.insert(key, g);
        Ok(Some(g))
    }
    fn prepare(
        &mut self,
        scene: &Scene,
        scale: f32,
        text_scale: f32,
    ) -> Result<Vec<Instance>, String> {
        let mut out = vec![];
        for d in &scene.draws {
            let clip = d.clip.snap(scale);
            if clip.w <= 0. || clip.h <= 0. {
                continue;
            }
            let t = d.transform;
            let mut instance = Instance {
                rect: [0.; 4],
                color: [0.; 4],
                params: [0.; 4],
                clip: [clip.x, clip.y, clip.right(), clip.bottom()],
                movement: [t.x * scale, t.y * scale, t.scale, t.opacity],
                rotation: [
                    t.rotate_x.to_radians(),
                    t.rotate_y.to_radians(),
                    t.perspective * scale,
                    t.rotate_z.to_radians(),
                ],
                origin: [d.origin.x * scale, d.origin.y * scale, 0., 0.],
                uv: [0.; 4],
                extra: [0.; 4],
            };
            let rect = |r: crate::Rect| {
                let r = r.snap(scale);
                [r.x, r.y, r.w, r.h]
            };
            match &d.primitive {
                Primitive::Rect {
                    bounds,
                    color,
                    stroke,
                } => {
                    instance.rect = rect(*bounds);
                    instance.color = color.linear();
                    instance.params = [
                        0.,
                        if *stroke == 1. {
                            1.
                        } else {
                            (stroke * scale).round()
                        },
                        0.,
                        0.,
                    ];
                    out.push(instance);
                }
                Primitive::Arc {
                    bounds,
                    color,
                    start,
                    sweep,
                    thickness,
                } => {
                    if *sweep <= 0. {
                        continue;
                    }
                    instance.rect = rect(*bounds);
                    instance.color = color.linear();
                    instance.params = [
                        1.,
                        thickness * scale,
                        *start,
                        sweep.min(std::f32::consts::TAU),
                    ];
                    out.push(instance);
                }
                Primitive::Line {
                    from,
                    to,
                    width,
                    color,
                } => {
                    instance.rect = [from.x * scale, from.y * scale, to.x * scale, to.y * scale];
                    instance.color = color.linear();
                    instance.params = [2., width * scale, 0., 0.];
                    out.push(instance);
                }
                Primitive::Triangle { points, color } => {
                    instance.rect = [
                        points[0].x * scale,
                        points[0].y * scale,
                        points[1].x * scale,
                        points[1].y * scale,
                    ];
                    instance.color = color.linear();
                    instance.params = [4., 0., points[2].x * scale, points[2].y * scale];
                    out.push(instance);
                }
                Primitive::ShapedText {
                    bounds,
                    buffer,
                    color,
                } => {
                    let text_clip = clip.intersect(bounds.snap(scale));
                    instance.clip = [
                        text_clip.x,
                        text_clip.y,
                        text_clip.right(),
                        text_clip.bottom(),
                    ];
                    instance.color = color.linear();
                    instance.params[0] = 3.;
                    for run in buffer.layout_runs() {
                        for glyph in run.glyphs {
                            let physical = glyph.physical(
                                (
                                    (bounds.x - buffer.scroll().horizontal) * scale,
                                    (bounds.y + run.line_y) * scale,
                                ),
                                scale,
                            );
                            if let Some(g) = self.glyph(physical.cache_key)? {
                                let mut i = instance;
                                i.rect = [
                                    (physical.x + g.left) as f32,
                                    (physical.y - g.top) as f32,
                                    g.width as f32,
                                    g.height as f32,
                                ];
                                i.uv = g.uv;
                                out.push(i);
                            }
                        }
                    }
                }
                Primitive::Text {
                    bounds,
                    text,
                    role,
                    color,
                    weight,
                } => {
                    let style = role.style();
                    let factor = scale * text_scale;
                    let key = format!(
                        "{}|{}|{}|{}|{}|{}|{}",
                        text,
                        style.family,
                        style.size,
                        weight.unwrap_or(style.weight),
                        bounds.w,
                        bounds.h,
                        factor
                    );
                    let buffer = if let Some(b) = self.buffers.get(&key) {
                        b.clone()
                    } else {
                        let mut b = Buffer::new(
                            &mut self.font_system,
                            Metrics::new(style.size * factor, style.line * factor),
                        );
                        b.set_size(Some(bounds.w * scale), Some(bounds.h * scale));
                        b.set_text(
                            text,
                            &Attrs::new()
                                .family(Family::Name(style.family))
                                .weight(Weight(weight.unwrap_or(style.weight)))
                                .letter_spacing(style.tracking),
                            Shaping::Advanced,
                            None,
                        );
                        b.shape_until_scroll(&mut self.font_system, false);
                        let b = Arc::new(b);
                        if self.buffers.len() > 512 {
                            self.buffers.clear();
                        }
                        self.buffers.insert(key, b.clone());
                        b
                    };
                    let text_clip = clip.intersect(bounds.snap(scale));
                    instance.clip = [
                        text_clip.x,
                        text_clip.y,
                        text_clip.right(),
                        text_clip.bottom(),
                    ];
                    instance.color = color.linear();
                    instance.params[0] = 3.;
                    for run in buffer.layout_runs() {
                        for glyph in run.glyphs {
                            let physical = glyph
                                .physical((bounds.x * scale, bounds.y * scale + run.line_y), 1.);
                            if let Some(g) = self.glyph(physical.cache_key)? {
                                let mut i = instance;
                                i.rect = [
                                    (physical.x + g.left) as f32,
                                    (physical.y - g.top) as f32,
                                    g.width as f32,
                                    g.height as f32,
                                ];
                                i.uv = g.uv;
                                out.push(i);
                            }
                        }
                    }
                }
            }
        }
        Ok(out)
    }
    /// Render into a window or application-owned texture. Idle callers do no GPU work.
    pub fn render(
        &mut self,
        scene: &Scene,
        view: &wgpu::TextureView,
        options: RenderOptions,
    ) -> Result<(), String> {
        let RenderOptions {
            size: [width, height],
            scale,
            text_scale,
            background,
        } = options;
        let items = match self.prepare(scene, scale, text_scale) {
            Ok(v) => v,
            Err(_) => {
                self.glyphs.clear();
                self.pen = (1, 1, 0);
                self.queue.write_texture(
                    wgpu::TexelCopyTextureInfo {
                        texture: &self.atlas,
                        mip_level: 0,
                        origin: wgpu::Origin3d::ZERO,
                        aspect: wgpu::TextureAspect::All,
                    },
                    &vec![0; (ATLAS * ATLAS) as usize],
                    wgpu::TexelCopyBufferLayout {
                        offset: 0,
                        bytes_per_row: Some(ATLAS),
                        rows_per_image: Some(ATLAS),
                    },
                    wgpu::Extent3d {
                        width: ATLAS,
                        height: ATLAS,
                        depth_or_array_layers: 1,
                    },
                );
                self.prepare(scene, scale, text_scale)?
            }
        };
        if items.len() > self.capacity {
            self.capacity = items.len().next_power_of_two();
            self.instances = Self::instance_buffer(&self.device, self.capacity);
            self.bind = Self::bind(
                &self.device,
                &self.layout,
                &self.instances,
                &self.screen,
                &self.atlas,
                &self.sampler,
            );
        }
        self.last_instances = items.len();
        if !items.is_empty() {
            self.queue
                .write_buffer(&self.instances, 0, bytemuck::cast_slice(&items));
        }
        self.queue.write_buffer(
            &self.screen,
            0,
            bytemuck::cast_slice(&[width as f32, height as f32, 0., 0.]),
        );
        let mut encoder = self.device.create_command_encoder(&Default::default());
        let c = background.linear();
        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Brockmann frame"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: c[0] as f64,
                            g: c[1] as f64,
                            b: c[2] as f64,
                            a: c[3] as f64,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind, &[]);
            pass.draw(0..6, 0..items.len() as u32);
        }
        self.queue.submit(Some(encoder.finish()));
        Ok(())
    }
}
