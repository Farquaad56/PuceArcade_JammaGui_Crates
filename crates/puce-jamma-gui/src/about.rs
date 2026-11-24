//! Fenêtre « À propos » : nom du programme, version, auteur et licence, fournis par le programme
//! qui utilise le frontal (`AppOptions::about`).

use eframe::egui;

/// Ce qu'affiche la fenêtre « À propos ».
#[derive(Clone, Debug, PartialEq, Eq, Default)]
pub struct About {
    /// Nom du programme (« PuceArcade CPS1/CPS2 »).
    pub name: String,
    /// Version du programme.
    pub version: String,
    /// Présentation courte (projet, auteur).
    pub text: String,
    /// Nom de la licence et texte complet.
    pub license_name: String,
    pub license_text: String,
}

/// État de la fenêtre.
#[derive(Default)]
pub struct AboutWindow {
    pub open: bool,
    show_license: bool,
}

/// Version du frontal JAMMA (affichée sous celle du programme).
pub const FRONTEND_VERSION: &str = env!("CARGO_PKG_VERSION");

impl AboutWindow {
    pub fn show(&mut self, ctx: &egui::Context, about: &About) {
        if !self.open {
            return;
        }
        let mut open = self.open;
        egui::Window::new("À propos")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .default_width(460.0)
            .anchor(egui::Align2::CENTER_CENTER, egui::Vec2::ZERO)
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.heading(&about.name);
                    if !about.version.is_empty() {
                        ui.label(format!("Version {}", about.version));
                    }
                    ui.weak(format!("Frontal PuceArcade JAMMA {FRONTEND_VERSION}"));
                    ui.add_space(8.0);
                    ui.label(&about.text);
                    if !about.license_name.is_empty() {
                        ui.add_space(8.0);
                        ui.label(format!("Licence : {}", about.license_name));
                    }
                });
                if about.license_text.is_empty() {
                    return;
                }
                ui.add_space(4.0);
                let label = if self.show_license {
                    "Masquer la licence"
                } else {
                    "Lire la licence"
                };
                ui.vertical_centered(|ui| {
                    if ui.button(label).clicked() {
                        self.show_license = !self.show_license;
                    }
                });
                if self.show_license {
                    ui.separator();
                    egui::ScrollArea::vertical()
                        .max_height(360.0)
                        .show(ui, |ui| {
                            ui.add(
                                egui::Label::new(egui::RichText::new(&about.license_text).small())
                                    .wrap(),
                            );
                        });
                }
            });
        self.open = open;
    }
}
