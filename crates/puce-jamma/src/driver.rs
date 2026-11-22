//! Catalogue et chargement des jeux (côté « système »).

/// Entrée du catalogue de jeux.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameEntry {
    /// Nom court MAME.
    pub name: String,
    /// Titre complet.
    pub title: String,
    /// Nom du parent (clone of), vide si original.
    pub parent: String,
    /// Les ROMs sont disponibles sur le système de fichiers.
    pub available: bool,
    /// Le jeu est supporté par l'émulateur.
    pub supported: bool,
}

/// Driver d'un système (catalogue + chargement).
pub trait BoardDriver: Send + Sync {
    /// Nom du système (ex. "Capcom CPS1").
    fn system_name(&self) -> &str;

    /// Liste des jeux, triés dans l'ordre d'affichage voulu par le système.
    fn games(&self) -> Vec<GameEntry>;

    /// Jeu par défaut à charger au démarrage.
    fn default_game(&self) -> Option<String>;

    /// Charge un jeu (appelé DANS le thread d'émulation).
    fn load(&self, name: &str) -> Result<Box<dyn crate::board::JammaBoard>, String>;

    /// Répertoire des savestates pour ce système.
    fn saves_dir(&self) -> std::path::PathBuf;
}
