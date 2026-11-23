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
    /// Catégorie : original, clone officiel, bootleg, hack ou prototype (ADR 002).
    pub kind: GameKind,
    /// Système du jeu quand un driver en sert plusieurs (« CPS1 », « CPS2 »…), vide sinon (ADR 003).
    pub system: String,
}

/// Catégorie d'un jeu du catalogue (ADR 002).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub enum GameKind {
    /// Version commerciale de référence (parent MAME).
    #[default]
    Original,
    /// Autre version commerciale du même jeu : région, révision, conversion officielle.
    Clone,
    /// Copie ou modification non autorisée vendue comme carte (bootleg).
    Bootleg,
    /// Programme modifié (hack).
    Hack,
    /// Prototype jamais commercialisé tel quel.
    Prototype,
}

impl GameKind {
    pub const ALL: [GameKind; 5] = [
        GameKind::Original,
        GameKind::Clone,
        GameKind::Bootleg,
        GameKind::Hack,
        GameKind::Prototype,
    ];

    /// Libellé court (pluriel) pour les filtres.
    pub fn label(self) -> &'static str {
        match self {
            GameKind::Original => "Originaux",
            GameKind::Clone => "Clones officiels",
            GameKind::Bootleg => "Bootlegs",
            GameKind::Hack => "Hacks",
            GameKind::Prototype => "Prototypes",
        }
    }

    /// Version commerciale officielle (original ou clone officiel) ?
    pub fn commercial(self) -> bool {
        matches!(self, GameKind::Original | GameKind::Clone)
    }

    /// Catégorie déduite des champs d'une ligne `GAME` de MAME : parent (« clone of »),
    /// constructeur et titre. Hack si le constructeur ou le titre le dit, puis bootleg, puis
    /// prototype (titre), sinon clone si le jeu a un parent, original sinon.
    pub fn from_mame(parent: &str, maker: &str, title: &str) -> GameKind {
        let (m, t) = (maker.to_lowercase(), title.to_lowercase());
        if m.contains("hack") || t.contains("hack") {
            GameKind::Hack
        } else if m.contains("bootleg") || t.contains("bootleg") {
            GameKind::Bootleg
        } else if t.contains("prototype") {
            GameKind::Prototype
        } else if parent.is_empty() {
            GameKind::Original
        } else {
            GameKind::Clone
        }
    }
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

#[cfg(test)]
mod tests {
    use super::GameKind;

    #[test]
    fn kinds_from_mame_fields() {
        let k = GameKind::from_mame;
        assert_eq!(
            k("", "Capcom", "Street Fighter II (World 910522)"),
            GameKind::Original
        );
        assert_eq!(
            k("sf2", "Capcom", "Street Fighter II (USA 910206)"),
            GameKind::Clone
        );
        assert_eq!(
            k(
                "sf2",
                "Capcom",
                "Street Fighter II (World 910204, conversion)"
            ),
            GameKind::Clone
        );
        assert_eq!(
            k("sf2ce", "bootleg", "Street Fighter II': Magic KO Turbo!!"),
            GameKind::Bootleg
        );
        assert_eq!(k("", "bootleg", "CPS1 Multi Game"), GameKind::Bootleg);
        assert_eq!(
            k(
                "ffight",
                "hack",
                "Final Fight 30th Anniversary Edition (World, hack)"
            ),
            GameKind::Hack
        );
        assert_eq!(
            k(
                "ffight",
                "Capcom",
                "Street Smart / Final Fight (Japan, hack)"
            ),
            GameKind::Hack
        );
        assert_eq!(
            k(
                "forgottn",
                "Capcom",
                "Forgotten Worlds (Japan) (English prototype)"
            ),
            GameKind::Prototype
        );
        assert!(GameKind::Clone.commercial() && !GameKind::Bootleg.commercial());
    }
}
