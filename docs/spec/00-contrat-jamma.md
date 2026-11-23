# 00 — Contrat « connecteur JAMMA » (crate `puce-jamma`, INTERFACE GELÉE v0.1)

Idée : une borne d'arcade JAMMA accepte n'importe quelle carte parce que le connecteur fixe vidéo (RGB + synchro), son (haut-parleur),
commandes (2 joueurs, directions, boutons, start, pièces, service, test, tilt) et alimentation. Ici le « connecteur » est un trait Rust :
une carte émulée (CPS1, Neo Geo, Cave, Galaxian…) implémente `JammaBoard`, le frontal (`puce-jamma-gui`) ne connaît que ce trait.
Extensions par rapport au JAMMA d'origine : boutons 4 à 6 (« kick harness » CPS / JAMMA+), joueurs 3-4 (connecteurs supplémentaires
des cartes C du CPS1), entrées analogiques (molettes, trackballs), son stéréo, DIP switches décrits, savestates, débogage.

## Types (src/lib.rs, modules `inputs`, `video`, `audio`, `dips`, `board`, `debug`, `driver`)
```rust
// inputs.rs — logique ACTIVE HAUTE (true = contact fermé = appuyé). Chaque carte convertit vers ses registres (souvent actifs bas).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct PlayerControls { pub up: bool, pub down: bool, pub left: bool, pub right: bool, pub buttons: [bool; 6], pub start: bool }
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct JammaInputs {
    pub players: [PlayerControls; 4],   // 0 = joueur 1 (JAMMA côté composants), 1 = joueur 2 (côté soudure), 2-3 = extensions
    pub coins: [bool; 4],               // pièce 1..4
    pub service: bool, pub test: bool, pub tilt: bool,
    pub analog: [i32; 4],               // déplacements relatifs depuis la trame précédente (molette/trackball), unités de la carte
}
// video.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq)] pub enum Rotation { Rot0, Rot90, Rot180, Rot270 }   // sens horaire, comme ROTxx de MAME
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct VideoInfo { pub width: u32, pub height: u32, pub refresh_hz: f64, pub aspect: (u32, u32), pub rotation: Rotation }
//   width × height = image visible native, AVANT rotation ; aspect = rapport d'affichage du moniteur APRÈS rotation (4:3 ou 3:4 en général)
pub struct VideoOut<'a> { pub info: VideoInfo, pub pixels: &'a [u32] }   // 0x00RRGGBB, width*height, ligne par ligne
// audio.rs
#[derive(Clone, Copy, Debug, PartialEq)] pub struct AudioInfo { pub sample_rate: f64, pub channels: u8 }   // 1 = haut-parleur JAMMA mono, 2 = stéréo
// dips.rs — description lisible par le menu DIP du frontal
#[derive(Clone, Debug, PartialEq, Eq)] pub struct DipSetting { pub label: String, pub value: u8 }
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DipSwitch { pub bank: usize, pub mask: u8, pub label: String, pub default: u8, pub settings: Vec<DipSetting> }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct DipBank { pub name: String, pub default: u8 }    // ex. "SW(A)" / "DSWA"
// board.rs
#[derive(Clone, Debug, PartialEq)]
pub struct BoardInfo {
    pub system: String,      // "Capcom CPS1"
    pub name: String,        // nom court MAME : "ghouls"
    pub title: String,       // "Ghouls'n Ghosts (World)"
    pub players: u8,         // 1..=4
    pub buttons: u8,         // boutons utilisés par joueur, 0..=6 (le frontal n'affiche que ceux-là)
    pub video: VideoInfo, pub audio: AudioInfo,
    pub dip_banks: Vec<DipBank>, pub dips: Vec<DipSwitch>,
    pub has_analog: bool,
}
pub trait JammaBoard {                                   // PAS `Send` : la carte est créée et utilisée dans le thread d'émulation
    fn info(&self) -> &BoardInfo;
    fn reset(&mut self);
    fn run_frame(&mut self);                             // exactement une trame vidéo
    fn frame_count(&self) -> u64;
    fn video(&self) -> VideoOut<'_>;                     // dernière trame complète
    fn take_audio(&mut self, out: &mut Vec<f32>);        // échantillons produits depuis le dernier appel, entrelacés selon `channels`, dans [-1, 1]
    fn set_inputs(&mut self, inputs: &JammaInputs);      // appliqué avant la prochaine run_frame
    fn dips(&self) -> Vec<u8>;                           // un octet par banque
    fn set_dips(&mut self, banks: &[u8]);
    fn save_state(&mut self) -> Vec<u8>;
    fn load_state(&mut self, data: &[u8]) -> Result<(), String>;
    fn debug(&mut self) -> Option<&mut dyn BoardDebug> { None }
}
// debug.rs — facultatif
#[derive(Clone, Debug, PartialEq, Eq)] pub struct CpuView { pub name: String, pub pc: u32, pub registers: Vec<(String, u32)>, pub address_bits: u8 }
#[derive(Clone, Debug, PartialEq, Eq)] pub struct DebugImage { pub name: String, pub width: u32, pub height: u32, pub pixels: Vec<u32> }
pub trait BoardDebug {
    fn cpus(&self) -> Vec<CpuView>;
    fn peek(&self, cpu: usize, addr: u32) -> u8;          // SANS effet de bord (registres d'E/S : 0xFF)
    fn step_instruction(&mut self, cpu: usize);           // une instruction du CPU (les autres puces suivent)
    fn registers(&self) -> Vec<(String, Vec<(String, String)>)>;   // groupes nommés de registres matériels (« CPS-A », « CPS-B »…)
    fn images(&self) -> Vec<DebugImage>;                  // palettes, tuiles, sprites, couches…
    fn layer_names(&self) -> Vec<String>;
    fn set_layer_mask(&mut self, mask: u32);              // bit n = couche n visible ; défaut : toutes
}
// driver.rs — catalogue et chargement (côté « système »)
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GameEntry { pub name: String, pub title: String, pub parent: String, pub available: bool, pub supported: bool,
                       pub kind: GameKind }                  // v0.5, ADR 002
pub enum GameKind { Original, Clone, Bootleg, Hack, Prototype } // GameKind::from_mame(parent, constructeur, titre)
pub trait BoardDriver: Send + Sync {
    fn system_name(&self) -> &str;
    fn games(&self) -> Vec<GameEntry>;                    // triés dans l'ordre d'affichage voulu par le système
    fn default_game(&self) -> Option<String>;
    fn load(&self, name: &str) -> Result<Box<dyn JammaBoard>, String>;   // appelé DANS le thread d'émulation
    fn saves_dir(&self) -> std::path::PathBuf;
}
```

## Utilitaires fournis par `puce-jamma` (module `util`)
- `pub fn rotate(src: &[u32], w: u32, h: u32, rot: Rotation) -> (Vec<u32>, u32, u32)` : image tournée (sens horaire) et ses dimensions.
  Rot90 : pixel source (x, y) -> destination (h-1-y, x), taille h × w. Rot180 : (w-1-x, h-1-y). Rot270 : (y, w-1-x), taille h × w.
- `pub fn default_dips(info: &BoardInfo) -> Vec<u8>` : octet par banque = `DipBank::default`.
- `pub fn dip_value(banks: &[u8], sw: &DipSwitch) -> u8` (= `banks[sw.bank] & sw.mask`) et `pub fn set_dip(banks: &mut [u8], sw: &DipSwitch, value: u8)`.
- `pub fn active_low(b: bool, bit: u8) -> u8` : 0 si appuyé, `bit` sinon (aide aux cartes à entrées actives basses).

## Règles du contrat
- Aucune dépendance (ni egui, ni threads, ni horloge système) dans `puce-jamma`.
- Le frontal appelle, par trame : `set_inputs` -> `run_frame` -> `video` -> `take_audio`. Il cadence à `info().video.refresh_hz`.
- Toute évolution du contrat = ADR + nouvelle version mineure (le CPS1 épingle une révision git).
