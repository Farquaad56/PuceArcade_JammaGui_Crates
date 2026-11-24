//! Filtres vidéo sur carte graphique (wgpu) : textures, pipelines et passes. Les données viennent de
//! `gpufx_data::build_job` ; les shaders (`gpufx.wgsl`) refont le calcul de `postfx::render`.
//!
//! `run` encode les passes hors écran (faisceau, tube, halo) ; `paint` dessine la passe finale dans
//! la passe de rendu d'egui ; `render_offscreen` fait tout dans une texture relue (tests).

use eframe::wgpu;

use crate::gpufx_data::{uniform_wgsl, Job, TABLE_W};

pub(crate) const OFF8: wgpu::TextureFormat = wgpu::TextureFormat::Rgba8Unorm;

/// Format flottant des textures du halo utilisable sur cette carte graphique (32 bits, sinon
/// 16 bits), ou rien si aucun ne peut servir de cible : pas de filtres sur carte graphique.
pub fn float_format(adapter: &wgpu::Adapter, device: &wgpu::Device) -> Option<wgpu::TextureFormat> {
    let need = wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING;
    // hors WebGPU complet (OpenGL), les possibilités propres à l'adaptateur ne valent que si
    // l'appareil les a demandées
    let specific = adapter.get_downlevel_capabilities().is_webgpu_compliant()
        || device
            .features()
            .contains(wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES);
    [
        wgpu::TextureFormat::Rgba32Float,
        wgpu::TextureFormat::Rgba16Float,
    ]
    .into_iter()
    .find(|f| {
        let u = adapter.get_texture_format_features(*f).allowed_usages;
        u.contains(need) && (specific || *f == wgpu::TextureFormat::Rgba16Float)
    })
}

/// Textures d'une taille d'image donnée.
struct Targets {
    key: (u32, u32, u32, u32, u32, u32, usize),
    src: wgpu::Texture,
    tables: wgpu::Texture,
    t1: wgpu::TextureView,
    t2: wgpu::TextureView,
    /// Halo : image réduite et texture intermédiaire du flou (1 × 1 sans halo).
    b0: wgpu::TextureView,
    b1: wgpu::TextureView,
    groups: Groups,
}

struct Groups {
    beam: wgpu::BindGroup,
    tube: wgpu::BindGroup,
    down: wgpu::BindGroup,
    blur_h: wgpu::BindGroup,
    blur_v: wgpu::BindGroup,
    fin: wgpu::BindGroup,
}

struct Pipes {
    beam: wgpu::RenderPipeline,
    tube: wgpu::RenderPipeline,
    down: wgpu::RenderPipeline,
    blur_h: wgpu::RenderPipeline,
    blur_v: wgpu::RenderPipeline,
    fin: wgpu::RenderPipeline,
}

/// Filtres sur carte graphique pour une cible de format donné.
pub struct GpuFx {
    pipes: Pipes,
    /// Format des textures du halo.
    float: wgpu::TextureFormat,
    layout: wgpu::BindGroupLayout,
    uniform: wgpu::Buffer,
    dummy8: wgpu::TextureView,
    dummy32: wgpu::TextureView,
    targets: Option<Targets>,
    srgb: bool,
    /// Taille de l'image prête pour `paint`, s'il y en a une.
    ready: Option<(u32, u32)>,
}

pub(crate) fn tex(
    device: &wgpu::Device,
    label: &str,
    (w, h): (u32, u32),
    format: wgpu::TextureFormat,
    target: bool,
) -> wgpu::Texture {
    let mut usage = wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST;
    if target {
        usage |= wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC;
    }
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some(label),
        size: wgpu::Extent3d {
            width: w.max(1),
            height: h.max(1),
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage,
        view_formats: &[],
    })
}

pub(crate) fn view(t: &wgpu::Texture) -> wgpu::TextureView {
    t.create_view(&wgpu::TextureViewDescriptor::default())
}

fn layout_entry(binding: u32) -> wgpu::BindGroupLayoutEntry {
    wgpu::BindGroupLayoutEntry {
        binding,
        visibility: wgpu::ShaderStages::FRAGMENT,
        ty: wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Float { filterable: false },
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        },
        count: None,
    }
}

impl GpuFx {
    /// `target` : format de la cible de la passe finale (celui de la fenêtre egui).
    /// `float` : format des textures du halo (`float_format`).
    pub fn new(
        device: &wgpu::Device,
        target: wgpu::TextureFormat,
        float: wgpu::TextureFormat,
    ) -> GpuFx {
        let src = format!("{}{}", uniform_wgsl(), include_str!("gpufx.wgsl"));
        let module = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("gpufx"),
            source: wgpu::ShaderSource::Wgsl(src.into()),
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("gpufx"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                layout_entry(1),
                layout_entry(2),
                layout_entry(3),
            ],
        });
        let pl = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("gpufx"),
            bind_group_layouts: &[Some(&layout)],
            immediate_size: 0,
        });
        let pipe = |entry: &str, format: wgpu::TextureFormat| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&pl),
                vertex: wgpu::VertexState {
                    module: &module,
                    entry_point: Some("vs"),
                    compilation_options: Default::default(),
                    buffers: &[],
                },
                primitive: wgpu::PrimitiveState::default(),
                depth_stencil: None,
                multisample: wgpu::MultisampleState::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &module,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format,
                        blend: None,
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview_mask: None,
                cache: None,
            })
        };
        let pipes = Pipes {
            beam: pipe("fs_beam", OFF8),
            tube: pipe("fs_tube", OFF8),
            down: pipe("fs_down", float),
            blur_h: pipe("fs_blur_h", float),
            blur_v: pipe("fs_blur_v", float),
            fin: pipe("fs_final", target),
        };
        let uniform = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("gpufx params"),
            size: crate::gpufx_data::Params::default().bytes().len() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
        GpuFx {
            pipes,
            float,
            layout,
            uniform,
            dummy8: view(&tex(device, "gpufx dummy8", (1, 1), OFF8, false)),
            dummy32: view(&tex(device, "gpufx dummy32", (1, 1), float, false)),
            targets: None,
            srgb: target.is_srgb(),
            ready: None,
        }
    }

    fn group(
        &self,
        device: &wgpu::Device,
        tables: &wgpu::TextureView,
        a: &wgpu::TextureView,
        b: &wgpu::TextureView,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("gpufx"),
            layout: &self.layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(tables),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(a),
                },
                wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::TextureView(b),
                },
            ],
        })
    }

    /// Textures et groupes pour la taille de `job` (recréés seulement si elle change).
    fn ensure_targets(&mut self, device: &wgpu::Device, job: &Job) {
        let (bw, bh) = job.bloom.map_or((0, 0), |b| (b.bw, b.bh));
        let key = (job.sw, job.sh, job.w, job.h, bw, bh, job.tables.len());
        if self.targets.as_ref().is_some_and(|t| t.key == key) {
            return;
        }
        let src = tex(device, "gpufx src", (job.sw, job.sh), OFF8, false);
        let rows = (job.tables.len() / TABLE_W) as u32;
        let tables = tex(
            device,
            "gpufx tables",
            (TABLE_W as u32, rows),
            wgpu::TextureFormat::R32Float,
            false,
        );
        let t1 = view(&tex(device, "gpufx t1", (job.w, job.h), OFF8, true));
        let t2 = view(&tex(device, "gpufx t2", (job.w, job.h), OFF8, true));
        let (b0, b1) = if job.bloom.is_some() {
            (
                view(&tex(device, "gpufx b0", (bw, bh), self.float, true)),
                view(&tex(device, "gpufx b1", (bw, bh), self.float, true)),
            )
        } else {
            (self.dummy32.clone(), self.dummy32.clone())
        };
        let (tv, sv) = (view(&tables), view(&src));
        let groups = Groups {
            beam: self.group(device, &tv, &sv, &self.dummy32),
            tube: self.group(device, &tv, &t1, &self.dummy32),
            down: self.group(device, &tv, &t2, &self.dummy32),
            blur_h: self.group(device, &tv, &self.dummy8, &b0),
            blur_v: self.group(device, &tv, &self.dummy8, &b1),
            fin: self.group(device, &tv, &t2, &b0),
        };
        self.targets = Some(Targets {
            key,
            src,
            tables,
            t1,
            t2,
            b0,
            b1,
            groups,
        });
    }

    /// Envoie `job` et encode les passes hors écran ; `paint` peut ensuite dessiner l'image.
    pub fn run(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        enc: &mut wgpu::CommandEncoder,
        job: &Job,
    ) {
        self.ensure_targets(device, job);
        let mut params = job.params.clone();
        params.set_u("srgb_out", u32::from(self.srgb));
        queue.write_buffer(&self.uniform, 0, &params.bytes());
        let Some(t) = self.targets.as_ref() else {
            return;
        };
        write_tex(queue, &t.src, &job.src, job.sw * 4, (job.sw, job.sh));
        let tb: Vec<u8> = job.tables.iter().flat_map(|v| v.to_le_bytes()).collect();
        let rows = (job.tables.len() / TABLE_W) as u32;
        write_tex(
            queue,
            &t.tables,
            &tb,
            TABLE_W as u32 * 4,
            (TABLE_W as u32, rows),
        );
        pass(enc, &t.t1, &self.pipes.beam, &t.groups.beam);
        pass(enc, &t.t2, &self.pipes.tube, &t.groups.tube);
        if job.bloom.is_some() {
            pass(enc, &t.b0, &self.pipes.down, &t.groups.down);
            for _ in 0..3 {
                pass(enc, &t.b1, &self.pipes.blur_h, &t.groups.blur_h);
                pass(enc, &t.b0, &self.pipes.blur_v, &t.groups.blur_v);
            }
        }
        self.ready = Some((job.w, job.h));
    }

    /// Taille de l'image prête à dessiner.
    pub fn ready(&self) -> Option<(u32, u32)> {
        self.ready
    }

    /// Passe finale dans une passe de rendu dont la zone (viewport) est l'image.
    pub fn paint(&self, rp: &mut wgpu::RenderPass<'_>) {
        if let (Some(t), Some(_)) = (self.targets.as_ref(), self.ready) {
            rp.set_pipeline(&self.pipes.fin);
            rp.set_bind_group(0, &t.groups.fin, &[]);
            rp.draw(0..3, 0..1);
        }
    }
}

fn write_tex(
    queue: &wgpu::Queue,
    t: &wgpu::Texture,
    data: &[u8],
    bytes_per_row: u32,
    (w, h): (u32, u32),
) {
    queue.write_texture(
        wgpu::TexelCopyTextureInfo {
            texture: t,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        data,
        wgpu::TexelCopyBufferLayout {
            offset: 0,
            bytes_per_row: Some(bytes_per_row),
            rows_per_image: Some(h),
        },
        wgpu::Extent3d {
            width: w,
            height: h,
            depth_or_array_layers: 1,
        },
    );
}

/// Une passe plein cadre de `pipe` vers `target`.
fn pass(
    enc: &mut wgpu::CommandEncoder,
    target: &wgpu::TextureView,
    pipe: &wgpu::RenderPipeline,
    group: &wgpu::BindGroup,
) {
    let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("gpufx"),
        color_attachments: &[Some(wgpu::RenderPassColorAttachment {
            view: target,
            depth_slice: None,
            resolve_target: None,
            ops: wgpu::Operations {
                load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                store: wgpu::StoreOp::Store,
            },
        })],
        depth_stencil_attachment: None,
        timestamp_writes: None,
        occlusion_query_set: None,
        multiview_mask: None,
    });
    rp.set_pipeline(pipe);
    rp.set_bind_group(0, group, &[]);
    rp.draw(0..3, 0..1);
}
