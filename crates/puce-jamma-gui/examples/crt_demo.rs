//! Aperçu du préréglage « Moniteur d'arcade 15 kHz » avec la géométrie du tube, sur une image PNG
//! (RGB 8 bits), avec le temps de calcul :
//! `cargo run -p puce-jamma-gui --release --example crt_demo -- entree.png sortie.png [échelle]`
use std::fs::File;
use std::io::BufWriter;
use std::time::Instant;

use puce_jamma_gui::postfx::{render, Fx, Preset};

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let scale: usize = a.get(2).and_then(|v| v.parse().ok()).unwrap_or(5);
    let mut dec = png::Decoder::new(std::io::BufReader::new(File::open(&a[0]).expect("png")))
        .read_info()
        .expect("png");
    let mut buf = vec![0; dec.output_buffer_size().expect("taille")];
    let info = dec.next_frame(&mut buf).expect("image");
    let n = info.color_type.samples();
    let src: Vec<u32> = buf[..info.buffer_size()]
        .chunks(n)
        .map(|p| u32::from(p[0]) << 16 | u32::from(p[1]) << 8 | u32::from(p[2]))
        .collect();
    let (w, h) = (info.width as usize, info.height as usize);
    let fx = Fx {
        curve: 35,
        vignette: 40,
        corner: 30,
        overscan: 2,
        ..Fx::preset(Preset::Arcade)
    };
    let (ow, oh) = (w * scale, h * scale);
    let t0 = Instant::now();
    let out = render(&src, w, h, &fx, ow, oh, 10);
    println!("{ow}×{oh} : {:.1} ms", t0.elapsed().as_secs_f64() * 1000.0);
    let mut enc = png::Encoder::new(
        BufWriter::new(File::create(&a[1]).expect("sortie")),
        ow as u32,
        oh as u32,
    );
    enc.set_color(png::ColorType::Rgb);
    let mut wr = enc.write_header().expect("en-tête");
    let rgb: Vec<u8> = out.iter().flat_map(|c| [c.r(), c.g(), c.b()]).collect();
    wr.write_image_data(&rgb).expect("écriture");
}
