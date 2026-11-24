//! Effets de tube cathodique : overscan, faisceau gaussien, masques à fentes et à points,
//! courbure et instabilité. Couleur, halo, masque posé et vignettage : `crt_light.rs`.
//!
//! Formules de géométrie et de masques inspirées des shaders CRT publics (Timothy Lottes).

use eframe::egui::Color32;

use crate::postfx::{Fx, Mask};

/// Nombre maximal de fils pour les filtres (au-delà, le gain est faible et la mémoire limite).
pub const MAX_THREADS: usize = 16;

/// Applique `f(y, rangée)` à toutes les rangées de `out`, sur plusieurs fils pour les grandes images.
pub fn par_rows(out: &mut [Color32], w: usize, f: impl Fn(usize, &mut [Color32]) + Sync) {
    let h = out.len() / w.max(1);
    let threads = if w * h < 200_000 {
        1
    } else {
        std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(MAX_THREADS)
    };
    let per = h.div_ceil(threads).max(1);
    std::thread::scope(|s| {
        for (i, chunk) in out.chunks_mut(per * w).enumerate() {
            let f = &f;
            s.spawn(move || {
                for (dy, row) in chunk.chunks_mut(w).enumerate() {
                    f(i * per + dy, row);
                }
            });
        }
    });
}

/// Overscan : retire `pct` % de l'image (moitié de chaque côté, sur les deux axes).
pub fn crop(src: &[u32], w: usize, h: usize, pct: u8) -> (Vec<u32>, usize, usize) {
    let p = usize::from(pct.min(10));
    let (cx, cy) = (w * p / 200, h * p / 200);
    let (nw, nh) = (w - 2 * cx, h - 2 * cy);
    let mut v = Vec::with_capacity(nw * nh);
    for y in cy..cy + nh {
        v.extend_from_slice(&src[y * w + cx..y * w + cx + nw]);
    }
    (v, nw, nh)
}

/// Faisceau gaussien : niveaux de valeur (par pas de 8) et pas de distance (1/32 de ligne).
pub const BEAM_LEVELS: usize = 32;
pub const BEAM_STEPS: usize = 32;
/// Distance maximale prise en compte : 2 lignes.
const BEAM_DIST: usize = 2 * BEAM_STEPS + 1;

/// Table des poids du faisceau : `t[niveau * BEAM_DIST + distance]`.
/// La largeur à mi-hauteur va de `dk` % (canal noir) à `br` % (canal au maximum) d'une ligne.
pub fn beam_table(dk: u8, br: u8) -> Vec<f32> {
    let (dk, br) = (f32::from(dk.clamp(20, 150)), f32::from(br.clamp(20, 150)));
    let mut t = Vec::with_capacity(BEAM_LEVELS * BEAM_DIST);
    for l in 0..BEAM_LEVELS {
        let v = (l as f32 + 0.5) / BEAM_LEVELS as f32;
        let fwhm = (dk + (br.max(dk) - dk) * v) / 100.0;
        let sigma = fwhm / 2.354_8;
        for d in 0..BEAM_DIST {
            let x = d as f32 / BEAM_STEPS as f32 / sigma;
            t.push((-0.5 * x * x).exp());
        }
    }
    t
}

/// Poids du faisceau pour une valeur de canal `v` à la distance `d` (en lignes) du centre.
#[inline]
pub fn beam_weight(t: &[f32], v: u8, d: f32) -> f32 {
    let di = ((d * BEAM_STEPS as f32) as usize).min(BEAM_DIST - 1);
    t[usize::from(v >> 3) * BEAM_DIST + di]
}

/// Largeur d'une colonne de phosphore : `pitch`, ou automatique selon la hauteur de sortie.
pub fn pitch(pitch: u8, h: usize) -> usize {
    if pitch == 0 {
        (h / 720).clamp(1, 4)
    } else {
        usize::from(pitch.min(4))
    }
}

/// Gains d'une colonne allumée et éteinte des masques calculés (moyenne ≈ 0,67 compensée par la force).
const ON: f32 = 1.4;
const OFF: f32 = 0.25;

/// Motif d'un masque calculé : (largeur, hauteur, gains RVB de chaque cellule).
pub fn procedural_mask(m: Mask, p: usize) -> (usize, usize, Vec<[f32; 3]>) {
    let p = p.max(1);
    let color = |i: usize| -> [f32; 3] { std::array::from_fn(|c| if c == i { ON } else { OFF }) };
    match m {
        Mask::Slot => {
            // colonnes R, V, B de `p` px ; une triade sur deux décalée de deux rangées de fentes
            let (w, h) = (6 * p, 4 * p);
            let cells = (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let triad = x / (3 * p);
                    if (y / p + 2 * (triad % 2)) % 4 == 3 {
                        [OFF; 3]
                    } else {
                        color((x / p) % 3)
                    }
                })
                .collect();
            (w, h, cells)
        }
        Mask::Shadow => {
            // points de 2p px, une rangée sur deux décalée d'une demi-triade
            let (w, h) = (6 * p, 2 * p);
            let cells = (0..h)
                .flat_map(|y| (0..w).map(move |x| (x, y)))
                .map(|(x, y)| {
                    let xs = x + if (y / p) % 2 == 1 { 3 * p } else { 0 };
                    color((xs / (2 * p)) % 3)
                })
                .collect();
            (w, h, cells)
        }
        _ => (1, 1, vec![[1.0; 3]]),
    }
}

/// Décalage horizontal de la ligne `y` (instabilité), en pixels écran.
pub(crate) fn jitter_dx(fx: &Fx, y: usize, h: usize, sh: usize, frame: u64) -> f32 {
    if fx.jitter == 0 {
        return 0.0;
    }
    let amp = f32::from(fx.jitter.min(100)) / 100.0 * (h as f32 / sh.max(1) as f32) * 0.6;
    // numéro de trame réduit avant le passage en f32 (précision des grands numéros)
    let t = (frame % 1_000_000) as f32;
    let line = y as f32 * sh as f32 / h as f32;
    // ondulation lente + petit bruit par ligne de l'image d'origine
    let hash =
        ((line as u32).wrapping_mul(2_654_435_761) ^ (frame as u32).wrapping_mul(40_503)) >> 16;
    let noise = (hash & 0xFF) as f32 / 255.0 - 0.5;
    amp * (0.7 * (line * 0.045 + t * 0.21).sin() + 0.6 * noise)
}

/// Échantillon bilinéaire en virgule fixe (poids sur 8 bits), noir hors de l'image.
#[inline]
fn bilinear(src: &[Color32], w: usize, h: usize, sx: f32, sy: f32) -> Color32 {
    // partie entière par conversion décalée (sx, sy > −1) : évite l'appel à floor()
    let (xi, yi) = ((sx + 1.0) as isize - 1, (sy + 1.0) as isize - 1);
    let (ax, ay) = (
        ((sx - xi as f32) * 256.0) as u32,
        ((sy - yi as f32) * 256.0) as u32,
    );
    let (x0, y0) = (xi, yi);
    let rgb = |p: Color32| [u32::from(p.r()), u32::from(p.g()), u32::from(p.b())];
    let (a, b, c, d) = if x0 >= 0 && y0 >= 0 && x0 + 1 < w as isize && y0 + 1 < h as isize {
        // cas courant : les quatre voisins sont dans l'image
        let i = y0 as usize * w + x0 as usize;
        (
            rgb(src[i]),
            rgb(src[i + 1]),
            rgb(src[i + w]),
            rgb(src[i + w + 1]),
        )
    } else {
        let at = |x: isize, y: isize| {
            if x < 0 || y < 0 || x >= w as isize || y >= h as isize {
                [0; 3]
            } else {
                rgb(src[y as usize * w + x as usize])
            }
        };
        (
            at(x0, y0),
            at(x0 + 1, y0),
            at(x0, y0 + 1),
            at(x0 + 1, y0 + 1),
        )
    };
    let m = |i: usize| {
        let top = a[i] * (256 - ax) + b[i] * ax;
        let bot = c[i] * (256 - ax) + d[i] * ax;
        ((top * (256 - ay) + bot * ay + (1 << 15)) >> 16) as u8
    };
    Color32::from_rgb(m(0), m(1), m(2))
}

/// Courbure et instabilité : rééchantillonne l'image (rien si les deux sont désactivées).
/// Avec u, v ∈ [−1, 1] : u' = u (1 + v² k₀) et v' = v (1 + u² k₁) ; hors de [−1, 1], noir.
pub fn warp(out: &mut [Color32], w: usize, h: usize, sh: usize, fx: &Fx, frame: u64) {
    if fx.curve == 0 && fx.jitter == 0 {
        return;
    }
    let src = out.to_vec();
    let c = f32::from(fx.curve.min(100)) / 100.0;
    let k = (c * 3.0 / 32.0, c * 3.0 / 24.0);
    let (wf, hf) = (w as f32, h as f32);
    // par colonne : u et le facteur (1 + u² k₁)
    let us: Vec<(f32, f32)> = (0..w)
        .map(|x| {
            let u = (x as f32 + 0.5) / wf * 2.0 - 1.0;
            (u, 1.0 + u * u * k.1)
        })
        .collect();
    let us = &us;
    let src = &src;
    par_rows(out, w, |y, row| {
        let dx = jitter_dx(fx, y, h, sh, frame);
        let v = (y as f32 + 0.5) / hf * 2.0 - 1.0;
        let ku = 1.0 + v * v * k.0;
        // sx = a·u + b, sy = c·(facteur colonne) + d
        let (a, b) = (ku * 0.5 * wf, 0.5 * wf - 0.5 - dx);
        let (cc, d) = (v * 0.5 * hf, 0.5 * hf - 0.5);
        let (umax, vlim) = (1.0 / ku, v.abs());
        for (o, &(u, kv)) in row.iter_mut().zip(us) {
            *o = if u.abs() > umax || vlim * kv > 1.0 {
                Color32::BLACK
            } else {
                bilinear(src, w, h, a * u + b, cc * kv + d)
            };
        }
    });
}

#[cfg(test)]
#[path = "crt_tests.rs"]
mod tests;
