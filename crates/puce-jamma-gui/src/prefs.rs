//! Préférences du frontal, sauvegardées dans `<config_dir>/prefs.toml`.
//! Reprises de ref/puce16-neogeo/ng-gui/prefs.rs ; retiré : ce qui est propre au Neo Geo (DIP MVS).
//! Ajouté : rotation forcée de l'image et fréquence d'échantillonnage souhaitée.
//! Un fichier absent ou invalide donne les valeurs par défaut (clé inconnue ou invalide ignorée).

use crate::overlay::{Corner, OverlayPrefs};
use crate::pixel2x::Scaler;
use crate::postfx::{Fx, Mask, ScanMode};
use puce_jamma::video::Rotation;
use std::path::{Path, PathBuf};

/// Mode d'échelle de l'image.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScaleMode {
    /// Plus grand facteur entier tenant dans la fenêtre.
    #[default]
    Auto,
    Fixed(u32),
}

/// Forme des pixels à l'affichage.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Aspect {
    /// Pixels carrés (pixel perfect).
    #[default]
    Square,
    /// Image étirée selon le rapport d'aspect du moniteur (non pixel perfect).
    Tv,
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct DisplayPrefs {
    pub scale: ScaleMode,
    pub fullscreen: bool,
    pub aspect: Aspect,
    /// Lissage bilinéaire (désactivé par défaut : pixels nets).
    pub smooth: bool,
    /// Rotation forcée de l'image (`None` = celle annoncée par la carte).
    pub rotation_override: Option<Rotation>,
}

/// Assignations action -> nom de touche / de bouton ; vide = valeurs par défaut.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct InputPrefs {
    pub keys: Vec<(String, String)>,
    pub pad: Vec<(String, String)>,
    /// Sensibilité de la souris pour trackball / molette, en % (`None` = 100).
    pub mouse_sens: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DebugPrefs {
    pub open: bool,
    /// Onglet actif du débogueur.
    pub tab: String,
    /// Région affichée par l'onglet Mémoire.
    pub region: String,
    pub show_status_bar: bool,
}

impl Default for DebugPrefs {
    fn default() -> Self {
        Self {
            open: false,
            tab: "cpu".into(),
            region: "work".into(),
            show_status_bar: true,
        }
    }
}

/// Volume sonore.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AudioPrefs {
    /// 0 à 100 %.
    pub volume: u8,
    pub mute: bool,
    /// Fréquence de sortie souhaitée en Hz (`None` = celle du périphérique).
    pub sample_rate_hint: Option<u32>,
}

impl Default for AudioPrefs {
    fn default() -> Self {
        AudioPrefs {
            volume: 100,
            mute: false,
            sample_rate_hint: None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Default)]
pub struct Prefs {
    pub audio: AudioPrefs,
    pub overlay: OverlayPrefs,
    pub display: DisplayPrefs,
    pub input: InputPrefs,
    pub debug: DebugPrefs,
    pub fx: Fx,
    /// Dernier jeu chargé (relancé au démarrage si le driver le connaît encore).
    pub last_game: Option<String>,
}

/// Chemin des préférences : `<config_dir>/prefs.toml`.
pub fn prefs_path(config_dir: &Path) -> PathBuf {
    config_dir.join("prefs.toml")
}

fn rotation_str(r: Option<Rotation>) -> &'static str {
    match r {
        None => "auto",
        Some(Rotation::Rot0) => "0",
        Some(Rotation::Rot90) => "90",
        Some(Rotation::Rot180) => "180",
        Some(Rotation::Rot270) => "270",
    }
}

/// `"0" | "90" | "180" | "270"` -> rotation forcée ; toute autre valeur (dont `"auto"`) -> `None`.
fn parse_rotation(v: &str) -> Option<Rotation> {
    match v {
        "0" => Some(Rotation::Rot0),
        "90" => Some(Rotation::Rot90),
        "180" => Some(Rotation::Rot180),
        "270" => Some(Rotation::Rot270),
        _ => None,
    }
}

/// Écrit `text` dans `path` sans jamais laisser de fichier tronqué (`.tmp` puis renommage).
pub fn write_atomic(path: &Path, text: &str) -> Result<(), String> {
    let mut tmp = path.as_os_str().to_owned();
    tmp.push(".tmp");
    let tmp = PathBuf::from(tmp);
    std::fs::write(&tmp, text).map_err(|e| format!("{} : {e}", tmp.display()))?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        format!("{} : {e}", path.display())
    })
}

fn scale_to_str(s: ScaleMode) -> String {
    match s {
        ScaleMode::Auto => "auto".into(),
        ScaleMode::Fixed(n) => format!("fixed:{n}"),
    }
}

fn parse_scale(v: &str) -> Option<ScaleMode> {
    if v == "auto" {
        return Some(ScaleMode::Auto);
    }
    let n: u32 = v.strip_prefix("fixed:")?.parse().ok()?;
    (1..=16).contains(&n).then_some(ScaleMode::Fixed(n))
}

fn parse_bool(v: &str) -> Option<bool> {
    match v {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    }
}

/// Un nom (action, touche, onglet) sûr à écrire tel quel entre guillemets.
fn is_plain(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || "_-:. ".contains(c))
}

fn scan_mode_str(m: ScanMode) -> &'static str {
    match m {
        ScanMode::Off => "off",
        ScanMode::Standard => "standard",
        ScanMode::Adaptive => "adaptive",
    }
}

fn mask_str(m: Mask) -> &'static str {
    match m {
        Mask::None => "none",
        Mask::Trinitron => "trinitron",
        Mask::FadedYellow => "faded",
    }
}

impl Prefs {
    pub fn to_toml(&self) -> String {
        let d = &self.display;
        let mut s = format!(
            "[display]\nscale = \"{}\"\nfullscreen = {}\naspect = \"{}\"\nsmooth = {}\nrotation = \"{}\"\n",
            scale_to_str(d.scale),
            d.fullscreen,
            match d.aspect {
                Aspect::Square => "square",
                Aspect::Tv => "tv",
            },
            d.smooth,
            rotation_str(d.rotation_override)
        );
        if let Some(n) = self.input.mouse_sens {
            s.push_str(&format!("\n[input]\nmouse_sensitivity = {n}\n"));
        }
        for (name, list) in [("keys", &self.input.keys), ("pad", &self.input.pad)] {
            s.push_str(&format!("\n[input.{name}]\n"));
            for (action, value) in list {
                if is_plain(action) && is_plain(value) && !action.contains(' ') {
                    s.push_str(&format!("{action} = \"{value}\"\n"));
                }
            }
        }
        s.push_str(&format!(
            "\n[audio]\nvolume = {}\nmute = {}\n",
            self.audio.volume, self.audio.mute
        ));
        if let Some(hz) = self.audio.sample_rate_hint {
            s.push_str(&format!("sample_rate = {hz}\n"));
        }
        let o = &self.overlay;
        s.push_str(&format!(
            "\n[overlay]\nenabled = {}\ndetail = \"{}\"\ncorner = \"{}\"\n",
            o.enabled,
            if o.full { "full" } else { "compact" },
            o.corner.name()
        ));
        let f = &self.fx;
        s.push_str(&format!(
            "\n[fx]\nscaler = \"{}\"\ninterp = {}\nscanlines = \"{}\"\nscan = {}\nscan_dark = {}\nscan_bright = {}\ngamma = {}\nmask = \"{}\"\nmask_pct = {}\n",
            f.scaler.key(),
            f.interp,
            scan_mode_str(f.scan_mode),
            f.scan,
            f.scan_dk,
            f.scan_br,
            f.gamma,
            mask_str(f.mask),
            f.mask_pct
        ));
        s.push_str(&format!(
            "red = {}\ngreen = {}\nblue = {}\nconv_rx = {}\nconv_ry = {}\nconv_bx = {}\nconv_by = {}\nsweep = {}\nsweep_speed = {}\nsweep_width = {}\nbeam = {}\nphosphor = {}\nbeam_floor = {}\nghost = {}\n",
            f.red, f.green, f.blue, f.conv_rx, f.conv_ry, f.conv_bx, f.conv_by, f.sweep, f.sweep_speed, f.sweep_width,
            f.beam, f.phosphor, f.beam_floor, f.ghost
        ));
        let g = &self.debug;
        s.push_str(&format!(
            "\n[debug]\nopen = {}\ntab = \"{}\"\nregion = \"{}\"\nshow_status_bar = {}\n",
            g.open, g.tab, g.region, g.show_status_bar
        ));
        if let Some(game) = self.last_game.as_deref().filter(|g| is_plain(g)) {
            s.push_str(&format!("\n[session]\nlast_game = \"{game}\"\n"));
        }
        s
    }

    /// Lit un `gui.toml` ; toute ligne inconnue ou invalide est ignorée (valeur par défaut).
    pub fn from_toml(text: &str) -> Prefs {
        let mut p = Prefs::default();
        let mut section = String::new();
        for line in text.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some(name) = line.strip_prefix('[').and_then(|l| l.strip_suffix(']')) {
                section = name.trim().to_string();
                continue;
            }
            let Some((k, v)) = line.split_once('=') else {
                continue;
            };
            let (k, v) = (k.trim(), v.trim().trim_matches('"'));
            match (section.as_str(), k) {
                ("display", "scale") => {
                    if let Some(s) = parse_scale(v) {
                        p.display.scale = s;
                    }
                }
                ("display", "fullscreen") => {
                    if let Some(b) = parse_bool(v) {
                        p.display.fullscreen = b;
                    }
                }
                ("display", "aspect") => match v {
                    "square" => p.display.aspect = Aspect::Square,
                    "tv" | "4:3" => p.display.aspect = Aspect::Tv,
                    _ => {}
                },
                ("display", "smooth") => {
                    if let Some(b) = parse_bool(v) {
                        p.display.smooth = b;
                    }
                }
                ("display", "rotation") => p.display.rotation_override = parse_rotation(v),
                ("input", "mouse_sensitivity") => {
                    if let Ok(n) = v.parse::<u32>() {
                        p.input.mouse_sens = Some(n.clamp(10, 500));
                    }
                }
                ("input.keys", _) if is_plain(k) && is_plain(v) => {
                    p.input.keys.push((k.to_string(), v.to_string()));
                }
                ("input.pad", _) if is_plain(k) && is_plain(v) => {
                    p.input.pad.push((k.to_string(), v.to_string()));
                }
                ("audio", "volume") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.audio.volume = n.min(100);
                    }
                }
                ("audio", "sample_rate") => {
                    if let Ok(n) = v.parse::<u32>() {
                        p.audio.sample_rate_hint = (8_000..=192_000).contains(&n).then_some(n);
                    }
                }
                ("audio", "mute") => {
                    if let Some(b) = parse_bool(v) {
                        p.audio.mute = b;
                    }
                }
                ("overlay", "enabled") => {
                    if let Some(b) = parse_bool(v) {
                        p.overlay.enabled = b;
                    }
                }
                ("overlay", "detail") => match v {
                    "full" => p.overlay.full = true,
                    "compact" => p.overlay.full = false,
                    _ => {}
                },
                ("overlay", "corner") => {
                    if let Some(c) = Corner::from_name(v) {
                        p.overlay.corner = c;
                    }
                }
                ("fx", "scaler") => {
                    if let Some(k) = Scaler::from_key(v) {
                        p.fx.scaler = k;
                    }
                }
                ("fx", "interp") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.interp = n.min(100);
                    }
                }
                ("fx", "scanlines") => match v {
                    "off" => p.fx.scan_mode = ScanMode::Off,
                    "standard" => p.fx.scan_mode = ScanMode::Standard,
                    "adaptive" => p.fx.scan_mode = ScanMode::Adaptive,
                    _ => {}
                },
                ("fx", "scan") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.scan = n.min(95);
                    }
                }
                ("fx", "scan_dark") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.scan_dk = n.min(95);
                    }
                }
                ("fx", "scan_bright") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.scan_br = n.min(95);
                    }
                }
                ("fx", "gamma") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.gamma = n.clamp(100, 200);
                    }
                }
                ("fx", "mask") => match v {
                    "none" => p.fx.mask = Mask::None,
                    "trinitron" => p.fx.mask = Mask::Trinitron,
                    "faded" => p.fx.mask = Mask::FadedYellow,
                    _ => {}
                },
                ("fx", "mask_pct") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.mask_pct = n.min(100);
                    }
                }
                ("fx", "red" | "green" | "blue") => {
                    if let Ok(n) = v.parse::<u8>() {
                        let n = n.min(200);
                        match k {
                            "red" => p.fx.red = n,
                            "green" => p.fx.green = n,
                            _ => p.fx.blue = n,
                        }
                    }
                }
                ("fx", "conv_rx" | "conv_ry" | "conv_bx" | "conv_by") => {
                    if let Ok(n) = v.parse::<i8>() {
                        let n = n.clamp(-8, 8);
                        match k {
                            "conv_rx" => p.fx.conv_rx = n,
                            "conv_ry" => p.fx.conv_ry = n,
                            "conv_bx" => p.fx.conv_bx = n,
                            _ => p.fx.conv_by = n,
                        }
                    }
                }
                ("fx", "sweep") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.sweep = n.min(100);
                    }
                }
                ("fx", "sweep_speed") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.sweep_speed = n.clamp(5, 100);
                    }
                }
                ("fx", "beam") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.beam = n.min(100);
                    }
                }
                ("fx", "phosphor") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.phosphor = n.min(100);
                    }
                }
                ("fx", "beam_floor") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.beam_floor = n.min(100);
                    }
                }
                ("fx", "ghost") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.ghost = n.min(90);
                    }
                }
                ("fx", "sweep_width") => {
                    if let Ok(n) = v.parse::<u8>() {
                        p.fx.sweep_width = n.clamp(2, 50);
                    }
                }
                ("debug", "open") => {
                    if let Some(b) = parse_bool(v) {
                        p.debug.open = b;
                    }
                }
                ("debug", "tab") if is_plain(v) => p.debug.tab = v.to_string(),
                ("debug", "region") if is_plain(v) => p.debug.region = v.to_string(),
                ("session", "last_game") if is_plain(v) => p.last_game = Some(v.to_string()),
                ("debug", "show_status_bar") => {
                    if let Some(b) = parse_bool(v) {
                        p.debug.show_status_bar = b;
                    }
                }
                _ => {}
            }
        }
        p
    }

    /// Charge les préférences ; fichier absent ou illisible = valeurs par défaut.
    pub fn load(path: &Path) -> Prefs {
        std::fs::read_to_string(path)
            .map(|t| Prefs::from_toml(&t))
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        write_atomic(path, &self.to_toml())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn custom() -> Prefs {
        Prefs {
            display: DisplayPrefs {
                scale: ScaleMode::Fixed(3),
                fullscreen: true,
                aspect: Aspect::Tv,
                smooth: true,
                rotation_override: Some(Rotation::Rot270),
            },
            input: InputPrefs {
                keys: vec![
                    ("P1A".into(), "Q".into()),
                    ("Start1".into(), "Space".into()),
                ],
                pad: vec![("P1B".into(), "North".into())],
                mouse_sens: Some(150),
            },
            debug: DebugPrefs {
                open: true,
                tab: "sprites".into(),
                region: "vram".into(),
                show_status_bar: false,
            },
            fx: Fx {
                scaler: Scaler::SuperEagle,
                red: 110,
                blue: 90,
                conv_rx: 2,
                conv_by: -1,
                sweep: 30,
                sweep_speed: 25,
                sweep_width: 20,
                beam: 30,
                phosphor: 45,
                beam_floor: 15,
                ghost: 40,
                ..Fx::preset(crate::postfx::Preset::SonyTrinitron)
            },
            audio: AudioPrefs {
                volume: 35,
                mute: true,
                sample_rate_hint: Some(48_000),
            },
            overlay: OverlayPrefs {
                enabled: true,
                full: false,
                corner: Corner::BottomRight,
            },
            last_game: Some("sf2ce".into()),
        }
    }

    #[test]
    fn round_trip() {
        let p = custom();
        assert_eq!(Prefs::from_toml(&p.to_toml()), p);
        let d = Prefs::default();
        assert_eq!(Prefs::from_toml(&d.to_toml()), d);
    }

    #[test]
    fn missing_or_garbage_gives_defaults() {
        assert_eq!(
            Prefs::load(Path::new("/inexistant/prefs.toml")),
            Prefs::default()
        );
        assert_eq!(
            Prefs::from_toml("n'importe quoi\n[[[\n= = ="),
            Prefs::default()
        );
    }

    #[test]
    fn unknown_keys_are_ignored_and_scale_parses() {
        let p = Prefs::from_toml(
            "[display]\nfoo = 1\nscale = \"fixed:3\"\nfullscreen = peut-être\n[autre]\nscale = \"fixed:9\"\n",
        );
        assert_eq!(p.display.scale, ScaleMode::Fixed(3));
        assert!(!p.display.fullscreen);
        assert_eq!(
            Prefs::from_toml("[display]\nscale = \"auto\"")
                .display
                .scale,
            ScaleMode::Auto
        );
        assert_eq!(
            Prefs::from_toml("[display]\nscale = \"fixed:0\"")
                .display
                .scale,
            ScaleMode::Auto
        );
    }

    #[test]
    fn save_is_atomic_and_reloads() {
        let dir = std::env::temp_dir().join(format!("jamma_gui_prefs_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = prefs_path(&dir);
        assert!(path.ends_with("prefs.toml"));
        let p = custom();
        p.save(&path).expect("sauvegarde");
        assert!(!dir.join("prefs.toml.tmp").exists());
        assert_eq!(Prefs::load(&path), p);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rotation_override_parses() {
        let rot = |t: &str| Prefs::from_toml(t).display.rotation_override;
        assert_eq!(rot("[display]\nrotation = \"90\""), Some(Rotation::Rot90));
        assert_eq!(rot("[display]\nrotation = \"x\""), None);
        assert_eq!(rot("[display]\nrotation = \"auto\""), None);
        assert_eq!(rot(""), None);
    }

    #[test]
    fn sample_rate_hint_parses() {
        let hz = |t: &str| Prefs::from_toml(t).audio.sample_rate_hint;
        assert_eq!(hz("[audio]\nsample_rate = 44100"), Some(44_100));
        assert_eq!(hz("[audio]\nsample_rate = 12"), None);
        assert_eq!(hz("[audio]\nsample_rate = rapide"), None);
    }

    #[test]
    fn neo_geo_tv_aspect_key_still_reads() {
        assert_eq!(
            Prefs::from_toml("[display]\naspect = \"4:3\"")
                .display
                .aspect,
            Aspect::Tv
        );
    }
}
