//! Interface de débogage optionnelle pour les cartes émulées.

/// Vue d'un CPU (programme counter + registres).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CpuView {
    pub name: String,
    pub pc: u32,
    /// Paires (nom du registre, valeur).
    pub registers: Vec<(String, u32)>,
    /// Nombre de bits d'adresse.
    pub address_bits: u8,
}

/// Image de débogage (palette, tuiles, sprites, couches…).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DebugImage {
    pub name: String,
    pub width: u32,
    pub height: u32,
    /// Pixels en 0x00RRGGBB.
    pub pixels: Vec<u32>,
}

/// Interface de débogage (facultative).
pub trait BoardDebug {
    /// Vues des CPUs.
    fn cpus(&self) -> Vec<CpuView>;

    /// Lecture mémoire sans effet de bord (registres d'E/S : 0xFF).
    fn peek(&self, cpu: usize, addr: u32) -> u8;

    /// Exécute une instruction du CPU (les autres puces suivent).
    fn step_instruction(&mut self, cpu: usize);

    /// Groupes nommés de registres matériels (« CPS-A », « CPS-B »…).
    fn registers(&self) -> Vec<(String, Vec<(String, String)>)>;

    /// Images de débogage (palettes, tuiles, sprites, couches…).
    fn images(&self) -> Vec<DebugImage>;

    /// Noms des couches vidéo.
    fn layer_names(&self) -> Vec<String>;

    /// Masque de visibilité des couches (bit n = couche n visible ; défaut : toutes).
    fn set_layer_mask(&mut self, mask: u32);
}
