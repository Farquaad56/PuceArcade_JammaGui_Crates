//! Métadonnées audio.

/// Informations audio d'une carte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AudioInfo {
    /// Fréquence d'échantillonnage en Hz.
    pub sample_rate: f64,
    /// Nombre de canaux (1 = haut-parleur JAMMA mono, 2 = stéréo).
    pub channels: u8,
}
