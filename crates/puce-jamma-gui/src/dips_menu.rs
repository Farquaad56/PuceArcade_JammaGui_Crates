//! Fenêtre « DIP switches » construite entièrement depuis `BoardInfo.dips` : une liste par switch,
//! regroupés par banque, bouton « Valeurs par défaut ». Persistance `<saves_dir>/<jeu>.dips` (1 octet par banque).

use std::path::{Path, PathBuf};

use eframe::egui;
use puce_jamma::board::BoardInfo;
use puce_jamma::dips::DipSwitch;
use puce_jamma::util::{default_dips, dip_value, set_dip};

/// Libellé du réglage courant d'un switch (`?` si la valeur ne correspond à aucun réglage).
pub fn current_label<'a>(banks: &[u8], sw: &'a DipSwitch) -> &'a str {
    let v = dip_value(banks, sw);
    sw.settings
        .iter()
        .find(|s| s.value == v)
        .map_or("?", |s| s.label.as_str())
}

/// Choisit le réglage `label` d'un switch ; les autres bits de la banque sont conservés.
pub fn choose(banks: &mut [u8], sw: &DipSwitch, label: &str) -> bool {
    match sw.settings.iter().find(|s| s.label == label) {
        Some(s) => {
            set_dip(banks, sw, s.value);
            true
        }
        None => false,
    }
}

pub fn dips_path(saves_dir: &Path, game: &str) -> PathBuf {
    saves_dir.join(format!("{game}.dips"))
}

/// DIP enregistrés pour un jeu ; valeurs par défaut si le fichier manque ou n'a pas la bonne taille.
pub fn load_dips(saves_dir: &Path, info: &BoardInfo) -> Vec<u8> {
    match std::fs::read(dips_path(saves_dir, &info.name)) {
        Ok(d) if d.len() == info.dip_banks.len() => d,
        _ => default_dips(info),
    }
}

pub fn save_dips(saves_dir: &Path, game: &str, banks: &[u8]) -> Result<(), String> {
    std::fs::create_dir_all(saves_dir).map_err(|e| format!("{} : {e}", saves_dir.display()))?;
    let path = dips_path(saves_dir, game);
    std::fs::write(&path, banks).map_err(|e| format!("{} : {e}", path.display()))
}

#[derive(Default)]
pub struct DipsWindow {
    pub open: bool,
}

impl DipsWindow {
    /// Dessine la fenêtre ; renvoie `true` si `banks` a changé.
    pub fn show(&mut self, ctx: &egui::Context, info: &BoardInfo, banks: &mut Vec<u8>) -> bool {
        if !self.open {
            return false;
        }
        let mut changed = false;
        let mut open = self.open;
        egui::Window::new("DIP switches")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                if info.dips.is_empty() {
                    ui.label("Cette carte n'a pas de DIP switches décrits.");
                }
                for (b, bank) in info.dip_banks.iter().enumerate() {
                    ui.strong(format!(
                        "{} (0x{:02X})",
                        bank.name,
                        banks.get(b).copied().unwrap_or(0)
                    ));
                    egui::Grid::new(("dips", b))
                        .num_columns(2)
                        .striped(true)
                        .show(ui, |ui| {
                            for (i, sw) in info.dips.iter().enumerate().filter(|(_, s)| s.bank == b)
                            {
                                ui.label(&sw.label);
                                let mut sel = current_label(banks, sw).to_string();
                                egui::ComboBox::from_id_salt(("dip", i))
                                    .selected_text(&sel)
                                    .show_ui(ui, |ui| {
                                        for s in &sw.settings {
                                            ui.selectable_value(
                                                &mut sel,
                                                s.label.clone(),
                                                &s.label,
                                            );
                                        }
                                    });
                                if sel != current_label(banks, sw) && choose(banks, sw, &sel) {
                                    changed = true;
                                }
                                ui.end_row();
                            }
                        });
                }
                ui.separator();
                if ui.button("Valeurs par défaut").clicked() {
                    *banks = default_dips(info);
                    changed = true;
                }
                ui.weak("Appliqué immédiatement et enregistré pour ce jeu.");
            });
        self.open = open;
        changed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma::board::JammaBoard;
    use puce_jamma::video::Rotation;
    use puce_jamma_testboard::TestBoard;

    #[test]
    fn testboard_cursor_speed() {
        let b = TestBoard::new(Rotation::Rot0);
        let info = b.info().clone();
        let sw = info
            .dips
            .iter()
            .find(|s| s.label == "Vitesse du curseur")
            .unwrap();
        let mut banks = vec![0x04];
        assert_eq!(current_label(&banks, sw), "2");
        assert!(choose(&mut banks, sw, "4"));
        assert_eq!(banks, vec![0x00]);
        let mut banks = vec![0x05];
        choose(&mut banks, sw, "4");
        assert_eq!(banks, vec![0x01], "les autres bits sont conservés");
        assert!(!choose(&mut banks, sw, "9"));
    }

    #[test]
    fn persistence() {
        let b = TestBoard::new(Rotation::Rot0);
        let info = b.info().clone();
        let dir = std::env::temp_dir().join(format!("jamma_dips_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(load_dips(&dir, &info), vec![0xFF]);
        save_dips(&dir, "mire", &[0x12]).unwrap();
        assert_eq!(load_dips(&dir, &info), vec![0x12]);
        std::fs::write(dips_path(&dir, "mire"), [1, 2]).unwrap();
        assert_eq!(
            load_dips(&dir, &info),
            vec![0xFF],
            "taille incorrecte -> défaut"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
