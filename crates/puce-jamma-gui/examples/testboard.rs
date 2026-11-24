//! Exemple : le frontal JAMMA sur la carte de test (mire horizontale et verticale).
//! `cargo run -p puce-jamma-gui --example testboard --release`

use puce_jamma_gui::{run, AppOptions};
use puce_jamma_testboard::driver::TestDriver;

fn main() {
    let res = run(
        Box::new(TestDriver::new("saves")),
        AppOptions {
            window_title: "Mire JAMMA".into(),
            config_dir: ".".into(),
            bezels_dir: None,
            about: Some(puce_jamma_gui::about::About {
                name: "Mire JAMMA".into(),
                version: env!("CARGO_PKG_VERSION").into(),
                text: "Carte de test du frontal PuceArcade JAMMA.".into(),
                license_name: "PuceArcade — usage non commercial, tous droits réservés".into(),
                license_text: include_str!("../../../LICENSE").into(),
            }),
        },
    );
    if let Err(e) = res {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
