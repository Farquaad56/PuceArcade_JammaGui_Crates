//! Mires de réglage dans l'application : image de la mire (à la définition du jeu, ou 320 × 240
//! sans jeu), mise en pause du jeu, panneau et raccourcis.

use eframe::egui;
use puce_jamma::video::{Rotation, VideoInfo};

use crate::app::{App, Shown};
use crate::mire::{self, Mire};
use crate::mire_menu::{MireCmd, MireState};

#[derive(Default)]
pub(crate) struct MireView {
    pub(crate) state: MireState,
    pub(crate) shown: Option<Shown>,
    tex: Option<egui::TextureHandle>,
    /// Mire, variante, taille et trame (mires animées) de l'image en texture.
    key: Option<(Mire, usize, usize, usize, u64)>,
}

/// Format d'affichage sans jeu chargé : 320 × 240, 4:3, 60 Hz.
pub(crate) fn default_info() -> VideoInfo {
    VideoInfo {
        width: 320,
        height: 240,
        refresh_hz: 60.0,
        aspect: (4, 3),
        rotation: Rotation::Rot0,
    }
}

/// Taille de l'image affichée (après rotation).
fn shown_size(info: &VideoInfo) -> (usize, usize) {
    let (w, h) = (info.width as usize, info.height as usize);
    match info.rotation {
        Rotation::Rot90 | Rotation::Rot270 => (h, w),
        _ => (w, h),
    }
}

impl App {
    /// Texture de la mire courante (recalculée si la mire, la variante ou la taille changent).
    pub(crate) fn mire_texture(
        &mut self,
        ctx: &egui::Context,
        info: &VideoInfo,
    ) -> egui::TextureId {
        let (w, h) = self.shown_size().unwrap_or_else(|| shown_size(info));
        let m = &mut self.mire;
        m.state.frame += 1;
        let s = &m.state;
        let anim = if s.kind.animated() { s.frame } else { 0 };
        let key = (s.kind, s.variant, w, h, anim);
        if m.key != Some(key) || m.tex.is_none() {
            let pixels = mire::render(s.kind, s.variant, w, h, s.frame);
            let img = egui::ColorImage {
                size: [w, h],
                source_size: egui::vec2(w as f32, h as f32),
                pixels: pixels
                    .iter()
                    .map(|p| egui::Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, *p as u8))
                    .collect(),
            };
            match &mut m.tex {
                Some(t) => t.set(img, egui::TextureOptions::NEAREST),
                None => {
                    m.tex = Some(ctx.load_texture("jamma_mire", img, egui::TextureOptions::NEAREST))
                }
            }
            m.shown = Some(Shown {
                pixels,
                w,
                h,
                frame: 0,
            });
            m.key = Some(key);
        }
        // trame changeante : barre de balayage et instabilité animées même sur une mire fixe
        if let Some(sh) = &mut m.shown {
            sh.frame = m.state.frame;
        }
        m.tex
            .as_ref()
            .map_or(egui::TextureId::default(), |t| t.id())
    }

    /// Affiche ou quitte les mires ; met le jeu en pause et le relance au besoin.
    pub(crate) fn toggle_mires(&mut self) {
        let on = !self.mire.state.active;
        self.mire.state.active = on;
        if on {
            self.mire.state.panel = true;
            if self.mire.state.pause_game && !self.paused && self.board.is_some() {
                self.set_paused(true);
                self.mire.state.paused_by_mire = true;
            }
        } else if std::mem::take(&mut self.mire.state.paused_by_mire) {
            self.set_paused(false);
        }
    }

    /// Panneau des mires.
    pub(crate) fn mire_panel(&mut self, ctx: &egui::Context) {
        match self.mire.state.show(ctx) {
            Some(MireCmd::Close) => self.toggle_mires(),
            Some(MireCmd::OpenFilters) => self.fx_win.open = true,
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn size_follows_rotation() {
        let mut i = default_info();
        assert_eq!(shown_size(&i), (320, 240));
        i.rotation = Rotation::Rot270;
        assert_eq!(shown_size(&i), (240, 320));
    }
}
