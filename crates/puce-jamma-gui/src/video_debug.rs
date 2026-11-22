//! Onglets « Registres » (groupes `registers` du DebugSnapshot) et « Vidéo » (chaque `DebugImage` en
//! texture, zoom 1 à 4, couches visibles -> `Cmd::LayerMask`). Ergonomie reprise de
//! ref/puce16-neogeo/ng-gui/video_debug.rs, sans connaissance d'une puce précise.

use eframe::egui;
use puce_jamma::debug::DebugImage;

use crate::debugger::DebugAction;
use crate::emu_thread::DebugSnapshot;

/// Convertit une image de débogage (0x00RRGGBB) en image egui opaque.
pub fn to_color_image(img: &DebugImage) -> egui::ColorImage {
    let (w, h) = (img.width as usize, img.height as usize);
    let pixels = img
        .pixels
        .iter()
        .take(w * h)
        .map(|p| egui::Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, *p as u8))
        .collect();
    egui::ColorImage {
        size: [w, h],
        source_size: egui::vec2(w as f32, h as f32),
        pixels,
    }
}

/// Masque des couches cochées (bit n = couche n visible).
pub fn layer_mask(visible: &[bool]) -> u32 {
    visible
        .iter()
        .enumerate()
        .filter(|(_, v)| **v)
        .fold(0, |m, (i, _)| m | (1 << i))
}

pub struct VideoDebug {
    zoom: u32,
    textures: Vec<egui::TextureHandle>,
    textures_gen: u64,
    layers: Vec<bool>,
}

impl Default for VideoDebug {
    fn default() -> Self {
        VideoDebug {
            zoom: 2,
            textures: Vec::new(),
            textures_gen: u64::MAX,
            layers: Vec::new(),
        }
    }
}

impl VideoDebug {
    pub fn registers_ui(&mut self, ui: &mut egui::Ui, s: &DebugSnapshot) {
        egui::ScrollArea::vertical().show(ui, |ui| {
            for (g, (group, regs)) in s.registers.iter().enumerate() {
                ui.collapsing(group, |ui| {
                    egui::Grid::new(("regs", g))
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for (name, value) in regs {
                                ui.monospace(name);
                                ui.monospace(value);
                                ui.end_row();
                            }
                        });
                });
            }
        });
    }

    /// Images et couches ; renvoie `LayerMask` quand une case change.
    pub fn video_ui(
        &mut self,
        ui: &mut egui::Ui,
        s: &DebugSnapshot,
        generation: u64,
    ) -> Option<DebugAction> {
        if self.textures_gen != generation {
            self.textures = s
                .images
                .iter()
                .enumerate()
                .map(|(i, img)| {
                    ui.ctx().load_texture(
                        format!("dbg_img_{i}"),
                        to_color_image(img),
                        egui::TextureOptions::NEAREST,
                    )
                })
                .collect();
            self.textures_gen = generation;
        }
        if self.layers.len() != s.layer_names.len() {
            self.layers = vec![true; s.layer_names.len()];
        }
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            ui.label("Couches");
            for (i, name) in s.layer_names.iter().enumerate() {
                if ui.checkbox(&mut self.layers[i], name).changed() {
                    action = Some(DebugAction::LayerMask(layer_mask(&self.layers)));
                }
            }
        });
        ui.add(egui::Slider::new(&mut self.zoom, 1..=4).text("Zoom"));
        egui::ScrollArea::both().show(ui, |ui| {
            for (img, tex) in s.images.iter().zip(&self.textures) {
                ui.label(format!("{} ({}×{})", img.name, img.width, img.height));
                let size = egui::vec2(
                    (img.width * self.zoom) as f32,
                    (img.height * self.zoom) as f32,
                );
                ui.image((tex.id(), size));
            }
        });
        action
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn debug_image_conversion() {
        let colors = vec![
            0xFFFFFF, 0xFFFF00, 0x00FFFF, 0x00FF00, 0xFF00FF, 0xFF0000, 0x0000FF, 0x000000,
        ];
        let img = DebugImage {
            name: "Barres".into(),
            width: 8,
            height: 1,
            pixels: colors.clone(),
        };
        let ci = to_color_image(&img);
        assert_eq!(ci.size, [8, 1]);
        for (p, c) in ci.pixels.iter().zip(colors) {
            assert_eq!(
                *p,
                egui::Color32::from_rgb((c >> 16) as u8, (c >> 8) as u8, c as u8)
            );
        }
    }

    #[test]
    fn mask_from_checkboxes() {
        assert_eq!(layer_mask(&[false, true, true]), 0b110);
        assert_eq!(layer_mask(&[true, true, true]), 0b111);
        assert_eq!(layer_mask(&[]), 0);
    }
}
