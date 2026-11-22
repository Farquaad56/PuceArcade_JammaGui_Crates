//! Entrées JAMMA — logique ACTIVE HAUTE (true = contact fermé = appuyé).
//! Chaque carte convertit vers ses registres (souvent actifs bas).

/// Contrôles d'un joueur unique.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerControls {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    /// Boutons 1 à 6 (index 0 = bouton 1).
    pub buttons: [bool; 6],
    pub start: bool,
}

/// Entrées complètes du connecteur JAMMA.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JammaInputs {
    /// 0 = joueur 1 (côté composants), 1 = joueur 2 (côté soudure), 2-3 = extensions.
    pub players: [PlayerControls; 4],
    /// Pièces 1 à 4.
    pub coins: [bool; 4],
    pub service: bool,
    pub test: bool,
    pub tilt: bool,
    /// Déplacements relatifs depuis la trame précédente (molette/trackball), unités de la carte.
    pub analog: [i32; 4],
}
