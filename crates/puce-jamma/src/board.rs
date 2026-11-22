//! Métadonnées d'une carte et trait principal `JammaBoard`.

use crate::audio::AudioInfo;
use crate::dips::{DipBank, DipSwitch};
use crate::inputs::JammaInputs;
use crate::video::VideoInfo;

/// Informations sur une carte émulée.
#[derive(Clone, Debug, PartialEq)]
pub struct BoardInfo {
    /// Nom du système (ex. "Capcom CPS1").
    pub system: String,
    /// Nom court MAME (ex. "ghouls").
    pub name: String,
    /// Titre complet (ex. "Ghouls'n Ghosts (World)").
    pub title: String,
    /// Nombre de joueurs supportés (1..=4).
    pub players: u8,
    /// Boutons utilisés par joueur (0..=6), le frontal n'affiche que ceux-là.
    pub buttons: u8,
    /// Informations vidéo.
    pub video: VideoInfo,
    /// Informations audio.
    pub audio: AudioInfo,
    /// Banques DIP disponibles.
    pub dip_banks: Vec<DipBank>,
    /// Switches DIP décrits.
    pub dips: Vec<DipSwitch>,
    /// La carte utilise des entrées analogiques (molettes, trackballs).
    pub has_analog: bool,
}

/// Trait principal : une carte émulée implémente ce trait.
/// PAS `Send` : la carte est créée et utilisée dans le thread d'émulation.
pub trait JammaBoard {
    /// Informations sur la carte.
    fn info(&self) -> &BoardInfo;

    /// Réinitialise la carte (comme un reset matériel).
    fn reset(&mut self);

    /// Exécute exactement une trame vidéo.
    fn run_frame(&mut self);

    /// Nombre de trames exécutées depuis le début.
    fn frame_count(&self) -> u64;

    /// Dernière trame complète produite.
    fn video(&self) -> crate::video::VideoOut<'_>;

    /// Échantillons audio produits depuis le dernier appel, entrelacés selon `channels`, dans [-1, 1].
    fn take_audio(&mut self, out: &mut Vec<f32>);

    /// Applique les entrées avant la prochaine run_frame.
    fn set_inputs(&mut self, inputs: &JammaInputs);

    /// Valeurs actuelles des DIP switches (un octet par banque).
    fn dips(&self) -> Vec<u8>;

    /// Définit les valeurs des DIP switches.
    fn set_dips(&mut self, banks: &[u8]);

    /// Sauvegarde l'état complet de la carte.
    fn save_state(&mut self) -> Vec<u8>;

    /// Restaure un état sauvegardé.
    fn load_state(&mut self, data: &[u8]) -> Result<(), String>;

    /// Accès optionnel au débogage (None si non supporté).
    fn debug(&mut self) -> Option<&mut dyn crate::debug::BoardDebug> {
        None
    }
}
