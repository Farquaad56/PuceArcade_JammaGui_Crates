//! Aperçu des filtres vidéo sur une image PNG (RGB 8 bits), agrandie ×3 :
//! `cargo run -p puce-jamma-gui --release --example fx_demo -- entree.png sortie.png [trame]`
//! Réglages : préréglage Sony Trinitron + canaux, convergence et barre de balayage.
use std::fs::File;
use std::io::BufWriter;

use puce_jamma_gui::postfx::{render, Fx, Preset};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let frame: u64 = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(70);
    let mut dec = png::Decoder::new(std::io::BufReader::new(File::open(&a[0]).expect("png")))
        .read_info()
        .unwrap();
    let mut buf = vec![0; dec.output_buffer_size().unwrap()];
    let info = dec.next_frame(&mut buf).unwrap();
    let (w, h) = (info.width as usize, info.height as usize);
    let n = info.color_type.samples();
    let src: Vec<u32> = buf[..info.buffer_size()]
        .chunks(n)
        .map(|p| u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
        .collect();
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
    let out = render(&src, w, h, &fx, w * 3, h * 3, frame);
    let mut enc = png::Encoder::new(
        BufWriter::new(File::create(&a[1]).unwrap()),
        (w * 3) as u32,
        (h * 3) as u32,
    );
    enc.set_color(png::ColorType::Rgb);
    enc.set_depth(png::BitDepth::Eight);
    let data: Vec<u8> = out.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
    enc.write_header().unwrap().write_image_data(&data).unwrap();
}
