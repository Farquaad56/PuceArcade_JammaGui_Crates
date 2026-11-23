//! Vérifie un bezel RetroArch : `cargo run -p puce-jamma-gui --example bezel_info -- bezels/sf2.cfg`
//! Affiche la taille de l'image, la zone transparente détectée et ses proportions.
fn main() {
    for a in std::env::args().skip(1) {
        match puce_jamma_gui::bezel::load(std::path::Path::new(&a)) {
            Ok(b) => {
                let (x0, y0, x1, y1) = b.hole;
                let (w, h) = (x1 - x0, y1 - y0);
                println!(
                    "{a} : {}×{}, écran {x0},{y0} -> {x1},{y1} ({w}×{h}, rapport {:.3})",
                    b.w,
                    b.h,
                    w as f64 / h as f64
                );
            }
            Err(e) => println!("{a} : {e}"),
        }
    }
}
