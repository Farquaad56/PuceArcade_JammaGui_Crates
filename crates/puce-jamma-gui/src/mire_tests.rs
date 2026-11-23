//! Tests des mires (`mire.rs`).

use super::*;

const SIZES: [(usize, usize); 3] = [(384, 224), (224, 384), (320, 240)];

#[test]
fn every_mire_and_variant_has_the_requested_size() {
    for m in Mire::ALL {
        assert!(!m.variants().is_empty() && !m.help().is_empty());
        for v in 0..m.variants().len() {
            for (w, h) in SIZES {
                assert_eq!(render(m, v, w, h, 3).len(), w * h, "{m:?} {v}");
            }
        }
    }
}

#[test]
fn geometry_lines_on_borders_and_center() {
    let (w, h) = (384, 224);
    let p = render(Mire::Geometry, 0, w, h, 0);
    assert_eq!(p[0], WHITE, "coin : bord de grille");
    assert_eq!(p[(h - 1) * w + w - 1], WHITE);
    assert_eq!(p[(h / 2) * w + 5], WHITE, "axe horizontal");
    assert_eq!(p[10 * w + 5], 0, "intérieur d'une case");
    assert_eq!(cells(384, 224), (16, 9));
    assert_eq!(cells(224, 384), (9, 16));
}

#[test]
fn overscan_frames_at_each_percent() {
    let (w, h) = (400, 300);
    let p = render(Mire::Overscan, 0, w, h, 0);
    assert_eq!(p[h / 4 * w], 0xFF_0000, "cadre 0 % sur le bord gauche");
    // 1 % de chaque côté = 4 px : cadre « 2 % » jaune ; 8 px : cadre « 4 % » vert
    assert_eq!(p[h / 4 * w + 4], 0xFF_FF00);
    assert_eq!(p[h / 4 * w + 8], 0x00_FF00);
}

#[test]
fn purity_and_bars_colors() {
    assert!(render(Mire::Purity, 2, 32, 32, 0)
        .iter()
        .all(|p| *p == 0x00_00FF));
    let p = render(Mire::ColorBars, 0, 70, 60, 0);
    assert_eq!(p[0], gray(191), "blanc 75 %");
    assert_eq!(p[69], 0x00_00BF, "bleu 75 %");
    assert_eq!(render(Mire::ColorBars, 1, 70, 60, 0)[0], WHITE);
}

#[test]
fn brightness_and_gamma_levels() {
    let (w, h) = (384, 224);
    let p = render(Mire::Brightness, 0, w, h, 0);
    assert_eq!(p[(h / 3) * w + w * 2 / 12 + 2], gray(5), "barre à 2 %");
    assert_eq!(p[5], 0, "fond noir");
    let g = render(Mire::Gamma, 0, w, h, 0);
    assert_eq!((g[0], g[1]), (WHITE, 0), "damier");
    assert_eq!(
        g[(h / 2) * w + w * 4 / 10 + w / 40 + 5],
        gray(186),
        "carré gamma 2.2"
    );
}

#[test]
fn motion_moves() {
    assert_ne!(
        render(Mire::Motion, 0, 64, 48, 1),
        render(Mire::Motion, 0, 64, 48, 5)
    );
    assert!(Mire::Motion.animated() && !Mire::Geometry.animated());
}
