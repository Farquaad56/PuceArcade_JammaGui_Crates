//! Préférences d'affichage (reprises de ng-gui, adaptées au contrat JAMMA).

/// Mode d'échelle de l'image.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScaleMode {
    /// Plus grand facteur entier tenant dans la fenêtre.
    #[default]
    Auto,
    Fixed(u32),
}

/// Forme des pixels à l'affichage.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Aspect {
    /// Pixels carrés (pixel perfect).
    #[default]
    Square,
    /// Image étirée selon le rapport d'aspect du moniteur (non pixel perfect).
    Tv,
}
