//! Branchement des filtres sur carte graphique dans egui : ressources installées au démarrage
//! dans le moteur de rendu wgpu d'eframe, puis un rappel de dessin par image affichée.
//! Repli sur les filtres processeur si la carte graphique ne convient pas (OpenGL sans cible en
//! flottants, image plus grande que les textures permises, zone hors de la fenêtre).

use std::sync::Arc;

use eframe::egui;
use eframe::egui_wgpu::{self, wgpu};

use crate::gpufx::{float_format, GpuFx};
use crate::gpufx_data::Job;

/// Filtres sur carte graphique disponibles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GpuInfo {
    /// Côté maximal d'une texture.
    pub max_dim: u32,
}

/// Installe les filtres dans le moteur de rendu (rien si la carte graphique ne convient pas).
pub fn install(rs: &egui_wgpu::RenderState) -> Option<GpuInfo> {
    let float = float_format(&rs.adapter, &rs.device)?;
    let fx = GpuFx::new(&rs.device, rs.target_format, float);
    rs.renderer.write().callback_resources.insert(fx);
    Some(GpuInfo {
        max_dim: rs.device.limits().max_texture_dimension_2d,
    })
}

/// L'image `px` peut-elle être filtrée sur la carte graphique dans `rect` (points) ?
pub fn fits(
    info: &GpuInfo,
    px: (u32, u32),
    rect: egui::Rect,
    screen: egui::Rect,
    ppp: f32,
) -> bool {
    let (l, t) = ((rect.min.x * ppp).round(), (rect.min.y * ppp).round());
    let (sw, sh) = (screen.max.x * ppp, screen.max.y * ppp);
    px.0 > 0
        && px.1 > 0
        && px.0 <= info.max_dim
        && px.1 <= info.max_dim
        && l >= 0.0
        && t >= 0.0
        && l + px.0 as f32 <= sw.ceil()
        && t + px.1 as f32 <= sh.ceil()
}

/// Rappel de dessin : nouvelle image à préparer (`job`), ou la précédente redessinée.
struct FxCallback {
    job: Option<Arc<Job>>,
    px: (u32, u32),
}

impl egui_wgpu::CallbackTrait for FxCallback {
    fn prepare(
        &self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        _screen: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        if let (Some(job), Some(fx)) = (&self.job, resources.get_mut::<GpuFx>()) {
            fx.run(device, queue, encoder, job);
        }
        Vec::new()
    }

    fn paint(
        &self,
        info: egui::PaintCallbackInfo,
        rp: &mut wgpu::RenderPass<'static>,
        resources: &egui_wgpu::CallbackResources,
    ) {
        let Some(fx) = resources.get::<GpuFx>() else {
            return;
        };
        if fx.ready() != Some(self.px) {
            return;
        }
        // zone exacte en pixels : un pixel de sortie par pixel d'écran
        let ppp = info.pixels_per_point;
        let (l, t) = (
            (info.viewport.min.x * ppp).round(),
            (info.viewport.min.y * ppp).round(),
        );
        rp.set_viewport(l, t, self.px.0 as f32, self.px.1 as f32, 0.0, 1.0);
        fx.paint(rp);
    }
}

/// Forme à ajouter au peintre : dessine l'image filtrée dans `rect`. `job` : nouvelle image
/// (None : la dernière préparée, inchangée).
pub fn shape(rect: egui::Rect, px: (u32, u32), job: Option<Job>) -> egui::Shape {
    egui::Shape::Callback(egui_wgpu::Callback::new_paint_callback(
        rect,
        FxCallback {
            job: job.map(Arc::new),
            px,
        },
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_inside_screen_only() {
        let g = GpuInfo { max_dim: 8192 };
        let screen = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(2560.0, 1440.0));
        let r = egui::Rect::from_min_size(egui::pos2(100.0, 50.0), egui::vec2(1536.0, 896.0));
        assert!(fits(&g, (1536, 896), r, screen, 1.0));
        // échelle d'affichage 2 : 3072 × 1792 pixels dans un écran de 5120 × 2880
        assert!(fits(&g, (3072, 1792), r, screen, 2.0));
        assert!(!fits(
            &GpuInfo { max_dim: 2048 },
            (3072, 1792),
            r,
            screen,
            2.0
        ));
        let out = egui::Rect::from_min_size(egui::pos2(-10.0, 0.0), egui::vec2(100.0, 100.0));
        assert!(!fits(&g, (100, 100), out, screen, 1.0));
        let wide = egui::Rect::from_min_size(egui::pos2(2000.0, 0.0), egui::vec2(800.0, 100.0));
        assert!(!fits(&g, (800, 100), wide, screen, 1.0));
    }
}
