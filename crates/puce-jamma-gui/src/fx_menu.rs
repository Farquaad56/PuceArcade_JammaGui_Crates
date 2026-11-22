//! Fenêtre « Filtres vidéo » : préréglages d'écran, interpolation, scanlines, gamma, masque,
//! et réglages du tube (canaux RVB, convergence, barre de balayage).

use eframe::egui;

use crate::pixel2x::Scaler;
use crate::postfx::{Fx, Mask, Preset, ScanMode, MEDIUM, SHARP, SOFT};

#[derive(Default)]
pub struct FxWindow {
    pub open: bool,
}

fn pct(ui: &mut egui::Ui, v: &mut u8, range: std::ops::RangeInclusive<u8>, step: f64) -> bool {
    ui.add(egui::Slider::new(v, range).suffix(" %").step_by(step))
        .changed()
}

impl FxWindow {
    /// Dessine la fenêtre ; renvoie `true` si un réglage a changé.
    pub fn show(&mut self, ctx: &egui::Context, fx: &mut Fx) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;
        let mut open = self.open;
        egui::Window::new("Filtres vidéo")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                let current = fx.detect();
                egui::ComboBox::from_label("Préréglage d'écran")
                    .selected_text(current.map_or("Personnalisé", Preset::label))
                    .show_ui(ui, |ui| {
                        for p in Preset::ALL {
                            if ui.selectable_label(current == Some(p), p.label()).clicked() {
                                // l'agrandissement et le tube se règlent à part
                                let scaler = fx.scaler;
                                *fx = Fx::preset(p).with_tube_of(fx);
                                fx.scaler = scaler;
                                changed = true;
                            }
                        }
                    });
                ui.separator();

                ui.strong("Agrandissement pixel art");
                egui::ComboBox::from_id_salt("pixel_scaler")
                    .selected_text(fx.scaler.label())
                    .show_ui(ui, |ui| {
                        for k in Scaler::ALL {
                            if ui.selectable_label(fx.scaler == k, k.label()).clicked() {
                                fx.scaler = k;
                                changed = true;
                            }
                        }
                    });
                if fx.scaler != Scaler::Off {
                    ui.weak("Double la définition avant les autres filtres ; utile à partir d'une échelle x2.");
                }
                if fx.scaler.approximate() {
                    ui.weak("Approximation : le code d'origine n'est pas public.");
                }
                ui.separator();

                ui.strong("Interpolation");
                ui.horizontal(|ui| {
                    for (label, v) in [
                        ("Aucune", 0),
                        ("Douce", SOFT),
                        ("Moyenne", MEDIUM),
                        ("Nette", SHARP),
                    ] {
                        if ui.selectable_label(fx.interp == v, label).clicked() {
                            fx.interp = v;
                            changed = true;
                        }
                    }
                });
                if fx.interp > 0 {
                    ui.horizontal(|ui| {
                        ui.label("Netteté");
                        changed |= pct(ui, &mut fx.interp, 5..=100, 5.0);
                    });
                }
                ui.separator();

                ui.strong("Scanlines");
                ui.horizontal(|ui| {
                    for (label, m) in [
                        ("Désactivées", ScanMode::Off),
                        ("Standard", ScanMode::Standard),
                        ("Adaptatives", ScanMode::Adaptive),
                    ] {
                        if ui.radio(fx.scan_mode == m, label).clicked() {
                            fx.scan_mode = m;
                            changed = true;
                        }
                    }
                });
                match fx.scan_mode {
                    ScanMode::Off => {}
                    ScanMode::Standard => {
                        ui.horizontal(|ui| {
                            ui.label("Profondeur");
                            changed |= pct(ui, &mut fx.scan, 0..=95, 5.0);
                        });
                    }
                    ScanMode::Adaptive => {
                        ui.horizontal(|ui| {
                            ui.label("Pixels sombres");
                            if pct(ui, &mut fx.scan_dk, 0..=100, 10.0) {
                                fx.scan_br = fx.scan_br.max(fx.scan_dk);
                                changed = true;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.label("Pixels clairs");
                            if pct(ui, &mut fx.scan_br, 0..=100, 10.0) {
                                fx.scan_dk = fx.scan_dk.min(fx.scan_br);
                                changed = true;
                            }
                        });
                    }
                }
                if fx.scan_mode != ScanMode::Off {
                    ui.weak("Visibles à partir d'une échelle x2.");
                }
                if fx.scan_mode == ScanMode::Adaptive {
                    ui.weak("Luminosité conservée entre deux lignes, selon la clarté du pixel.");
                }
                ui.separator();

                ui.strong("Gamma");
                ui.horizontal(|ui| {
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut fx.gamma, 100..=200)
                                .custom_formatter(|v, _| {
                                    if v <= 100.0 {
                                        "désactivé".to_string()
                                    } else {
                                        format!("{:.2}", v / 100.0)
                                    }
                                })
                                .custom_parser(|s| {
                                    s.replace(',', ".").parse::<f64>().ok().map(|v| v * 100.0)
                                }),
                        )
                        .changed();
                });
                ui.separator();

                ui.strong("Masque");
                ui.horizontal(|ui| {
                    for (label, m) in [
                        ("Aucun", Mask::None),
                        ("Sony Trinitron", Mask::Trinitron),
                        ("Tube fané (jaune)", Mask::FadedYellow),
                    ] {
                        if ui.selectable_label(fx.mask == m, label).clicked() {
                            fx.mask = m;
                            changed = true;
                        }
                    }
                });
                if fx.mask != Mask::None {
                    ui.horizontal(|ui| {
                        ui.label("Force");
                        changed |= pct(ui, &mut fx.mask_pct, 0..=100, 5.0);
                    });
                }
                ui.separator();
                changed |= tube_section(ui, fx);
                ui.separator();
                if ui.button("Tout désactiver").clicked() {
                    *fx = Fx::default();
                    changed = true;
                }
            });
        self.open = open;
        changed
    }
}

/// Curseur de convergence en pixels écran (−8 à 8).
fn px(ui: &mut egui::Ui, label: &str, v: &mut i8) -> bool {
    ui.label(label);
    ui.add(egui::Slider::new(v, -8..=8).suffix(" px")).changed()
}

/// Section « Tube CRT » : canaux, convergence, barre de balayage.
fn tube_section(ui: &mut egui::Ui, fx: &mut Fx) -> bool {
    let mut changed = false;
    egui::CollapsingHeader::new(egui::RichText::new("Tube CRT").strong())
        .default_open(fx.tube_active())
        .show(ui, |ui| {
            ui.label("Canaux RVB");
            egui::Grid::new("fx_rgb").num_columns(2).show(ui, |ui| {
                for (label, v) in [
                    ("Rouge", &mut fx.red),
                    ("Vert", &mut fx.green),
                    ("Bleu", &mut fx.blue),
                ] {
                    ui.label(label);
                    changed |= pct(ui, v, 0..=200, 1.0);
                    ui.end_row();
                }
            });
            if ui.small_button("Canaux neutres").clicked() {
                (fx.red, fx.green, fx.blue) = (100, 100, 100);
                changed = true;
            }
            ui.separator();

            ui.label("Convergence (décalage par rapport au vert)");
            egui::Grid::new("fx_conv").num_columns(4).show(ui, |ui| {
                changed |= px(ui, "Rouge X", &mut fx.conv_rx);
                changed |= px(ui, "Y", &mut fx.conv_ry);
                ui.end_row();
                changed |= px(ui, "Bleu X", &mut fx.conv_bx);
                changed |= px(ui, "Y", &mut fx.conv_by);
                ui.end_row();
            });
            ui.horizontal(|ui| {
                if ui.small_button("Convergence parfaite").clicked() {
                    (fx.conv_rx, fx.conv_ry, fx.conv_bx, fx.conv_by) = (0, 0, 0, 0);
                    changed = true;
                }
                if ui.small_button("Tube fatigué").clicked() {
                    (fx.conv_rx, fx.conv_ry, fx.conv_bx, fx.conv_by) = (1, 0, -1, 1);
                    changed = true;
                }
            });
            ui.weak("En pixels de l'image affichée : 1 px suffit souvent en ×4.");
            ui.separator();

            ui.label("Barre de balayage");
            egui::Grid::new("fx_sweep").num_columns(2).show(ui, |ui| {
                ui.label("Intensité");
                changed |= pct(ui, &mut fx.sweep, 0..=100, 5.0);
                ui.end_row();
                if fx.sweep > 0 {
                    ui.label("Durée d'un passage");
                    changed |= ui
                        .add(
                            egui::Slider::new(&mut fx.sweep_speed, 5..=100)
                                .custom_formatter(|v, _| format!("{:.1} s", v / 10.0))
                                .custom_parser(|s| {
                                    s.trim_end_matches('s')
                                        .trim()
                                        .replace(',', ".")
                                        .parse::<f64>()
                                        .ok()
                                        .map(|v| v * 10.0)
                                }),
                        )
                        .changed();
                    ui.end_row();
                    ui.label("Hauteur");
                    changed |= pct(ui, &mut fx.sweep_width, 2..=50, 1.0);
                    ui.end_row();
                }
            });
            ui.separator();

            ui.label("Faisceau et phosphore");
            egui::Grid::new("fx_beam").num_columns(2).show(ui, |ui| {
                ui.label("Faisceau au ralenti");
                changed |= ui
                    .add(
                        egui::Slider::new(&mut fx.beam, 0..=100)
                            .custom_formatter(|v, _| {
                                if v < 1.0 {
                                    "désactivé".to_string()
                                } else {
                                    format!("{:.1} s par image", v / 10.0)
                                }
                            })
                            .custom_parser(|s| {
                                s.split_whitespace()
                                    .next()?
                                    .replace(',', ".")
                                    .parse::<f64>()
                                    .ok()
                                    .map(|v| v * 10.0)
                            }),
                    )
                    .changed();
                ui.end_row();
                if fx.beam > 0 {
                    ui.label("Persistance du phosphore");
                    changed |= pct(ui, &mut fx.phosphor, 0..=100, 5.0);
                    ui.end_row();
                    ui.label("Luminosité minimale");
                    changed |= pct(ui, &mut fx.beam_floor, 0..=100, 5.0);
                    ui.end_row();
                }
                ui.label("Rémanence entre images");
                changed |= pct(ui, &mut fx.ghost, 0..=90, 5.0);
                ui.end_row();
            });
            if fx.beam > 0 {
                ui.weak("L'image se redessine ligne par ligne ; le jeu continue en temps réel.");
            }
        });
    changed
}
