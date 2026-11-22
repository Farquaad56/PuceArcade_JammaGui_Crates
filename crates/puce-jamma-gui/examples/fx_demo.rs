//! Aperçu des filtres vidéo sur une image PNG (RGB 8 bits), agrandie ×3 :
//! `cargo run -p puce-jamma-gui --release --example fx_demo -- entree.png sortie.png [trame]`
//! Réglages : préréglage Sony Trinitron + canaux, convergence et barre de balayage.
use std::fs::File;
use std::io::BufWriter;

use puce_jamma_gui::postfx::{render, Fx, Preset};

fn load(path: &str) -> (Vec<u32>, usize, usize) {
    let mut dec = png::Decoder::new(std::io::BufReader::new(File::open(path).expect("png")))
        .read_info()
        .unwrap();
    let mut buf = vec![0; dec.output_buffer_size().unwrap()];
    let info = dec.next_frame(&mut buf).unwrap();
    let n = info.color_type.samples();
    let src = buf[..info.buffer_size()]
        .chunks(n)
        .map(|p| u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
        .collect();
    (src, info.width as usize, info.height as usize)
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let frame: u64 = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(70);
    let (src, w, h) = load(&a[0]);
    let fx = Fx {
        red: 110,
        blue: 90,
        conv_rx: 2,
        conv_bx: -2,
        conv_by: 1,
        sweep: 80,
        sweep_width: 25,
        ..Fx::preset(Preset::SonyTrinitron)
    };
    let (ow, oh) = (w * 3, h * 3);
    let out = render(&src, w, h, &fx, ow, oh, frame);
    let mut enc = png::Encoder::new(
        BufWriter::new(File::create(&a[1]).unwrap()),
        ow as u32,
        oh as u32,
    );
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let data: Vec<u8> = out.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
    enc.write_header().unwrap().write_image_data(&data).unwrap();
}
