//! Commandes JAMMA : 51 actions (4 joueurs × 11 + 7 système), touches et boutons par défaut,
//! assignations (préférences) et conversion clavier + manettes -> `JammaInputs` (logique active haute).
//! Repris de ref/puce16-neogeo/ng-gui/input.rs : mêmes noms de touches (egui) et de boutons (gilrs),
//! même échange quand une touche déjà prise est réaffectée ; actions Neo Geo remplacées par celles du JAMMA.

use std::collections::HashSet;

use eframe::egui::Key;
use gilrs::Button;
use puce_jamma::inputs::JammaInputs;

/// Nombre d'actions : 4 joueurs × (4 directions + 6 boutons + Start) + 4 pièces + Service + Test + Tilt.
pub const N_ACTIONS: usize = 51;
/// Actions par joueur.
const PER_PLAYER: usize = 11;

/// Commande d'un joueur.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Control {
    Up,
    Down,
    Left,
    Right,
    /// Bouton 1 à 6 (index 0 à 5).
    Button(u8),
    Start,
}

/// Action JAMMA assignable.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    /// Joueur 0 à 3.
    Player(u8, Control),
    /// Pièce 1 à 4 (index 0 à 3).
    Coin(u8),
    Service,
    Test,
    Tilt,
}

const CONTROLS: [Control; PER_PLAYER] = [
    Control::Up,
    Control::Down,
    Control::Left,
    Control::Right,
    Control::Button(0),
    Control::Button(1),
    Control::Button(2),
    Control::Button(3),
    Control::Button(4),
    Control::Button(5),
    Control::Start,
];

/// Boutons de manette assignables et leur nom dans prefs.toml (noms de l'original).
const BUTTONS: [(Button, &str); 19] = [
    (Button::South, "South"),
    (Button::East, "East"),
    (Button::North, "North"),
    (Button::West, "West"),
    (Button::C, "C"),
    (Button::Z, "Z"),
    (Button::LeftTrigger, "LeftTrigger"),
    (Button::LeftTrigger2, "LeftTrigger2"),
    (Button::RightTrigger, "RightTrigger"),
    (Button::RightTrigger2, "RightTrigger2"),
    (Button::Select, "Select"),
    (Button::Start, "Start"),
    (Button::Mode, "Mode"),
    (Button::LeftThumb, "LeftThumb"),
    (Button::RightThumb, "RightThumb"),
    (Button::DPadUp, "DPadUp"),
    (Button::DPadDown, "DPadDown"),
    (Button::DPadLeft, "DPadLeft"),
    (Button::DPadRight, "DPadRight"),
];

impl Action {
    /// Toutes les actions, dans l'ordre d'affichage (joueurs 1 à 4, puis système).
    pub fn all() -> impl Iterator<Item = Action> {
        (0..N_ACTIONS).map(Action::from_index)
    }

    pub fn index(self) -> usize {
        match self {
            Action::Player(p, c) => {
                p as usize * PER_PLAYER + CONTROLS.iter().position(|x| *x == c).unwrap_or(0)
            }
            Action::Coin(n) => 44 + n as usize,
            Action::Service => 48,
            Action::Test => 49,
            Action::Tilt => 50,
        }
    }

    pub fn from_index(i: usize) -> Action {
        match i {
            0..=43 => Action::Player((i / PER_PLAYER) as u8, CONTROLS[i % PER_PLAYER]),
            44..=47 => Action::Coin((i - 44) as u8),
            48 => Action::Service,
            49 => Action::Test,
            _ => Action::Tilt,
        }
    }

    /// Nom stable écrit dans prefs.toml : `P1Up`, `P2B3`, `P1Start`, `Coin1`, `Service`, `Test`, `Tilt`.
    pub fn name(self) -> String {
        match self {
            Action::Player(p, c) => {
                let c = match c {
                    Control::Up => "Up".to_string(),
                    Control::Down => "Down".to_string(),
                    Control::Left => "Left".to_string(),
                    Control::Right => "Right".to_string(),
                    Control::Button(b) => format!("B{}", b + 1),
                    Control::Start => "Start".to_string(),
                };
                format!("P{}{c}", p + 1)
            }
            Action::Coin(n) => format!("Coin{}", n + 1),
            Action::Service => "Service".into(),
            Action::Test => "Test".into(),
            Action::Tilt => "Tilt".into(),
        }
    }

    pub fn from_name(name: &str) -> Option<Action> {
        Action::all().find(|a| a.name() == name)
    }

    /// Libellé affiché.
    pub fn label(self) -> String {
        match self {
            Action::Player(p, c) => {
                let c = match c {
                    Control::Up => "Haut".to_string(),
                    Control::Down => "Bas".to_string(),
                    Control::Left => "Gauche".to_string(),
                    Control::Right => "Droite".to_string(),
                    Control::Button(b) => format!("Bouton {}", b + 1),
                    Control::Start => "Start".to_string(),
                };
                format!("J{} {c}", p + 1)
            }
            Action::Coin(n) => format!("Pièce {}", n + 1),
            Action::Service => "Service".into(),
            Action::Test => "Test".into(),
            Action::Tilt => "Tilt".into(),
        }
    }

    /// Joueur (0 à 3) dont la manette porte cette action ; `None` pour Service, Test et Tilt.
    pub fn player(self) -> Option<usize> {
        match self {
            Action::Player(p, _) => Some(p as usize),
            Action::Coin(n) => Some(n as usize),
            _ => None,
        }
    }

    pub fn default_key(self) -> Option<Key> {
        use Control::*;
        const P1: [Key; 6] = [Key::Z, Key::X, Key::A, Key::S, Key::Q, Key::W];
        const P2: [Key; 6] = [Key::G, Key::H, Key::T, Key::Y, Key::U, Key::O];
        Some(match self {
            Action::Player(0, c) => match c {
                Up => Key::ArrowUp,
                Down => Key::ArrowDown,
                Left => Key::ArrowLeft,
                Right => Key::ArrowRight,
                Button(b) => P1[b as usize],
                Start => Key::Enter,
            },
            Action::Player(1, c) => match c {
                Up => Key::I,
                Down => Key::K,
                Left => Key::J,
                Right => Key::L,
                Button(b) => P2[b as usize],
                Start => Key::P,
            },
            Action::Coin(n) => [Key::Num5, Key::Num6, Key::Num7, Key::Num8][n as usize],
            Action::Service => Key::Num9,
            Action::Test => Key::F2,
            _ => return None,
        })
    }

    /// Manette n -> joueur n : croix, South/East/West/North/gâchettes = B1..B6, Start, Select = pièce.
    pub fn default_button(self) -> Option<Button> {
        const B: [Button; 6] = [
            Button::South,
            Button::East,
            Button::West,
            Button::North,
            Button::LeftTrigger,
            Button::RightTrigger,
        ];
        Some(match self {
            Action::Player(_, c) => match c {
                Control::Up => Button::DPadUp,
                Control::Down => Button::DPadDown,
                Control::Left => Button::DPadLeft,
                Control::Right => Button::DPadRight,
                Control::Button(b) => B[b as usize],
                Control::Start => Button::Start,
            },
            Action::Coin(_) => Button::Select,
            _ => return None,
        })
    }
}

/// Nom d'une touche dans prefs.toml (nom egui).
pub fn key_name(key: Key) -> String {
    key.name().to_string()
}

pub fn parse_key(name: &str) -> Option<Key> {
    Key::from_name(name)
}

pub fn button_name(b: Button) -> &'static str {
    BUTTONS
        .iter()
        .find(|(x, _)| *x == b)
        .map_or("?", |(_, n)| n)
}

pub fn parse_button(name: &str) -> Option<Button> {
    BUTTONS.iter().find(|(_, n)| *n == name).map(|(b, _)| *b)
}

/// Applique une action pressée à `j`.
pub fn press(j: &mut JammaInputs, a: Action) {
    match a {
        Action::Player(p, c) => {
            let pc = &mut j.players[p as usize];
            match c {
                Control::Up => pc.up = true,
                Control::Down => pc.down = true,
                Control::Left => pc.left = true,
                Control::Right => pc.right = true,
                Control::Button(b) => pc.buttons[b as usize] = true,
                Control::Start => pc.start = true,
            }
        }
        Action::Coin(n) => j.coins[n as usize] = true,
        Action::Service => j.service = true,
        Action::Test => j.test = true,
        Action::Tilt => j.tilt = true,
    }
}

/// État du clavier (touches maintenues) et des manettes (actions pressées).
#[derive(Default)]
pub struct InputState {
    keys: HashSet<Key>,
    /// Actions pressées par les manettes (rempli par `gamepad::Gamepads::poll`).
    pub pad_pressed: Vec<Action>,
}

impl InputState {
    /// Remplace les touches maintenues (relevées dans egui : `i.keys_down`).
    pub fn set_keys(&mut self, keys: HashSet<Key>) {
        self.keys = keys;
    }

    /// Simule l'état d'une touche (tests, fenêtre de configuration).
    pub fn set_key(&mut self, key: Key, down: bool) {
        if down {
            self.keys.insert(key);
        } else {
            self.keys.remove(&key);
        }
    }

    /// Commandes JAMMA selon les assignations clavier (les manettes s'ajoutent via `pad_pressed`).
    pub fn to_jamma(&self, b: &crate::bindings::Bindings) -> JammaInputs {
        let mut j = JammaInputs::default();
        for a in Action::all() {
            if b.key(a).is_some_and(|k| self.keys.contains(&k)) {
                press(&mut j, a);
            }
        }
        for &a in &self.pad_pressed {
            press(&mut j, a);
        }
        j
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bindings::Bindings;

    #[test]
    fn fifty_one_actions_with_unique_names() {
        let names: HashSet<String> = Action::all().map(|a| a.name()).collect();
        assert_eq!(names.len(), N_ACTIONS);
        for (i, a) in Action::all().enumerate() {
            assert_eq!(a.index(), i);
            assert_eq!(Action::from_name(&a.name()), Some(a));
        }
        assert_eq!(Action::from_index(0).name(), "P1Up");
        assert_eq!(Action::from_index(15).name(), "P2B1"); // 11 actions par joueur : 15 = J2, 5e action
        assert_eq!(Action::from_index(43).name(), "P4Start");
    }

    #[test]
    fn no_key_means_default_inputs() {
        let i = InputState::default();
        assert_eq!(i.to_jamma(&Bindings::default()), JammaInputs::default());
    }

    #[test]
    fn default_keys() {
        let b = Bindings::default();
        let mut i = InputState::default();
        i.set_key(Key::ArrowRight, true);
        i.set_key(Key::Z, true);
        let j = i.to_jamma(&b);
        assert!(j.players[0].right && j.players[0].buttons[0]);
        assert!(!j.players[0].left && !j.players[0].buttons[1]);
        let only = |k: Key| {
            let mut i = InputState::default();
            i.set_key(k, true);
            i.to_jamma(&b)
        };
        assert!(only(Key::Q).players[0].buttons[4]);
        assert!(only(Key::Num5).coins[0]);
        assert!(only(Key::F2).test);
        assert!(only(Key::O).players[1].buttons[5]);
        assert!(only(Key::P).players[1].start);
        assert!(only(Key::Num9).service);
        assert_eq!(Action::Tilt.default_key(), None);
        assert_eq!(Action::Player(2, Control::Up).default_key(), None);
    }

    #[test]
    fn gamepad_actions_are_added() {
        let i = InputState {
            pad_pressed: vec![Action::Player(3, Control::Button(5)), Action::Coin(3)],
            ..InputState::default()
        };
        let j = i.to_jamma(&Bindings::default());
        assert!(j.players[3].buttons[5] && j.coins[3]);
    }

    #[test]
    fn key_and_button_names_round_trip() {
        for a in Action::all() {
            if let Some(k) = a.default_key() {
                assert_eq!(parse_key(&key_name(k)), Some(k), "{k:?}");
            }
            if let Some(b) = a.default_button() {
                assert_eq!(parse_button(button_name(b)), Some(b));
            }
        }
    }
}
