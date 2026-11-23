//! Mires de réglage : état (mire et variante affichées, panneau, pause du jeu) et panneau de
//! commande. La mire remplace l'image du jeu et passe par les filtres et le bezel.
//!
//! Raccourcis : Ctrl+M affiche ou quitte les mires ; Page préc. / Page suiv. change de mire ;
//! Ctrl + Page préc. / Page suiv. change de variante ; Ctrl+H masque ou montre le panneau.

use eframe::egui;

use crate::mire::Mire;

/// Ce que le panneau demande à l'application.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MireCmd {
    Close,
    OpenFilters,
}

pub struct MireState {
    /// Mires affichées à la place du jeu.
    pub active: bool,
    pub kind: Mire,
    pub variant: usize,
    /// Panneau de commande visible.
    pub panel: bool,
    /// Mettre le jeu en pause pendant les mires.
    pub pause_game: bool,
    /// Le jeu a été mis en pause par les mires (à relancer en les quittant).
    pub paused_by_mire: bool,
    /// Compteur de trames d'affichage (animation, balayage, instabilité).
    pub frame: u64,
}

impl Default for MireState {
    fn default() -> Self {
        MireState {
            active: false,
            kind: Mire::default(),
            variant: 0,
            panel: true,
            pause_game: true,
            paused_by_mire: false,
            frame: 0,
        }
    }
}

impl MireState {
    /// Mire suivante (`d` = 1) ou précédente (`d` = −1), en boucle ; variante remise à 0.
    pub fn step(&mut self, d: i32) {
        let n = Mire::ALL.len() as i32;
        let i = Mire::ALL.iter().position(|m| *m == self.kind).unwrap_or(0) as i32;
        self.kind = Mire::ALL[(i + d).rem_euclid(n) as usize];
        self.variant = 0;
    }

    /// Variante suivante ou précédente, en boucle.
    pub fn step_variant(&mut self, d: i32) {
        let n = self.kind.variants().len() as i32;
        self.variant = (self.variant as i32 + d).rem_euclid(n) as usize;
    }

    /// Panneau de commande ; renvoie une demande éventuelle.
    pub fn show(&mut self, ctx: &egui::Context) -> Option<MireCmd> {
        if !(self.active && self.panel) {
            return None;
        }
        let mut cmd = None;
        let mut open = true;
        egui::Window::new("Mires de réglage")
            .open(&mut open)
            .resizable(false)
            .default_width(360.0)
            .show(ctx, |ui| {
                egui::Grid::new("mire_list")
                    .num_columns(2)
                    .spacing([8.0, 2.0])
                    .show(ui, |ui| {
                        for (i, m) in Mire::ALL.into_iter().enumerate() {
                            if ui.selectable_label(self.kind == m, m.label()).clicked() {
                                self.kind = m;
                                self.variant = 0;
                            }
                            if i % 2 == 1 {
                                ui.end_row();
                            }
                        }
                    });
                let variants = self.kind.variants();
                if variants.len() > 1 {
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        ui.label("Variante :");
                        for (i, v) in variants.iter().enumerate() {
                            ui.selectable_value(&mut self.variant, i, *v);
                        }
                    });
                }
                ui.separator();
                ui.label(self.kind.help());
                ui.separator();
                ui.checkbox(&mut self.pause_game, "Jeu en pause pendant les mires");
                ui.horizontal(|ui| {
                    if ui.button("Filtres vidéo…").clicked() {
                        cmd = Some(MireCmd::OpenFilters);
                    }
                    if ui.button("Quitter les mires").clicked() {
                        cmd = Some(MireCmd::Close);
                    }
                });
                ui.weak(
                    "Page préc. / suiv. : mire · Ctrl + Page : variante · Ctrl+H : panneau · \
                     Ctrl+M : quitter",
                );
            });
        if !open {
            cmd = Some(MireCmd::Close);
        }
        cmd
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stepping_wraps() {
        let mut s = MireState::default();
        s.step(-1);
        assert_eq!(s.kind, Mire::Motion);
        s.step(1);
        assert_eq!(s.kind, Mire::Geometry);
        s.kind = Mire::Purity;
        s.step_variant(-1);
        assert_eq!(s.variant, 5);
        s.step_variant(1);
        assert_eq!(s.variant, 0);
        s.variant = 3;
        s.step(1);
        assert_eq!((s.kind, s.variant), (Mire::Contrast, 0));
    }
}
