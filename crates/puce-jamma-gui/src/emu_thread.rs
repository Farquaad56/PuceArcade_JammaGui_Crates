//! Types échangés avec le thread d'émulation, pas d'émulation unitaire et cadence.
//! Structure reprise de ref/puce16-neogeo/ng-gui/emu_thread.rs, sur le contrat JAMMA.
//! La boucle du thread est dans `emu_loop.rs`.

use std::path::PathBuf;
use std::time::Duration;

use puce_jamma::board::{BoardInfo, JammaBoard};
use puce_jamma::debug::{CpuView, DebugImage};
use puce_jamma::inputs::JammaInputs;
use puce_jamma::video::VideoInfo;

/// Commande envoyée au thread d'émulation.
#[derive(Clone, Debug, PartialEq)]
pub enum Cmd {
    /// Charge un jeu du driver (arrête le précédent).
    Load(String),
    Reset,
    Pause(bool),
    /// Commandes appliquées avant chaque trame.
    Inputs(JammaInputs),
    /// Un octet par banque de DIP.
    Dips(Vec<u8>),
    SaveState(u8),
    LoadState(u8),
    /// En pause : une instruction du CPU donné (si la carte a un débogueur), puis instantané de débogage.
    Step(usize),
    /// Débogueur ouvert : instantanés envoyés régulièrement en marche (≈ 6 par seconde),
    /// à la mise en pause et dès l'activation.
    DebugLive(bool),
    /// En pause : une trame, puis instantané de débogage.
    FrameAdvance,
    /// Couches visibles (débogueur), bit n = couche n.
    LayerMask(u32),
    /// Capture PNG de l'image courante.
    Screenshot,
    /// Lecture de `len` octets (au plus `MAX_PEEK`) à `addr` dans l'espace du CPU `cpu`, sans effet de bord.
    Peek {
        cpu: usize,
        addr: u32,
        len: usize,
    },
    Quit,
}

/// Taille maximale d'une lecture `Peek`.
pub const MAX_PEEK: usize = 4096;

/// Événement publié par le thread d'émulation.
#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Loaded(BoardInfo),
    LoadError(String),
    Frame {
        pixels: Vec<u32>,
        info: VideoInfo,
        frame: u64,
    },
    /// Échantillons entrelacés selon `BoardInfo::audio.channels`, dans [-1, 1].
    Audio(Vec<f32>),
    /// Temps de calcul de la dernière trame, en ms.
    Stats {
        emu_ms: f32,
    },
    Saved(u8),
    StateError(String),
    ScreenshotSaved(PathBuf),
    Debug(DebugSnapshot),
    /// Réponse à `Peek` (vide si la carte n'a pas de débogueur).
    Memory(Vec<u8>),
}

/// Instantané de débogage (vide si la carte n'a pas de débogueur).
#[derive(Clone, Debug, PartialEq, Default)]
pub struct DebugSnapshot {
    pub cpus: Vec<CpuView>,
    pub registers: Vec<(String, Vec<(String, String)>)>,
    pub images: Vec<DebugImage>,
    pub layer_names: Vec<String>,
}

impl DebugSnapshot {
    /// Instantané de la carte (vide si elle n'a pas de débogueur).
    pub fn of(board: &mut dyn JammaBoard) -> DebugSnapshot {
        match board.debug() {
            Some(d) => DebugSnapshot {
                cpus: d.cpus(),
                registers: d.registers(),
                images: d.images(),
                layer_names: d.layer_names(),
            },
            None => DebugSnapshot::default(),
        }
    }
}

/// Une trame : `set_inputs`, `run_frame`, copie de l'image, `take_audio` (ordre du contrat).
pub fn step_once(board: &mut dyn JammaBoard, inputs: &JammaInputs) -> (Vec<u32>, Vec<f32>) {
    board.set_inputs(inputs);
    board.run_frame();
    let pixels = board.video().pixels.to_vec();
    let mut audio = Vec::new();
    board.take_audio(&mut audio);
    (pixels, audio)
}

/// Durée d'une trame à `refresh_hz`.
pub fn frame_period(refresh_hz: f64) -> Duration {
    Duration::from_nanos((1e9 / refresh_hz).round() as u64)
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma::video::Rotation;
    use puce_jamma_testboard::TestBoard;

    #[test]
    fn frame_period_values() {
        assert_eq!(frame_period(60.0), Duration::from_nanos(16_666_667));
        assert_eq!(frame_period(59.637), Duration::from_nanos(16_768_114));
    }

    #[test]
    fn step_once_on_testboard() {
        let mut b = TestBoard::new(Rotation::Rot0);
        let (pixels, audio) = step_once(&mut b, &JammaInputs::default());
        assert_eq!(pixels.len(), 288 * 224);
        assert_eq!(audio.len(), 800);
        assert_eq!(b.frame_count(), 1);
    }

    #[test]
    fn debug_snapshot_of_testboard() {
        let mut b = TestBoard::new(Rotation::Rot0);
        let s = DebugSnapshot::of(&mut b);
        assert!(!s.cpus.is_empty());
    }
}
