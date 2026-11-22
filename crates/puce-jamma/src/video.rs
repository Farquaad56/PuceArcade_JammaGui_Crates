//! Métadonnées vidéo et sortie d'image.

/// Rotation de l'image (sens horaire, comme ROTxx de MAME).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rotation {
    Rot0,
    Rot90,
    Rot180,
    Rot270,
}

/// Informations vidéo d'une carte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoInfo {
    /// Largeur de l'image visible native (avant rotation).
    pub width: u32,
    /// Hauteur de l'image visible native (avant rotation).
    pub height: u32,
    /// Fréquence de rafraîchissement en Hz.
    pub refresh_hz: f64,
    /// Rapport d'affichage du moniteur après rotation (ex. 4:3 ou 3:4).
    pub aspect: (u32, u32),
    /// Rotation appliquée à l'image native.
    pub rotation: Rotation,
}

/// Sortie vidéo d'une trame complète.
pub struct VideoOut<'a> {
    pub info: VideoInfo,
    /// Pixels en 0x00RRGGBB, width*height, ligne par ligne.
    pub pixels: &'a [u32],
}
