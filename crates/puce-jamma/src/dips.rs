//! Description des DIP switches pour le menu du frontal.

/// Une option d'un switch DIP.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DipSetting {
    pub label: String,
    pub value: u8,
}

/// Un switch DIP complet (description + valeurs possibles).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DipSwitch {
    /// Index de la banque DIP concernée.
    pub bank: usize,
    /// Masque des bits du switch dans l'octet de la banque.
    pub mask: u8,
    /// Libellé affiché au joueur.
    pub label: String,
    /// Valeur par défaut (bits masqués).
    pub default: u8,
    /// Options possibles.
    pub settings: Vec<DipSetting>,
}

/// Une banque DIP (ex. "SW(A)" / "DSWA").
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DipBank {
    pub name: String,
    /// Valeur par défaut de l'octet complet.
    pub default: u8,
}
