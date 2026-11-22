//! Assignations action -> touche et action -> bouton de manette, et lien avec `prefs.toml`.
//! Repris de ref/puce16-neogeo/ng-gui/input.rs (`Bindings`) : une touche déjà prise réaffectée à une
//! autre action est retirée de l'ancienne, qui reçoit en échange la touche précédente de la nouvelle.

use eframe::egui::Key;
use gilrs::Button;

use crate::input::{button_name, key_name, parse_button, parse_key, Action, N_ACTIONS};
use crate::prefs::InputPrefs;

/// Valeur de prefs.toml pour « aucune assignation ».
const NONE: &str = "none";

#[derive(Clone, Debug, PartialEq)]
pub struct Bindings {
    keys: Vec<Option<Key>>,
    pad: Vec<Option<Button>>,
}

impl Default for Bindings {
    fn default() -> Self {
        let mut b = Bindings {
            keys: vec![None; N_ACTIONS],
            pad: vec![None; N_ACTIONS],
        };
        b.reset(|_| true);
        b
    }
}

impl Bindings {
    pub fn key(&self, a: Action) -> Option<Key> {
        self.keys[a.index()]
    }

    pub fn button(&self, a: Action) -> Option<Button> {
        self.pad[a.index()]
    }

    /// Assigne `key` à `a`. Si une autre action l'utilisait, elle reçoit l'ancienne touche de `a`.
    pub fn set_key(&mut self, a: Action, key: Key) {
        let old = self.keys[a.index()];
        if let Some(o) = Action::all().find(|o| *o != a && self.keys[o.index()] == Some(key)) {
            self.keys[o.index()] = old;
        }
        self.keys[a.index()] = Some(key);
    }

    /// Comme `set_key`, l'échange ne concernant que les actions de la même manette (même joueur).
    pub fn set_button(&mut self, a: Action, btn: Button) {
        let old = self.pad[a.index()];
        let other = Action::all()
            .find(|o| *o != a && o.player() == a.player() && self.pad[o.index()] == Some(btn));
        if let Some(o) = other {
            self.pad[o.index()] = old;
        }
        self.pad[a.index()] = Some(btn);
    }

    pub fn clear_key(&mut self, a: Action) {
        self.keys[a.index()] = None;
    }

    pub fn clear_button(&mut self, a: Action) {
        self.pad[a.index()] = None;
    }

    /// Remet les valeurs par défaut des actions retenues par `which`.
    pub fn reset(&mut self, which: impl Fn(Action) -> bool) {
        for a in Action::all().filter(|a| which(*a)) {
            self.keys[a.index()] = a.default_key();
            self.pad[a.index()] = a.default_button();
        }
    }

    /// Assignations lues dans prefs.toml ; une action ou une touche inconnue garde sa valeur par défaut.
    pub fn from_prefs(p: &InputPrefs) -> Bindings {
        let mut b = Bindings::default();
        for (action, value) in &p.keys {
            let Some(a) = Action::from_name(action) else {
                continue;
            };
            if value == NONE {
                b.keys[a.index()] = None;
            } else if let Some(k) = parse_key(value) {
                b.keys[a.index()] = Some(k);
            }
        }
        for (action, value) in &p.pad {
            let Some(a) = Action::from_name(action) else {
                continue;
            };
            if value == NONE {
                b.pad[a.index()] = None;
            } else if let Some(btn) = parse_button(value) {
                b.pad[a.index()] = Some(btn);
            }
        }
        b
    }

    /// Assignations à écrire dans prefs.toml (`mouse_sens` est conservé tel quel par l'appelant).
    pub fn to_prefs(&self, mouse_sens: Option<u32>) -> InputPrefs {
        let names = |f: &dyn Fn(Action) -> Option<String>| {
            Action::all()
                .map(|a| (a.name(), f(a).unwrap_or_else(|| NONE.into())))
                .collect()
        };
        InputPrefs {
            keys: names(&|a| self.key(a).map(key_name)),
            pad: names(&|a| self.button(a).map(|b| button_name(b).to_string())),
            mouse_sens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::input::Control;

    const P1B1: Action = Action::Player(0, Control::Button(0));
    const P1B2: Action = Action::Player(0, Control::Button(1));

    #[test]
    fn prefs_round_trip() {
        let mut b = Bindings::default();
        b.set_key(P1B1, Key::F5);
        b.clear_key(Action::Coin(1));
        b.set_button(Action::Tilt, Button::Mode);
        let p = b.to_prefs(Some(150));
        assert_eq!(p.mouse_sens, Some(150));
        assert_eq!(Bindings::from_prefs(&p), b);
        assert_eq!(
            Bindings::from_prefs(&InputPrefs::default()),
            Bindings::default()
        );
    }

    #[test]
    fn reassigned_key_leaves_its_old_action() {
        let mut b = Bindings::default();
        b.set_key(P1B2, Key::Z); // Z était à J1 B1, X à J1 B2
        assert_eq!(b.key(P1B2), Some(Key::Z));
        assert_eq!(b.key(P1B1), Some(Key::X));
        // une action sans touche prend celle d'une autre : l'autre n'en a plus
        b.set_key(Action::Tilt, Key::Enter);
        assert_eq!(b.key(Action::Player(0, Control::Start)), None);
        // manettes : échange seulement sur la manette du même joueur
        b.set_button(P1B1, Button::East);
        assert_eq!(b.button(P1B2), Some(Button::South));
        assert_eq!(
            b.button(Action::Player(1, Control::Button(1))),
            Some(Button::East)
        );
    }

    #[test]
    fn invalid_prefs_entries_keep_defaults() {
        let p = InputPrefs {
            keys: vec![
                ("P1B1".into(), "PasUneTouche".into()),
                ("Inconnue".into(), "Q".into()),
                ("P1B2".into(), "F7".into()),
            ],
            pad: vec![("P1B1".into(), "Bidule".into())],
            mouse_sens: None,
        };
        let b = Bindings::from_prefs(&p);
        assert_eq!(b.key(P1B1), Some(Key::Z));
        assert_eq!(b.key(P1B2), Some(Key::F7));
        assert_eq!(b.button(P1B1), Some(Button::South));
    }
}
