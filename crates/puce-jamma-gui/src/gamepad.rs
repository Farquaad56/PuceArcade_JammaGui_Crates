//! Manettes (gilrs) : manette n -> joueur n (n ≤ 4). La croix et le stick gauche donnent les directions,
//! les autres boutons suivent les assignations (`Bindings`). Repris de ref/puce16-neogeo/ng-gui/input.rs.

use gilrs::{Axis, Button, EventType, Gilrs};

use crate::bindings::Bindings;
use crate::input::{Action, Control};

/// Seuil du stick analogique au-delà duquel une direction est pressée.
pub const STICK_THRESHOLD: f32 = 0.5;

/// Directions données par le stick gauche (axe Y de gilrs : haut positif).
pub fn stick_directions(x: f32, y: f32) -> [bool; 4] {
    [
        y > STICK_THRESHOLD,
        y < -STICK_THRESHOLD,
        x < -STICK_THRESHOLD,
        x > STICK_THRESHOLD,
    ]
}

/// Actions pressées par la manette du joueur `player`, d'après ses boutons et son stick.
pub fn pad_actions(
    player: usize,
    b: &Bindings,
    pressed: impl Fn(Button) -> bool,
    stick: (f32, f32),
) -> Vec<Action> {
    let mut out: Vec<Action> = Action::all()
        .filter(|a| a.player() == Some(player))
        .filter(|a| b.button(*a).is_some_and(&pressed))
        .collect();
    let dirs = [Control::Up, Control::Down, Control::Left, Control::Right];
    for (d, on) in dirs.into_iter().zip(stick_directions(stick.0, stick.1)) {
        let a = Action::Player(player as u8, d);
        if on && !out.contains(&a) {
            out.push(a);
        }
    }
    out
}

/// Manettes connectées ; absentes si gilrs ne peut pas s'initialiser.
pub struct Gamepads {
    gilrs: Option<Gilrs>,
    /// Dernier bouton pressé (capture de la fenêtre de configuration) : (manette, bouton).
    last: Option<(usize, Button)>,
}

impl Default for Gamepads {
    fn default() -> Self {
        Gamepads {
            gilrs: Gilrs::new().ok(),
            last: None,
        }
    }
}

impl Gamepads {
    /// Lit les événements et renvoie les actions pressées par les 4 premières manettes.
    pub fn poll(&mut self, b: &Bindings) -> Vec<Action> {
        let Some(g) = self.gilrs.as_mut() else {
            return Vec::new();
        };
        while let Some(ev) = g.next_event() {
            if let EventType::ButtonPressed(btn, _) = ev.event {
                let n = g.gamepads().position(|(id, _)| id == ev.id).unwrap_or(0);
                self.last = Some((n, btn));
            }
        }
        let mut out = Vec::new();
        for (n, (_, pad)) in g.gamepads().take(4).enumerate() {
            let stick = (pad.value(Axis::LeftStickX), pad.value(Axis::LeftStickY));
            out.extend(pad_actions(n, b, |btn| pad.is_pressed(btn), stick));
        }
        out
    }

    /// Bouton pressé depuis le dernier appel : (numéro de manette, bouton).
    pub fn take_pressed_button(&mut self) -> Option<(usize, Button)> {
        self.last.take()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stick_threshold() {
        assert_eq!(stick_directions(0.0, 0.0), [false; 4]);
        assert_eq!(stick_directions(0.6, 0.0), [false, false, false, true]);
        assert_eq!(stick_directions(-0.6, 0.7), [true, false, true, false]);
        assert_eq!(stick_directions(0.5, -0.5), [false; 4]);
    }

    #[test]
    fn default_mapping_per_player() {
        let b = Bindings::default();
        let acts = pad_actions(
            2,
            &b,
            |btn| matches!(btn, Button::South | Button::Select | Button::RightTrigger),
            (0.0, -1.0),
        );
        assert!(acts.contains(&Action::Player(2, Control::Button(0))));
        assert!(acts.contains(&Action::Player(2, Control::Button(5))));
        assert!(acts.contains(&Action::Coin(2)));
        assert!(acts.contains(&Action::Player(2, Control::Down)));
        assert!(acts.iter().all(|a| a.player() == Some(2)));
    }
}
