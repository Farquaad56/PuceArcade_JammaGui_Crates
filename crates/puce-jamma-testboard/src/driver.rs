//! Driver pour la carte de test JAMMA (mire).

use std::path::{Path, PathBuf};

use puce_jamma::board::JammaBoard;
use puce_jamma::driver::{BoardDriver, GameEntry, GameKind};
use puce_jamma::video::Rotation;

use crate::TestBoard;

/// Driver pour la carte de test JAMMA.
pub struct TestDriver {
    saves: PathBuf,
}

impl TestDriver {
    /// Crée un nouveau driver avec le répertoire de sauvegarde spécifié.
    pub fn new(saves_dir: impl AsRef<Path>) -> Self {
        Self {
            saves: saves_dir.as_ref().to_path_buf(),
        }
    }
}

impl BoardDriver for TestDriver {
    fn system_name(&self) -> &str {
        "Mire JAMMA"
    }

    fn games(&self) -> Vec<GameEntry> {
        vec![
            GameEntry {
                name: "mire".to_string(),
                title: "Mire de test JAMMA (horizontal)".to_string(),
                parent: "".to_string(),
                available: true,
                supported: true,
                kind: GameKind::Original,
                system: String::new(),
            },
            GameEntry {
                name: "mire_v".to_string(),
                title: "Mire de test JAMMA (vertical)".to_string(),
                parent: "".to_string(),
                available: true,
                supported: true,
                kind: GameKind::Original,
                system: String::new(),
            },
        ]
    }

    fn default_game(&self) -> Option<String> {
        Some("mire".to_string())
    }

    fn load(&self, name: &str) -> Result<Box<dyn JammaBoard>, String> {
        match name {
            "mire" => Ok(Box::new(TestBoard::new(Rotation::Rot0))),
            "mire_v" => Ok(Box::new(TestBoard::new(Rotation::Rot270))),
            _ => Err(format!("Jeu inconnu : {}", name)),
        }
    }

    fn saves_dir(&self) -> PathBuf {
        self.saves.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_driver_games() {
        let driver = TestDriver::new("/tmp/saves");
        assert_eq!(driver.games().len(), 2);
        assert_eq!(driver.system_name(), "Mire JAMMA");
        assert_eq!(driver.default_game(), Some("mire".to_string()));
    }

    #[test]
    fn test_driver_load_mire() {
        let driver = TestDriver::new("/tmp/saves");
        let board = driver.load("mire").unwrap();
        assert_eq!(board.info().name, "mire");
        assert_eq!(board.info().video.rotation, Rotation::Rot0);
        assert_eq!(board.info().video.aspect, (4, 3));
    }

    #[test]
    fn test_driver_load_mire_v() {
        let driver = TestDriver::new("/tmp/saves");
        let board = driver.load("mire_v").unwrap();
        assert_eq!(board.info().name, "mire_v");
        assert_eq!(board.info().video.rotation, Rotation::Rot270);
        assert_eq!(board.info().video.aspect, (3, 4));
    }

    #[test]
    fn test_driver_load_unknown() {
        let driver = TestDriver::new("/tmp/saves");
        let result = driver.load("zz");
        assert!(result.is_err());
    }
}
