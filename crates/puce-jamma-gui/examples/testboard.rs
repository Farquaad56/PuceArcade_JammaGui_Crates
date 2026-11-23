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
        },
    );
    if let Err(e) = res {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
