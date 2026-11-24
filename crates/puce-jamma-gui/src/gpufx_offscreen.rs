//! Rendu complet des filtres sur carte graphique dans une texture relue (tests : comparaison avec
//! `postfx::render`).

use eframe::wgpu;

use crate::gpufx::{tex, view, GpuFx, OFF8};

/// Filtres pour `render_offscreen`.
pub fn offscreen_fx(device: &wgpu::Device, float: wgpu::TextureFormat) -> GpuFx {
    GpuFx::new(device, OFF8, float)
}
use crate::gpufx_data::Job;

/// Rendu complet dans une texture relue : pixels RGBA (tests, comparaison avec `postfx::render`).
/// `fx` : filtres créés pour une cible `OFF8` (`offscreen_fx`), réutilisables d'une image à l'autre.
pub fn render_offscreen(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    fx: &mut GpuFx,
    job: &Job,
) -> Vec<[u8; 4]> {
    let out = tex(device, "gpufx out", (job.w, job.h), OFF8, true);
    let ov = view(&out);
    let mut enc = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());
    fx.run(device, queue, &mut enc, job);
    {
        let mut rp = enc.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("gpufx out"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &ov,
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
        fx.paint(&mut rp);
    }
    let row = (job.w * 4).div_ceil(256) * 256;
    let buf = device.create_buffer(&wgpu::BufferDescriptor {
        label: Some("gpufx readback"),
        size: u64::from(row * job.h),
        usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
        mapped_at_creation: false,
    });
    enc.copy_texture_to_buffer(
        wgpu::TexelCopyTextureInfo {
            texture: &out,
            mip_level: 0,
            origin: wgpu::Origin3d::ZERO,
            aspect: wgpu::TextureAspect::All,
        },
        wgpu::TexelCopyBufferInfo {
            buffer: &buf,
            layout: wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(row),
                rows_per_image: Some(job.h),
            },
        },
        wgpu::Extent3d {
            width: job.w,
            height: job.h,
            depth_or_array_layers: 1,
        },
    );
    queue.submit([enc.finish()]);
    let slice = buf.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    let _ = device.poll(wgpu::PollType::wait_indefinitely());
    let Ok(data) = slice.get_mapped_range() else {
        return Vec::new();
    };
    let mut px = Vec::with_capacity((job.w * job.h) as usize);
    for y in 0..job.h as usize {
        let r = &data[y * row as usize..y * row as usize + job.w as usize * 4];
        px.extend(r.as_chunks::<4>().0.iter().copied());
    }
    px
}
