//! Liste des jeux du driver : filtre et fenêtre « Ouvrir un jeu ».

use eframe::egui;
use puce_jamma::driver::GameEntry;

/// Jeux à afficher : filtre insensible à la casse sur le nom ou le titre, ordre du driver conservé.
pub fn game_rows<'a>(
    games: &'a [GameEntry],
    filter: &str,
    only_available: bool,
) -> Vec<&'a GameEntry> {
    let f = filter.trim().to_lowercase();
    games
        .iter()
        .filter(|g| !only_available || g.available)
        .filter(|g| {
            f.is_empty()
                || g.name.to_lowercase().contains(&f)
                || g.title.to_lowercase().contains(&f)
        })
        .collect()
}

#[derive(Default)]
pub struct GamesWindow {
    pub open: bool,
    filter: String,
    only_available: bool,
}

impl GamesWindow {
    /// Affiche la liste ; renvoie le nom du jeu choisi.
    pub fn show(&mut self, ctx: &egui::Context, games: &[GameEntry]) -> Option<String> {
        if !self.open {
            return None;
        }
        let mut chosen = None;
        let mut open = self.open;
        egui::Window::new("Ouvrir un jeu")
            .open(&mut open)
            .default_height(480.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    ui.label("Filtre");
                    ui.text_edit_singleline(&mut self.filter);
                    ui.checkbox(&mut self.only_available, "Disponibles seulement");
                });
                ui.separator();
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for g in game_rows(games, &self.filter, self.only_available) {
                        let indent = if g.parent.is_empty() { "" } else { "    " };
                        let mut text = format!("{indent}{}  ({})", g.title, g.name);
                        if !g.supported {
                            text.push_str("  [non pris en charge]");
                        }
                        let enabled = g.available && g.supported;
                        if ui
                            .add_enabled(enabled, egui::Button::new(text).frame(false))
                            .clicked()
                        {
                            chosen = Some(g.name.clone());
                        }
                    }
                });
            });
        self.open = open && chosen.is_none();
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn g(name: &str, parent: &str, available: bool) -> GameEntry {
        GameEntry {
            name: name.into(),
            title: format!("Titre {name}"),
            parent: parent.into(),
            available,
            supported: true,
        }
    }

    #[test]
    fn filter_and_availability() {
        let list = vec![
            g("ghouls", "", true),
            g("sf2", "", true),
            g("sf2ce", "sf2", false),
        ];
        let names = |v: Vec<&GameEntry>| v.iter().map(|e| e.name.clone()).collect::<Vec<_>>();
        assert_eq!(names(game_rows(&list, "sf2", false)), ["sf2", "sf2ce"]);
        assert_eq!(names(game_rows(&list, "SF2", true)), ["sf2"]);
        assert_eq!(
            names(game_rows(&list, "", false)),
            ["ghouls", "sf2", "sf2ce"]
        );
        assert_eq!(names(game_rows(&list, "titre g", false)), ["ghouls"]);
    }
}
