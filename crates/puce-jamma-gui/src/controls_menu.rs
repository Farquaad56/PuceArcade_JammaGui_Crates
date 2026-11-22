//! Fenêtre « Commandes » : assignation des touches et des boutons de manette.
//! Reprise de ref/puce16-neogeo/ng-gui/controls_menu.rs ; n'affiche que les joueurs `1..=info.players`
//! et les boutons `1..=info.buttons` de la carte chargée.

use eframe::egui::{self, Key};
use puce_jamma::board::BoardInfo;

use crate::bindings::Bindings;
use crate::gamepad::Gamepads;
use crate::input::{button_name, Action, Control};

/// Ce que l'on attend de l'utilisateur.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Capture {
    Key(Action),
    Button(Action),
}

#[derive(Default)]
pub struct ControlsWindow {
    pub open: bool,
    capture: Option<Capture>,
}

const WAIT_KEY: &str = "Appuyez sur une touche…";
const WAIT_BUTTON: &str = "Appuyez sur un bouton…";

/// Nom affiché d'une touche (les flèches n'ont pas de glyphe dans la police par défaut).
pub fn key_label(k: Key) -> &'static str {
    match k {
        Key::ArrowUp => "Flèche haut",
        Key::ArrowDown => "Flèche bas",
        Key::ArrowLeft => "Flèche gauche",
        Key::ArrowRight => "Flèche droite",
        Key::Enter => "Entrée",
        Key::Space => "Espace",
        other => other.name(),
    }
}

/// Texte de la case « clavier » d'une action.
pub fn key_cell(b: &Bindings, a: Action, capturing: bool) -> String {
    if capturing {
        return WAIT_KEY.into();
    }
    b.key(a).map_or("—".into(), |k| key_label(k).to_string())
}

/// Texte de la case « manette » d'une action.
pub fn button_cell(b: &Bindings, a: Action, capturing: bool) -> String {
    if capturing {
        return WAIT_BUTTON.into();
    }
    b.button(a)
        .map_or("—".into(), |x| button_name(x).to_string())
}

/// Nombre de colonnes : un joueur par colonne (1 à 4), puis « Système ».
pub fn groups(info: &BoardInfo) -> usize {
    info.players.clamp(1, 4) as usize + 1
}

/// Actions affichées dans la colonne `group` pour la carte `info`.
pub fn visible_actions(info: &BoardInfo, group: usize) -> Vec<Action> {
    let players = info.players.clamp(1, 4) as usize;
    let buttons = info.buttons.min(6);
    Action::all()
        .filter(|a| match *a {
            Action::Player(p, c) => {
                group < players
                    && p as usize == group
                    && !matches!(c, Control::Button(b) if b >= buttons)
            }
            Action::Coin(n) => group == players && (n as usize) < players.max(2),
            _ => group == players,
        })
        .collect()
}

impl ControlsWindow {
    /// Une capture est en cours : les touches ne doivent pas aller au jeu.
    pub fn capturing(&self) -> bool {
        self.open && self.capture.is_some()
    }

    /// Affiche la fenêtre ; renvoie vrai si les assignations ont changé.
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        info: &BoardInfo,
        bindings: &mut Bindings,
        pads: &mut Gamepads,
    ) -> bool {
        if !self.open {
            self.capture = None;
            return false;
        }
        let mut changed = self.poll_capture(ctx, bindings, pads);
        let mut open = self.open;
        let n = groups(info);
        egui::Window::new("Commandes")
            .open(&mut open)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(
                    "Cliquez sur une case puis appuyez sur la touche ou le bouton. Échap annule.",
                );
                ui.label("Une touche déjà utilisée est échangée avec l'autre action.");
                ui.separator();
                ui.columns(n, |cols| {
                    for (g, ui) in cols.iter_mut().enumerate() {
                        let title = if g + 1 == n {
                            "Système".to_string()
                        } else {
                            format!("Joueur {}", g + 1)
                        };
                        ui.strong(&title);
                        self.grid(ui, g, &visible_actions(info, g), bindings, pads);
                        if ui.button(format!("Défaut {title}")).clicked() {
                            let acts = visible_actions(info, g);
                            bindings.reset(|a| acts.contains(&a));
                            changed = true;
                        }
                    }
                });
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Tout par défaut").clicked() {
                        *bindings = Bindings::default();
                        self.capture = None;
                        changed = true;
                    }
                    ui.weak(
                        "Le stick gauche donne toujours les directions. Sauvegarde automatique.",
                    );
                });
            });
        self.open = open;
        changed
    }

    fn grid(
        &mut self,
        ui: &mut egui::Ui,
        g: usize,
        acts: &[Action],
        b: &Bindings,
        pads: &mut Gamepads,
    ) {
        egui::Grid::new(("controls", g))
            .num_columns(3)
            .striped(true)
            .show(ui, |ui| {
                ui.weak("Action");
                ui.weak("Clavier");
                ui.weak("Manette");
                ui.end_row();
                for &a in acts {
                    ui.label(a.label());
                    let cap = self.capture == Some(Capture::Key(a));
                    if ui.selectable_label(cap, key_cell(b, a, cap)).clicked() {
                        self.capture = Some(Capture::Key(a));
                    }
                    let cap = self.capture == Some(Capture::Button(a));
                    if ui.selectable_label(cap, button_cell(b, a, cap)).clicked() {
                        pads.take_pressed_button();
                        self.capture = Some(Capture::Button(a));
                    }
                    ui.end_row();
                }
            });
    }

    /// Termine la capture en cours si une touche ou un bouton vient d'être pressé.
    fn poll_capture(&mut self, ctx: &egui::Context, b: &mut Bindings, pads: &mut Gamepads) -> bool {
        let Some(cap) = self.capture else {
            return false;
        };
        let key = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Key {
                    key,
                    pressed: true,
                    repeat: false,
                    ..
                } => Some(*key),
                _ => None,
            })
        });
        if key == Some(Key::Escape) {
            self.capture = None;
            return false;
        }
        match cap {
            Capture::Key(a) => {
                let Some(k) = key else { return false };
                b.set_key(a, k);
            }
            Capture::Button(a) => {
                let Some((pad, btn)) = pads.take_pressed_button() else {
                    return false;
                };
                // Service, Test et Tilt acceptent n'importe quelle manette.
                if a.player().is_some_and(|p| p != pad) {
                    return false;
                }
                b.set_button(a, btn);
            }
        }
        self.capture = None;
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma::audio::AudioInfo;
    use puce_jamma::video::{Rotation, VideoInfo};

    fn info(players: u8, buttons: u8) -> BoardInfo {
        BoardInfo {
            system: "Test".into(),
            name: "t".into(),
            title: "t".into(),
            players,
            buttons,
            video: VideoInfo {
                width: 320,
                height: 240,
                refresh_hz: 60.0,
                aspect: (4, 3),
                rotation: Rotation::Rot0,
            },
            audio: AudioInfo {
                sample_rate: 48_000.0,
                channels: 1,
            },
            dip_banks: vec![],
            dips: vec![],
            has_analog: false,
        }
    }

    #[test]
    fn only_used_players_and_buttons_are_shown() {
        let i = info(2, 3);
        assert_eq!(groups(&i), 3);
        let p1 = visible_actions(&i, 0);
        assert_eq!(p1.len(), 4 + 3 + 1);
        assert!(p1.contains(&Action::Player(0, Control::Button(2))));
        assert!(!p1.contains(&Action::Player(0, Control::Button(3))));
        assert!(visible_actions(&i, 1).iter().all(|a| a.player() == Some(1)));
        let sys = visible_actions(&i, 2);
        assert_eq!(
            sys,
            vec![
                Action::Coin(0),
                Action::Coin(1),
                Action::Service,
                Action::Test,
                Action::Tilt
            ]
        );
        // SF2 : 2 joueurs, 6 boutons ; captain commando : 4 joueurs
        assert_eq!(visible_actions(&info(2, 6), 0).len(), 11);
        assert_eq!(visible_actions(&info(4, 3), 4).len(), 4 + 3);
    }

    #[test]
    fn cell_texts() {
        let mut b = Bindings::default();
        let p1b1 = Action::Player(0, Control::Button(0));
        assert_eq!(key_cell(&b, p1b1, false), "Z");
        assert_eq!(key_cell(&b, p1b1, true), WAIT_KEY);
        assert_eq!(
            key_cell(&b, Action::Player(0, Control::Up), false),
            "Flèche haut"
        );
        assert_eq!(key_cell(&b, Action::Coin(0), false), "5");
        assert_eq!(button_cell(&b, p1b1, false), "South");
        assert_eq!(button_cell(&b, Action::Tilt, false), "—");
        assert_eq!(button_cell(&b, Action::Tilt, true), WAIT_BUTTON);
        b.set_key(Action::Tilt, Key::Enter);
        assert_eq!(key_cell(&b, Action::Player(0, Control::Start), false), "—");
    }

    #[test]
    fn closed_window_never_captures() {
        let mut w = ControlsWindow {
            open: false,
            capture: Some(Capture::Key(Action::Test)),
        };
        assert!(!w.capturing());
        w.open = true;
        assert!(w.capturing());
    }
}
