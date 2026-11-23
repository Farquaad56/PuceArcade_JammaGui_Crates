//! Tests des effets de tube (`crt.rs`).

use super::*;
use crate::crt_light::temp_gains;
use crate::postfx::NEUTRAL_TEMP;
use crate::postfx::{render, ScanMode};

fn flat(n: usize, v: u32) -> Vec<u32> {
    vec![v; n]
}

#[test]
fn overscan_crops_both_sides() {
    let src: Vec<u32> = (0..200 * 100).map(|i| i as u32).collect();
    let (v, w, h) = crop(&src, 200, 100, 10);
    assert_eq!((w, h), (180, 90));
    assert_eq!(v[0], (5 * 200 + 10) as u32);
}

#[test]
fn beam_widens_with_brightness() {
    let t = beam_table(40, 120);
    // à mi-chemin entre deux lignes, un canal clair éclaire plus qu'un canal sombre
    assert!(beam_weight(&t, 255, 0.5) > 2.0 * beam_weight(&t, 8, 0.5));
    assert!((beam_weight(&t, 128, 0.0) - 1.0).abs() < 1e-6);
}

#[test]
fn gaussian_scanlines_dark_gaps_between_dim_lines() {
    let fx = Fx {
        scan_mode: ScanMode::Gaussian,
        ..Fx::default()
    };
    // gris sombre à ×4 : centre de ligne plus clair que l'espace entre deux lignes
    let o = render(&flat(16 * 8, 0x40_4040), 16, 8, &fx, 64, 32, 0);
    let (center, gap) = (o[(4 * 2 + 1) * 64].r(), o[(4 * 2) * 64].r());
    assert!(center > gap + 10, "centre {center}, espace {gap}");
}

#[test]
fn procedural_masks_have_expected_shape() {
    let (w, h, c) = procedural_mask(Mask::Slot, 1);
    assert_eq!((w, h, c.len()), (6, 4, 24));
    assert_eq!(c[0], [ON, OFF, OFF]);
    assert_eq!(
        c[3 * 6],
        [OFF; 3],
        "fente coupée en rangée 3 pour la 1re triade"
    );
    let (w, h, c) = procedural_mask(Mask::Shadow, 2);
    assert_eq!((w, h), (12, 4));
    assert_eq!(
        c[2 * w],
        [OFF, ON, OFF],
        "rangée impaire décalée d'une demi-triade (1,5 point : vert)"
    );
    assert_eq!(pitch(0, 2160), 3);
    assert_eq!(pitch(0, 600), 1);
}

#[test]
fn temperature_9300_is_bluish_and_neutral_is_identity() {
    assert_eq!(temp_gains(NEUTRAL_TEMP), [1.0; 3]);
    let g = temp_gains(93);
    assert!(
        (g[2] - 1.0).abs() < 1e-6 && g[0] < 0.9 && g[0] < g[1],
        "{g:?}"
    );
    let warm = temp_gains(50);
    assert!(warm[0] > warm[2]);
}

#[test]
fn black_level_lifts_black() {
    let fx = Fx {
        black: 10,
        ..Fx::default()
    };
    let o = render(&flat(16, 0), 4, 4, &fx, 4, 4, 0);
    assert_eq!(o[0].r(), 25);
    let o = render(&flat(16, 0xFF_FFFF), 4, 4, &fx, 4, 4, 0);
    assert_eq!(o[0].r(), 255);
}

#[test]
fn curvature_blackens_corners_keeps_center() {
    let fx = Fx {
        curve: 100,
        ..Fx::default()
    };
    let o = render(&flat(64 * 64, 0x80_8080), 64, 64, &fx, 64, 64, 0);
    assert_eq!(o[0], Color32::BLACK);
    assert_eq!(o[32 * 64 + 32], Color32::from_rgb(128, 128, 128));
}

#[test]
fn vignette_and_corner_darken_edges() {
    let fx = Fx {
        vignette: 100,
        corner: 50,
        ..Fx::default()
    };
    let o = render(&flat(100 * 100, 0xFF_FFFF), 100, 100, &fx, 100, 100, 0);
    assert_eq!(o[0], Color32::BLACK, "coin arrondi");
    assert!(o[50 * 100 + 2].r() < o[50 * 100 + 50].r());
    assert_eq!(o[50 * 100 + 50].r(), 255);
}

#[test]
fn bloom_spreads_light_into_dark() {
    let mut src = flat(64 * 64, 0);
    for y in 28..36 {
        for x in 28..36 {
            src[y * 64 + x] = 0xFF_FFFF;
        }
    }
    let fx = Fx {
        bloom: 80,
        bloom_size: 4,
        ..Fx::default()
    };
    let o = render(&src, 64, 64, &fx, 64, 64, 0);
    assert!(o[32 * 64 + 40].r() > 20, "halo à droite du carré");
    assert_eq!(o[0].r(), 0, "loin du carré : noir");
}

#[test]
fn jitter_changes_with_frame_and_geometry_can_be_removed() {
    let src: Vec<u32> = (0..64 * 32)
        .map(|i| if i % 3 == 0 { 0xFF_FFFF } else { 0 })
        .collect();
    let fx = Fx {
        jitter: 100,
        ..Fx::default()
    };
    assert_ne!(
        render(&src, 64, 32, &fx, 256, 128, 1),
        render(&src, 64, 32, &fx, 256, 128, 2)
    );
    let g = Fx {
        curve: 40,
        vignette: 30,
        corner: 20,
        black: 3,
        ..Fx::default()
    };
    let n = g.without_geometry();
    assert!(g.geometry_active() && !n.geometry_active() && n.black == 3);
}

#[test]
fn purity_tints_corners_not_center() {
    let fx = Fx {
        purity: 100,
        ..Fx::default()
    };
    let o = render(&flat(100 * 100, 0x80_8080), 100, 100, &fx, 100, 100, 0);
    let tl = o[0];
    assert!(
        tl.r() > tl.g() && tl.b() > tl.g(),
        "magenta en haut à gauche : {tl:?}"
    );
    let c = o[50 * 100 + 50];
    assert!((i32::from(c.r()) - i32::from(c.g())).abs() <= 4);
}

#[test]
fn bloom_leaves_black_cells_black_on_a_grid() {
    // grille fine blanche sur noir : aucun bloc parasite au milieu des cases
    let (w, h) = (384, 224);
    let src = crate::mire::render(crate::mire::Mire::Geometry, 0, w, h, 0);
    let fx = Fx {
        bloom: 25,
        ..Fx::default()
    };
    let o = render(&src, w, h, &fx, w * 4, h * 4, 0);
    let max = (40 * 4..45 * 4)
        .flat_map(|y| (108 * 4..116 * 4).map(move |x| (x, y)))
        .map(|(x, y)| o[y * w * 4 + x].r())
        .max()
        .unwrap_or(0);
    assert!(max < 80, "pixel parasite : {max}");
}

#[test]
fn sweep_and_jitter_animate_at_large_frame_numbers() {
    // mires et longues parties : les numéros de trame grands doivent encore animer les effets
    let fx = Fx {
        sweep: 80,
        ..Fx::default()
    };
    let big = 1u64 << 40;
    assert_ne!(
        crate::postfx::sweep_gains(&fx, 100, big),
        crate::postfx::sweep_gains(&fx, 100, big + 30)
    );
    let src: Vec<u32> = (0..64 * 32)
        .map(|i| if i % 3 == 0 { 0xFF_FFFF } else { 0 })
        .collect();
    let j = Fx {
        jitter: 100,
        ..Fx::default()
    };
    assert_ne!(
        render(&src, 64, 32, &j, 256, 128, big),
        render(&src, 64, 32, &j, 256, 128, big + 7)
    );
}
