//! Fenêtre « Filtres vidéo » : préréglage d'écran en tête, puis cinq onglets (Image, Lignes et
//! masque, Couleur, Tube, Effets). Un point marque les onglets où un réglage agit ; chaque onglet
//! se réinitialise seul. Le contenu des onglets est dans `fx_tabs.rs`.

use eframe::egui;

use crate::fx_tabs;
use crate::postfx::{Fx, Mask, Preset, ScanMode, NEUTRAL_TEMP};

/// Onglets de la fenêtre.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Tab {
    #[default]
    Image,
    Lines,
    Color,
    Tube,
    Effects,
}

impl Tab {
    pub const ALL: [Tab; 5] = [Tab::Image, Tab::Lines, Tab::Color, Tab::Tube, Tab::Effects];

    pub fn label(self) -> &'static str {
        match self {
            Tab::Image => "Image",
            Tab::Lines => "Lignes et masque",
            Tab::Color => "Couleur",
            Tab::Tube => "Tube",
            Tab::Effects => "Effets",
        }
    }
}

/// Contexte d'affichage utile aux réglages.
#[derive(Clone, Copy, Debug, Default)]
pub struct FxEnv {
    /// Un bezel est affiché (la géométrie du tube est alors désactivée).
    pub bezel: bool,
    /// Fréquence mesurée de l'écran, en Hz.
    pub refresh_hz: f32,
    /// Fréquence d'image du jeu, en Hz (0 sans jeu).
    pub game_hz: f32,
    /// Les filtres sont calculés par la carte graphique (sinon par le processeur).
    pub gpu: bool,
}

impl FxEnv {
    /// L'écran est-il assez rapide pour intercaler une image noire entre deux images du jeu ?
    pub fn bfi_possible(&self) -> bool {
        self.game_hz > 0.0 && self.refresh_hz >= 1.8 * self.game_hz
    }
}

/// Un réglage de l'onglet agit-il sur l'image ?
pub fn tab_active(fx: &Fx, tab: Tab) -> bool {
    let d = Fx::default();
    {
        match tab {
            Tab::Image => fx.scaler != d.scaler || fx.interp > 0 || fx.overscan > 0,
            Tab::Lines => fx.scan_mode != ScanMode::Off || fx.mask != Mask::None,
            Tab::Color => {
                fx.gamma != 100
                    || fx.temp != NEUTRAL_TEMP
                    || fx.black > 0
                    || (fx.red, fx.green, fx.blue) != (100, 100, 100)
            }
            Tab::Tube => {
                fx.geometry_active()
                    || fx.bloom > 0
                    || (fx.conv_rx, fx.conv_ry, fx.conv_bx, fx.conv_by) != (0, 0, 0, 0)
            }
            Tab::Effects => fx.sweep > 0 || fx.jitter > 0 || fx.purity > 0 || fx.bfi,
        }
    }
}

/// Réglages avec ceux de l'onglet remis à leur valeur par défaut.
pub fn reset_tab(fx: &Fx, tab: Tab) -> Fx {
    let d = Fx::default();
    let mut f = *fx;
    match tab {
        Tab::Image => {
            (f.scaler, f.interp, f.overscan) = (d.scaler, d.interp, d.overscan);
        }
        Tab::Lines => {
            (f.scan_mode, f.scan, f.scan_dk, f.scan_br) =
                (d.scan_mode, d.scan, d.scan_dk, d.scan_br);
            (f.beam_dk, f.beam_br) = (d.beam_dk, d.beam_br);
            (f.mask, f.mask_pct, f.mask_pitch) = (d.mask, d.mask_pct, d.mask_pitch);
        }
        Tab::Color => {
            (f.gamma, f.temp, f.black) = (d.gamma, d.temp, d.black);
            (f.red, f.green, f.blue) = (d.red, d.green, d.blue);
        }
        Tab::Tube => {
            (f.curve, f.vignette, f.corner) = (d.curve, d.vignette, d.corner);
            (f.bloom, f.bloom_size) = (d.bloom, d.bloom_size);
            (f.conv_rx, f.conv_ry, f.conv_bx, f.conv_by) = (0, 0, 0, 0);
        }
        Tab::Effects => {
            (f.sweep, f.sweep_speed, f.sweep_width) = (d.sweep, d.sweep_speed, d.sweep_width);
            (f.jitter, f.purity, f.bfi) = (d.jitter, d.purity, d.bfi);
        }
    }
    f
}

#[derive(Default)]
pub struct FxWindow {
    pub open: bool,
    pub tab: Tab,
}

impl FxWindow {
    /// Dessine la fenêtre ; renvoie `true` si un réglage a changé.
    pub fn show(&mut self, ctx: &egui::Context, fx: &mut Fx, env: &FxEnv) -> bool {
        if !self.open {
            return false;
        }
        let before = *fx;
        let mut open = self.open;
        egui::Window::new("Filtres vidéo")
            .open(&mut open)
            .resizable(false)
            .min_width(430.0)
            .show(ctx, |ui| {
                header(ui, fx);
                ui.weak(if env.gpu {
                    "Calcul : carte graphique"
                } else {
                    "Calcul : processeur (carte graphique indisponible ou image trop grande)"
                });
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    for t in Tab::ALL {
                        let mark = if tab_active(fx, t) { " •" } else { "" };
                        let text = format!("{}{mark}", t.label());
                        if ui.selectable_label(self.tab == t, text).clicked() {
                            self.tab = t;
                        }
                    }
                });
                ui.separator();
                match self.tab {
                    Tab::Image => fx_tabs::image(ui, fx),
                    Tab::Lines => fx_tabs::lines(ui, fx),
                    Tab::Color => fx_tabs::color(ui, fx),
                    Tab::Tube => fx_tabs::tube(ui, fx, env),
                    Tab::Effects => fx_tabs::effects(ui, fx, env),
                }
                ui.separator();
                let reset = reset_tab(fx, self.tab);
                if ui
                    .add_enabled(reset != *fx, egui::Button::new("Réinitialiser cet onglet"))
                    .clicked()
                {
                    *fx = reset;
                }
            });
        self.open = open;
        *fx != before
    }
}

/// Préréglage d'écran et « Tout désactiver ».
fn header(ui: &mut egui::Ui, fx: &mut Fx) {
    ui.horizontal(|ui| {
        let current = fx.detect();
        ui.label("Préréglage d'écran");
        egui::ComboBox::from_id_salt("fx_preset")
            .selected_text(current.map_or("Personnalisé", Preset::label))
            .width(200.0)
            .show_ui(ui, |ui| {
                for p in Preset::ALL {
                    if ui.selectable_label(current == Some(p), p.label()).clicked() {
                        // les réglages personnels (tube, géométrie, effets, agrandissement) restent
                        let scaler = fx.scaler;
                        *fx = Fx::preset(p).with_tube_of(fx);
                        fx.scaler = scaler;
                    }
                }
            });
        if ui.button("Tout désactiver").clicked() {
            *fx = Fx::default();
        }
    });
    ui.weak(
        "Un préréglage règle les lignes, le masque, la couleur et le halo ; le reste est gardé.",
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reset_tab_only_touches_its_settings() {
        let fx = Fx {
            interp: 50,
            temp: 93,
            curve: 40,
            jitter: 20,
            mask: Mask::Slot,
            ..Fx::default()
        };
        let r = reset_tab(&fx, Tab::Color);
        assert_eq!(
            (r.temp, r.interp, r.curve, r.jitter),
            (NEUTRAL_TEMP, 50, 40, 20)
        );
        assert_eq!(reset_tab(&fx, Tab::Lines).mask, Mask::None);
        assert!(tab_active(&fx, Tab::Image) && tab_active(&fx, Tab::Lines));
        assert!(tab_active(&fx, Tab::Tube) && tab_active(&fx, Tab::Effects));
        assert!(!tab_active(&Fx::default(), Tab::Image));
    }

    #[test]
    fn bfi_needs_fast_screen() {
        let env = |r| FxEnv {
            gpu: false,
            bezel: false,
            refresh_hz: r,
            game_hz: 59.6,
        };
        assert!(env(120.0).bfi_possible() && env(144.0).bfi_possible());
        assert!(!env(60.0).bfi_possible() && !env(75.0).bfi_possible());
    }
}
