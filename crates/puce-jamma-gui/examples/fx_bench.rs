//! Coût des filtres par trame pour quelques réglages, en 1080p et 4K :
//! `cargo run -p puce-jamma-gui --release --example fx_bench`
use puce_jamma_gui::postfx::{render, Fx, Mask, Preset, ScanMode};
use std::time::Instant;
fn main() {
    let src: Vec<u32> = (0..384 * 224)
        .map(|i| (i as u32).wrapping_mul(2654435761) & 0xFFFFFF)
        .collect();
    let base = Fx::default();
    let cases: Vec<(&str, Fx)> = vec![
        ("trinitron (avant)", Fx::preset(Preset::SonyTrinitron)),
        (
            "adaptive seul",
            Fx {
                scan_mode: ScanMode::Adaptive,
                ..base
            },
        ),
        (
            "gaussien seul",
            Fx {
                scan_mode: ScanMode::Gaussian,
                ..base
            },
        ),
        (
            "slot seul",
            Fx {
                mask: Mask::Slot,
                ..base
            },
        ),
        (
            "temp+noir",
            Fx {
                temp: 93,
                black: 2,
                ..base
            },
        ),
        ("halo", Fx { bloom: 25, ..base }),
        ("courbure", Fx { curve: 35, ..base }),
        (
            "vignette+coins",
            Fx {
                vignette: 40,
                corner: 30,
                ..base
            },
        ),
        ("arcade", Fx::preset(Preset::Arcade)),
        (
            "arcade+géom",
            Fx {
                curve: 35,
                vignette: 40,
                corner: 30,
                ..Fx::preset(Preset::Arcade)
            },
        ),
    ];
    for (w, h) in [(1920usize, 1080usize), (2880, 2160)] {
        for (n, fx) in &cases {
            render(&src, 384, 224, fx, w, h, 0);
            let t = Instant::now();
            for i in 0..5 {
                render(&src, 384, 224, fx, w, h, i);
            }
            println!(
                "{w}x{h} {n:20} {:6.1} ms",
                t.elapsed().as_secs_f64() * 200.0
            );
        }
    }
}
