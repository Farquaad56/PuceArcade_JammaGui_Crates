//! Lumière du tube : halo (lumière diffusée dans le verre), puis une passe finale unique qui pose
//! la couleur (température du blanc, taches de pureté, niveau du noir), le masque d'écran et le
//! vignettage avec les coins arrondis. Les gains qui varient sur l'écran sont séparables
//! (colonne × rangée) et précalculés.
//!
//! Température du blanc : approximation de Tanner Helland ; vignettage : forme des shaders CRT
//! publics de Timothy Lottes.

use eframe::egui::Color32;

use crate::crt::{par_rows, MAX_THREADS};
use crate::postfx::{Fx, NEUTRAL_TEMP};

/// Couleur du blanc à `kelvin` (approximation de Tanner Helland), composantes 0 à 1.
fn kelvin_rgb(kelvin: f32) -> [f32; 3] {
    let t = kelvin / 100.0;
    let r = if t <= 66.0 {
        255.0
    } else {
        329.7 * (t - 60.0).powf(-0.133_2)
    };
    let g = if t <= 66.0 {
        99.47 * t.ln() - 161.12
    } else {
        288.12 * (t - 60.0).powf(-0.075_5)
    };
    let b = if t >= 66.0 {
        255.0
    } else if t <= 19.0 {
        0.0
    } else {
        138.52 * (t - 10.0).ln() - 305.04
    };
    [r, g, b].map(|v| v.clamp(0.0, 255.0) / 255.0)
}

/// Gains RVB pour passer du blanc neutre (6 500 K) à `temp` centaines de kelvins, le plus fort à 1.
pub fn temp_gains(temp: u8) -> [f32; 3] {
    if temp == NEUTRAL_TEMP {
        return [1.0; 3];
    }
    let (a, n) = (
        kelvin_rgb(f32::from(temp.clamp(50, 93)) * 100.0),
        kelvin_rgb(6_500.0),
    );
    let g: [f32; 3] = std::array::from_fn(|i| a[i] / n[i].max(1e-3));
    let m = g.iter().cloned().fold(0.0, f32::max).max(1e-3);
    g.map(|v| v / m)
}

/// Masque d'écran prêt à poser : motif `pw` × `ph`, gains RVB en 8.8 (256 = inchangé).
pub struct MaskGains<'a> {
    pub pw: usize,
    pub ph: usize,
    pub cells: &'a [[u32; 3]],
}

impl MaskGains<'_> {
    fn neutral(&self) -> bool {
        self.cells.len() == 1 && self.cells[0] == [256; 3]
    }
}

/// Taches de pureté : rayon des taches (en fraction de l'écran, au carré).
const BLOB: f32 = 0.09;

/// Profils séparables d'une dimension : vignettage, taches près de 0 et près de 1.
pub(crate) fn profile(n: usize, e: f32) -> Vec<[f32; 3]> {
    (0..n)
        .map(|i| {
            let u = (i as f32 + 0.5) / n as f32;
            let vig = if e > 0.0 {
                (4.0 * u * (1.0 - u)).clamp(0.0, 1.0).powf(e)
            } else {
                1.0
            };
            [
                vig,
                (-(u * u) / BLOB).exp(),
                (-((u - 1.0) * (u - 1.0)) / BLOB).exp(),
            ]
        })
        .collect()
}

/// Passe finale : couleur, masque, vignettage et coins arrondis.
pub fn finish(out: &mut [Color32], w: usize, h: usize, fx: &Fx, mask: &MaskGains) {
    if fx.temp == NEUTRAL_TEMP
        && fx.black == 0
        && fx.purity == 0
        && fx.vignette == 0
        && fx.corner == 0
        && mask.neutral()
    {
        return;
    }
    let t = temp_gains(fx.temp);
    let black = f32::from(fx.black.min(20)) / 100.0 * 255.0;
    let lift = 1.0 - black / 255.0;
    let pk = f32::from(fx.purity.min(100)) / 100.0 * 0.25;
    let e = f32::from(fx.vignette.min(100)) / 100.0 * 0.6;
    let r = f32::from(fx.corner.min(100)) / 100.0 * 0.12 * w.min(h) as f32;
    let (cols, rows) = (profile(w, e), profile(h, e));
    if pk == 0.0 {
        finish_int(out, (w, h), (t, black, lift), r, (&cols, &rows), mask);
        return;
    }
    let (cols, rows) = (&cols, &rows);
    par_rows(out, w, |y, row| {
        let ry = rows[y];
        let near_y = (y as f32) < r || (y as f32 + r) >= h as f32;
        let mrow = (y % mask.ph) * mask.pw;
        for (x, o) in row.iter_mut().enumerate() {
            let cx = cols[x];
            let mut vig = cx[0] * ry[0];
            if near_y {
                vig *= corner(x, y, w, h, r);
            }
            // taches : magenta en haut à gauche, vert en bas à droite, bleu en haut à droite
            let pg = if pk > 0.0 {
                let (m, g, b) = (cx[1] * ry[1], cx[2] * ry[2], cx[2] * ry[1] * 0.7);
                [
                    1.0 + pk * (m - g - 0.5 * b),
                    1.0 + pk * (g - m - 0.5 * b),
                    1.0 + pk * (m + b - g),
                ]
            } else {
                [1.0; 3]
            };
            let mg = &mask.cells[mrow + x % mask.pw];
            let c = [o.r(), o.g(), o.b()];
            let f = |i: usize| {
                let v = (black + f32::from(c[i]) * t[i] * pg[i] * lift).min(255.0);
                (v * mg[i] as f32 * (1.0 / 256.0) * vig).min(255.0) as u8
            };
            *o = Color32::from_rgb(f(0), f(1), f(2));
        }
    });
}

/// Coin arrondi : facteur 0 à 1 (antialiasé sur 1 px) au pixel (x, y) pour le rayon `r`.
#[inline]
pub(crate) fn corner(x: usize, y: usize, w: usize, h: usize, r: f32) -> f32 {
    let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
    let dx = (r - fx.min(w as f32 - fx)).max(0.0);
    let dy = (r - fy.min(h as f32 - fy)).max(0.0);
    if dx > 0.0 && dy > 0.0 {
        (0.5 - ((dx * dx + dy * dy).sqrt() - r)).clamp(0.0, 1.0)
    } else {
        1.0
    }
}

/// Passe finale sans taches de pureté, en entiers : table par canal (température, noir), puis
/// gain du masque (8.8) × vignettage (8 bits).
fn finish_int(
    out: &mut [Color32],
    (w, h): (usize, usize),
    (t, black, lift): ([f32; 3], f32, f32),
    r: f32,
    (cols, rows): (&[[f32; 3]], &[[f32; 3]]),
    mask: &MaskGains,
) {
    let lut: [[u32; 256]; 3] = std::array::from_fn(|c| {
        std::array::from_fn(|v| (black + v as f32 * t[c] * lift).min(255.0) as u32)
    });
    let vcol: Vec<u32> = cols.iter().map(|p| (p[0] * 256.0 + 0.5) as u32).collect();
    let rc = r.ceil() as usize;
    let lut = &lut;
    let vcol = &vcol;
    par_rows(out, w, |y, row| {
        let vy = (rows[y][0] * 256.0 + 0.5) as u32;
        let near_y = y < rc || y + rc >= h;
        let mrow = (y % mask.ph) * mask.pw;
        for (x, o) in row.iter_mut().enumerate() {
            let mut vig = (vcol[x] * vy + 128) >> 8;
            if near_y && (x < rc || x + rc >= w) {
                vig = (vig as f32 * corner(x, y, w, h, r)) as u32;
            }
            let mg = &mask.cells[mrow + x % mask.pw];
            let f = |c: usize, v: u8| {
                ((lut[c][usize::from(v)] * mg[c] * vig + (1 << 15)) >> 16).min(255) as u8
            };
            *o = Color32::from_rgb(f(0, o.r()), f(1, o.g()), f(2, o.b()));
        }
    });
}

/// Halo : flou de l'image (en lumière quasi linéaire, à basse définition) ajouté en mode « écran ».
/// `sh` = hauteur de l'image d'origine (taille du halo en lignes d'origine).
pub fn bloom(out: &mut [Color32], w: usize, h: usize, sh: usize, fx: &Fx) {
    if fx.bloom == 0 || w < 4 || h < 4 {
        return;
    }
    let s = (h / 270).max(1);
    let (bw, bh) = (w.div_ceil(s), h.div_ceil(s));
    let mut buf = downsample(out, w, s, bw, bh);
    let radius =
        (f32::from(fx.bloom_size.clamp(1, 10)) * h as f32 / sh.max(1) as f32 / s as f32).max(1.0);
    let r = ((radius / 1.7) as usize).max(1);
    for _ in 0..3 {
        box_blur(&mut buf, bw, bh, r);
    }
    // lueur finale (racine, intensité) à basse définition, en 0..255 × 256
    let k = f32::from(fx.bloom.min(100)) / 100.0 * 255.0 * 256.0;
    let n = 1.0 / (s * s) as f32;
    let glow: Vec<[u32; 3]> = buf
        .iter()
        // max(0) : la somme glissante du flou laisse de petits négatifs dans le noir (sqrt → NaN)
        .map(|b| b.map(|v| ((v * n).max(0.0).sqrt() * k).min(65_280.0) as u32))
        .collect();
    let xs: Vec<(usize, usize, u32)> = (0..w)
        .map(|x| {
            let bx = ((x as f32 + 0.5) / s as f32 - 0.5).clamp(0.0, (bw - 1) as f32);
            let x0 = bx as usize;
            (x0, (x0 + 1).min(bw - 1), (bx.fract() * 256.0) as u32)
        })
        .collect();
    let (glow, xs) = (&glow, &xs);
    par_rows(out, w, |y, row| {
        let by = ((y as f32 + 0.5) / s as f32 - 0.5).clamp(0.0, (bh - 1) as f32);
        let (y0, y1) = (by as usize, (by as usize + 1).min(bh - 1));
        let fy = (by.fract() * 256.0) as u32;
        // rangée basse définition interpolée verticalement une fois pour toute la rangée
        let line: Vec<[u32; 3]> = (0..bw)
            .map(|x| {
                let (a, b) = (glow[y0 * bw + x], glow[y1 * bw + x]);
                std::array::from_fn(|c| (a[c] * (256 - fy) + b[c] * fy) >> 8)
            })
            .collect();
        for (o, &(x0, x1, fx_)) in row.iter_mut().zip(xs) {
            let (a, b) = (&line[x0], &line[x1]);
            let f = |c: usize, base: u8| {
                let g = ((a[c] * (256 - fx_) + b[c] * fx_) >> 16).min(255);
                let base = u32::from(base);
                // écran : base + g (255 − base) / 255
                (base + ((g * (255 - base) * 257) >> 16)).min(255) as u8
            };
            *o = Color32::from_rgb(f(0, o.r()), f(1, o.g()), f(2, o.b()));
        }
    });
}

/// Réduction de l'image par blocs `s` × `s` en lumière quasi linéaire (somme des carrés),
/// répartie sur plusieurs fils par rangées de blocs.
fn downsample(out: &[Color32], w: usize, s: usize, bw: usize, bh: usize) -> Vec<[f32; 3]> {
    let sq: Vec<f32> = (0..256).map(|v| (v as f32 / 255.0).powi(2)).collect();
    let mut buf = vec![[0f32; 3]; bw * bh];
    let h = out.len() / w;
    let threads = std::thread::available_parallelism()
        .map_or(1, |n| n.get())
        .min(MAX_THREADS);
    let per = bh.div_ceil(threads).max(1);
    let sq = &sq;
    std::thread::scope(|sc| {
        for (i, chunk) in buf.chunks_mut(per * bw).enumerate() {
            sc.spawn(move || {
                for (dby, line) in chunk.chunks_mut(bw).enumerate() {
                    let by = i * per + dby;
                    for y in by * s..((by + 1) * s).min(h) {
                        for (x, p) in out[y * w..(y + 1) * w].iter().enumerate() {
                            let b = &mut line[x / s];
                            b[0] += sq[usize::from(p.r())];
                            b[1] += sq[usize::from(p.g())];
                            b[2] += sq[usize::from(p.b())];
                        }
                    }
                }
            });
        }
    });
    buf
}

/// Une passe de flou en boîte le long de `len` éléments espacés de `step`, pour `count` lignes
/// commençant tous les `stride` éléments.
fn blur_lines(
    src: &[[f32; 3]],
    dst: &mut [[f32; 3]],
    (len, step): (usize, usize),
    (count, stride): (usize, usize),
    r: usize,
) {
    let norm = 1.0 / (2 * r + 1) as f32;
    for line in 0..count {
        let base = line * stride;
        let at = |i: isize| src[base + i.clamp(0, len as isize - 1) as usize * step];
        let mut acc = [0f32; 3];
        for i in -(r as isize)..=(r as isize) {
            let p = at(i);
            (0..3).for_each(|c| acc[c] += p[c]);
        }
        for i in 0..len {
            dst[base + i * step] = acc.map(|v| v * norm);
            let (add, sub) = (at(i as isize + r as isize + 1), at(i as isize - r as isize));
            (0..3).for_each(|c| acc[c] += add[c] - sub[c]);
        }
    }
}

/// Flou en boîte séparable de rayon `r`.
fn box_blur(buf: &mut [[f32; 3]], w: usize, h: usize, r: usize) {
    let mut tmp = buf.to_vec();
    blur_lines(buf, &mut tmp, (w, 1), (h, w), r);
    blur_lines(&tmp, buf, (h, w), (w, 1), r);
}
