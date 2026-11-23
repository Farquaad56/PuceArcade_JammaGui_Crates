//! Liste des jeux du driver : filtre par texte, disponibilité et catégorie (original, clone
//! officiel, bootleg, hack, prototype), avec un code couleur par catégorie ; fenêtre
//! « Ouvrir un jeu ».

use eframe::egui;
use puce_jamma::driver::{GameEntry, GameKind};

/// Couleur d'une catégorie (lisible sur le thème sombre) : blanc pour les originaux, bleu pour
/// les clones officiels, orange pour les bootlegs, magenta pour les hacks, jaune pour les prototypes.
pub fn kind_color(k: GameKind) -> egui::Color32 {
    match k {
        GameKind::Original => egui::Color32::from_rgb(235, 235, 235),
        GameKind::Clone => egui::Color32::from_rgb(120, 175, 255),
        GameKind::Bootleg => egui::Color32::from_rgb(255, 155, 60),
        GameKind::Hack => egui::Color32::from_rgb(240, 105, 220),
        GameKind::Prototype => egui::Color32::from_rgb(230, 210, 80),
    }
}

/// Rang d'une catégorie dans `GameKind::ALL`.
fn kind_index(k: GameKind) -> usize {
    GameKind::ALL.iter().position(|x| *x == k).unwrap_or(0)
}

/// Critères de la liste.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameFilter {
    pub text: String,
    pub only_available: bool,
    /// Catégories affichées, dans l'ordre de `GameKind::ALL`.
    pub kinds: [bool; 5],
    /// Systèmes masqués (ADR 003) ; vide : tous affichés.
    pub hidden_systems: Vec<String>,
}

impl Default for GameFilter {
    fn default() -> Self {
        GameFilter {
            text: String::new(),
            only_available: false,
            kinds: [true; 5],
            hidden_systems: Vec::new(),
        }
    }
}

/// Jeux à afficher : texte insensible à la casse sur le nom ou le titre, disponibilité,
/// catégorie ; ordre du driver conservé.
pub fn game_rows<'a>(games: &'a [GameEntry], f: &GameFilter) -> Vec<&'a GameEntry> {
    let t = f.text.trim().to_lowercase();
    games
        .iter()
        .filter(|g| !f.only_available || g.available)
        .filter(|g| f.kinds[kind_index(g.kind)])
        .filter(|g| !f.hidden_systems.contains(&g.system))
        .filter(|g| {
            t.is_empty()
                || g.name.to_lowercase().contains(&t)
                || g.title.to_lowercase().contains(&t)
        })
        .collect()
}

/// Nombre de jeux par catégorie (pour les cases du filtre).
pub fn kind_counts(games: &[GameEntry]) -> [usize; 5] {
    let mut n = [0; 5];
    for g in games {
        n[kind_index(g.kind)] += 1;
    }
    n
}

/// Systèmes du catalogue avec leur nombre de jeux, dans l'ordre d'apparition ; vide si le catalogue
/// n'a qu'un système (ou aucun renseigné) : pas de filtre ni d'étiquette à afficher.
pub fn system_counts(games: &[GameEntry]) -> Vec<(String, usize)> {
    let mut out: Vec<(String, usize)> = Vec::new();
    for g in games.iter().filter(|g| !g.system.is_empty()) {
        match out.iter_mut().find(|(s, _)| *s == g.system) {
            Some((_, n)) => *n += 1,
            None => out.push((g.system.clone(), 1)),
        }
    }
    if out.len() < 2 {
        out.clear();
    }
    out
}

#[derive(Default)]
pub struct GamesWindow {
    pub open: bool,
    filter: GameFilter,
}

impl GamesWindow {
    /// Cases des catégories (légende des couleurs) et raccourcis de sélection.
    fn kind_filters(&mut self, ui: &mut egui::Ui, games: &[GameEntry]) {
        let counts = kind_counts(games);
        ui.horizontal_wrapped(|ui| {
            for (i, k) in GameKind::ALL.into_iter().enumerate() {
                let text = egui::RichText::new(format!("● {} ({})", k.label(), counts[i]))
                    .color(kind_color(k));
                ui.checkbox(&mut self.filter.kinds[i], text);
            }
        });
        ui.horizontal(|ui| {
            if ui.small_button("Commerciaux seulement").clicked() {
                self.filter.kinds = GameKind::ALL.map(GameKind::commercial);
            }
            if ui.small_button("Tout afficher").clicked() {
                self.filter.kinds = [true; 5];
            }
        });
    }

    /// Cases des systèmes (une par système), si le catalogue en a plusieurs.
    fn system_filters(&mut self, ui: &mut egui::Ui, systems: &[(String, usize)]) {
        if systems.is_empty() {
            return;
        }
        ui.horizontal_wrapped(|ui| {
            ui.label("Système");
            for (sys, n) in systems {
                let mut shown = !self.filter.hidden_systems.contains(sys);
                if ui.checkbox(&mut shown, format!("{sys} ({n})")).changed() {
                    if shown {
                        self.filter.hidden_systems.retain(|s| s != sys);
                    } else {
                        self.filter.hidden_systems.push(sys.clone());
                    }
                }
            }
        });
    }

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
                    ui.text_edit_singleline(&mut self.filter.text);
                    ui.checkbox(&mut self.filter.only_available, "Disponibles seulement");
                });
                let systems = system_counts(games);
                self.system_filters(ui, &systems);
                self.kind_filters(ui, games);
                ui.separator();
                let rows = game_rows(games, &self.filter);
                ui.weak(format!("{} jeux affichés", rows.len()));
                egui::ScrollArea::vertical().show(ui, |ui| {
                    for g in rows {
                        let indent = if g.parent.is_empty() { "" } else { "    " };
                        let sys = if systems.is_empty() || g.system.is_empty() {
                            String::new()
                        } else {
                            format!("[{}] ", g.system)
                        };
                        let mut text = format!("{indent}● {sys}{}  ({})", g.title, g.name);
                        if !g.supported {
                            text.push_str("  [non pris en charge]");
                        }
                        let label = egui::RichText::new(text).color(kind_color(g.kind));
                        let enabled = g.available && g.supported;
                        if ui
                            .add_enabled(enabled, egui::Button::new(label).frame(false))
                            .on_hover_text(g.kind.label())
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

    fn g(name: &str, parent: &str, available: bool, kind: GameKind) -> GameEntry {
        GameEntry {
            name: name.into(),
            title: format!("Titre {name}"),
            parent: parent.into(),
            available,
            supported: true,
            kind,
            system: String::new(),
        }
    }

    fn names(v: Vec<&GameEntry>) -> Vec<String> {
        v.iter().map(|e| e.name.clone()).collect()
    }

    #[test]
    fn filter_and_availability() {
        let list = vec![
            g("ghouls", "", true, GameKind::Original),
            g("sf2", "", true, GameKind::Original),
            g("sf2ce", "sf2", false, GameKind::Clone),
        ];
        let f = |text: &str, only: bool| GameFilter {
            text: text.into(),
            only_available: only,
            ..GameFilter::default()
        };
        assert_eq!(names(game_rows(&list, &f("sf2", false))), ["sf2", "sf2ce"]);
        assert_eq!(names(game_rows(&list, &f("SF2", true))), ["sf2"]);
        assert_eq!(
            names(game_rows(&list, &f("", false))),
            ["ghouls", "sf2", "sf2ce"]
        );
        assert_eq!(names(game_rows(&list, &f("titre g", false))), ["ghouls"]);
    }

    #[test]
    fn filter_by_kind() {
        let list = vec![
            g("sf2", "", true, GameKind::Original),
            g("sf2ce", "sf2", true, GameKind::Clone),
            g("sf2rb", "sf2ce", true, GameKind::Bootleg),
            g("ffightae", "ffight", true, GameKind::Hack),
            g("gulunpa", "", true, GameKind::Prototype),
        ];
        let commercial = GameFilter {
            kinds: GameKind::ALL.map(GameKind::commercial),
            ..GameFilter::default()
        };
        assert_eq!(names(game_rows(&list, &commercial)), ["sf2", "sf2ce"]);
        let bootlegs = GameFilter {
            kinds: [false, false, true, true, false],
            ..GameFilter::default()
        };
        assert_eq!(names(game_rows(&list, &bootlegs)), ["sf2rb", "ffightae"]);
        assert_eq!(kind_counts(&list), [1, 1, 1, 1, 1]);
        assert_ne!(
            kind_color(GameKind::Original),
            kind_color(GameKind::Bootleg)
        );
    }

    #[test]
    fn filter_by_system() {
        let mut list = vec![
            g("sf2", "", true, GameKind::Original),
            g("ssf2", "", true, GameKind::Original),
            g("xmcota", "", true, GameKind::Original),
        ];
        list[0].system = "CPS1".into();
        list[1].system = "CPS2".into();
        list[2].system = "CPS2".into();
        assert_eq!(
            system_counts(&list),
            vec![("CPS1".to_string(), 1), ("CPS2".to_string(), 2)]
        );
        let f = GameFilter {
            hidden_systems: vec!["CPS1".into()],
            ..Default::default()
        };
        assert_eq!(names(game_rows(&list, &f)), ["ssf2", "xmcota"]);
        // un seul système : ni filtre ni étiquette
        list[0].system = "CPS2".into();
        assert!(system_counts(&list).is_empty());
        list.iter_mut().for_each(|e| e.system.clear());
        assert!(system_counts(&list).is_empty());
    }
}
