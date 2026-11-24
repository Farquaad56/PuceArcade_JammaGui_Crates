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

/// Une voie sonore d'une puce (canal FM, voix PCM ou ADPCM…), pour l'oscilloscope du débogueur.
///
/// La carte choisit le découpage : une voie par canal de la puce, regroupées par `chip`. Les
/// échantillons sont pris à la sortie de la voie, avant le mixage et le panoramique.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct AudioVoice {
    /// Nom court affiché sur l'oscilloscope (« FM 1 », « PCM 3 »…).
    pub name: String,
    /// Puce ou groupe de la voie (« YM2151 », « QSound »…) ; les voies d'un même groupe se suivent.
    pub chip: String,
    /// Fréquence d'échantillonnage de `samples`, en Hz.
    pub sample_rate: u32,
    /// Derniers échantillons de la voie, du plus ancien au plus récent, dans [-1, 1].
    pub samples: Vec<f32>,
    /// La voie joue (note tenue, échantillon en cours) ; sert à atténuer les voies au repos.
    pub active: bool,
    /// Détail libre affiché sous le nom (« Do 4 · algo 5 », « vol 3FFF · pan 10 »…).
    pub detail: String,
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

    /// Voies sonores pour l'oscilloscope (vide par défaut : onglet Son sans voie).
    fn voices(&self) -> Vec<AudioVoice> {
        Vec::new()
    }

    /// Active ou coupe l'enregistrement des voies (coûteux : seulement onglet Son ouvert).
    fn set_voice_scope(&mut self, _on: bool) {}

    /// Voies coupées dans le mixage (bit n = voie n de `voices()` muette ; 0 = son normal).
    /// L'oscilloscope continue de montrer une voie coupée.
    fn set_voice_mute(&mut self, _mask: u64) {}
}
