//! puce-jamma-gui — frontal egui réutilisable pour cartes JAMMA émulées (voir docs/ARCHITECTURE.md).
//!
//! Une carte implémente `puce_jamma::JammaBoard`, son système fournit un `BoardDriver` ;
//! `run(driver, options)` ouvre la fenêtre (image, filtres, overlay, son, manettes, DIP, états, débogueur).
#![forbid(unsafe_code)]

pub use puce_jamma;

use std::path::{Path, PathBuf};
use std::sync::Arc;

use puce_jamma::driver::BoardDriver;

/// Options de la fenêtre du frontal.
pub struct AppOptions {
    pub window_title: String,
    /// Dossier de `prefs.toml`.
    pub config_dir: PathBuf,
    /// Dossier des bezels au format RetroArch (`<jeu>.cfg` + PNG), affichés en plein écran.
    pub bezels_dir: Option<PathBuf>,
}

/// Jeu lancé au démarrage : le dernier jeu des préférences s'il existe encore chez le driver,
/// sinon le jeu par défaut du driver.
pub fn startup_game(driver: &dyn BoardDriver, config_dir: &Path) -> Option<String> {
    let last = prefs::Prefs::load(&prefs::prefs_path(config_dir)).last_game;
    let games = driver.games();
    last.filter(|g| {
        games
            .iter()
            .any(|e| &e.name == g && e.available && e.supported)
    })
    .or_else(|| driver.default_game())
}

/// Ouvre la fenêtre et rend la main quand elle est fermée.
pub fn run(driver: Box<dyn BoardDriver>, opts: AppOptions) -> Result<(), String> {
    let driver: Arc<dyn BoardDriver> = Arc::from(driver);
    let game = startup_game(driver.as_ref(), &opts.config_dir);
    let native = eframe::NativeOptions {
        viewport: eframe::egui::ViewportBuilder::default()
            .with_title(&opts.window_title)
            .with_inner_size(app::DEFAULT_WINDOW),
        ..Default::default()
    };
    let config_dir = opts.config_dir;
    let bezels_dir = opts.bezels_dir;
    eframe::run_native(
        &opts.window_title,
        native,
        Box::new(move |_cc| {
            Ok(Box::new(
                app::App::new(driver, &config_dir, game).with_bezels(bezels_dir),
            ))
        }),
    )
    .map_err(|e| e.to_string())
}

pub mod app;
mod app_actions;
mod app_mire;
mod app_panels;
pub mod audio;
pub mod audio_debug;
pub mod bezel;
pub mod bindings;
pub mod capture;
pub mod controls_menu;
pub mod crt;
pub mod crt_light;
pub mod debugger;
pub mod dips_menu;
pub mod display;
pub mod emu_loop;
pub mod emu_thread;
pub mod fx_menu;
mod fx_prefs;
mod fx_tabs;
pub mod gamepad;
pub mod games;
pub mod input;
pub mod menu;
pub mod mire;
mod mire_canvas;
pub mod mire_menu;
pub mod overlay;
pub mod pixel2x;
pub mod postfx;
pub mod prefs;
pub mod resample;
pub mod statusbar;
pub mod video_debug;

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma_testboard::driver::TestDriver;

    #[test]
    fn startup_game_prefers_last_known_game() {
        let dir = std::env::temp_dir().join(format!("jamma_startup_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let d = TestDriver::new(&dir);
        assert_eq!(startup_game(&d, &dir).as_deref(), Some("mire"));
        let mut p = prefs::Prefs {
            last_game: Some("mire_v".into()),
            ..Default::default()
        };
        p.save(&prefs::prefs_path(&dir)).unwrap();
        assert_eq!(startup_game(&d, &dir).as_deref(), Some("mire_v"));
        p.last_game = Some("disparu".into());
        p.save(&prefs::prefs_path(&dir)).unwrap();
        assert_eq!(startup_game(&d, &dir).as_deref(), Some("mire"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
