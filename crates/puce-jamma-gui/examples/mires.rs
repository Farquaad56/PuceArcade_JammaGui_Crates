//! Enregistre toutes les mires de réglage en PNG (une image par mire et variante) :
//! `cargo run -p puce-jamma-gui --release --example mires -- dossier [largeur hauteur]`
use std::fs::File;
use std::io::BufWriter;

use puce_jamma_gui::mire::{render, Mire};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let dir = std::path::PathBuf::from(a.first().map_or(".", String::as_str));
    let w: usize = a.get(1).and_then(|v| v.parse().ok()).unwrap_or(384);
    let h: usize = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(224);
    std::fs::create_dir_all(&dir).expect("dossier");
    for (i, m) in Mire::ALL.into_iter().enumerate() {
        for v in 0..m.variants().len() {
            let px = render(m, v, w, h, 30);
            let path = dir.join(format!("{:02}_{v}.png", i + 1));
            let mut enc = png::Encoder::new(
                BufWriter::new(File::create(&path).expect("png")),
                w as u32,
                h as u32,
            );
            enc.set_color(png::ColorType::Rgb);
            let rgb: Vec<u8> = px
                .iter()
                .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
                .collect();
            enc.write_header()
                .expect("en-tête")
                .write_image_data(&rgb)
                .expect("écriture");
            println!("{} : {} ({})", path.display(), m.label(), m.variants()[v]);
        }
    }
}
