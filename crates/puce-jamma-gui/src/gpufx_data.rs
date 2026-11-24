//! Données des filtres sur carte graphique, préparées par le processeur : source (après overscan
//! et agrandissement ×2), paramètres uniformes et tables (les mêmes que `postfx::render`, pour un
//! rendu identique). Fonctions pures, testées sans carte graphique.

use crate::crt;
use crate::crt_light;
use crate::postfx::{self, Fx, ScanMode, NEUTRAL_TEMP};

/// Type d'un champ des paramètres uniformes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Ty {
    U,
    I,
    F,
}

/// Champs des paramètres, dans l'ordre du tampon (4 octets chacun) ; la structure WGSL est
/// produite à partir de cette liste (`uniform_wgsl`).
pub const FIELDS: &[(&str, Ty)] = &[
    ("w", Ty::U),
    ("h", Ty::U),
    ("sw", Ty::U),
    ("sh", Ty::U),
    ("mode", Ty::U),
    ("kernel_on", Ty::U),
    ("o_kern", Ty::U),
    ("o_vpos", Ty::U),
    ("o_hpos", Ty::U),
    ("o_wrow", Ty::U),
    ("o_wcol", Ty::U),
    ("o_bcol", Ty::U),
    ("o_brow", Ty::U),
    ("o_std", Ty::U),
    ("o_dark", Ty::U),
    ("o_bright", Ty::U),
    ("o_beam", Ty::U),
    ("o_gamma", Ty::U),
    ("o_sweep", Ty::U),
    ("o_jitter", Ty::U),
    ("o_lut", Ty::U),
    ("o_vcol", Ty::U),
    ("o_vrow", Ty::U),
    ("o_cols", Ty::U),
    ("o_rows", Ty::U),
    ("o_mask", Ty::U),
    ("pw", Ty::U),
    ("ph", Ty::U),
    ("warp_on", Ty::U),
    ("k0", Ty::F),
    ("k1", Ty::F),
    ("tube_on", Ty::U),
    ("conv_on", Ty::U),
    ("conv_rx", Ty::I),
    ("conv_ry", Ty::I),
    ("conv_bx", Ty::I),
    ("conv_by", Ty::I),
    ("gr", Ty::U),
    ("gg", Ty::U),
    ("gb", Ty::U),
    ("bloom_on", Ty::U),
    ("bs", Ty::U),
    ("bw", Ty::U),
    ("bh", Ty::U),
    ("br", Ty::U),
    ("glow_k", Ty::F),
    ("glow_n", Ty::F),
    ("fin_on", Ty::U),
    ("fin_int", Ty::U),
    ("black", Ty::F),
    ("lift", Ty::F),
    ("t0", Ty::F),
    ("t1", Ty::F),
    ("t2", Ty::F),
    ("pk", Ty::F),
    ("corner_r", Ty::F),
    ("rc", Ty::U),
    ("srgb_out", Ty::U),
];

/// Structure WGSL des paramètres (`struct P { … }`).
pub fn uniform_wgsl() -> String {
    let mut s = String::from("struct P {\n");
    for (name, ty) in FIELDS {
        let t = match ty {
            Ty::U => "u32",
            Ty::I => "i32",
            Ty::F => "f32",
        };
        s.push_str(&format!("    {name}: {t},\n"));
    }
    // taille multiple de 16 octets
    for i in 0..(4 - FIELDS.len() % 4) % 4 {
        s.push_str(&format!("    _pad{i}: u32,\n"));
    }
    s.push_str("};\n");
    s
}

/// Valeurs des paramètres, sérialisées dans l'ordre de `FIELDS`.
#[derive(Clone, Debug, PartialEq)]
pub struct Params {
    words: Vec<u32>,
}

impl Default for Params {
    fn default() -> Self {
        Params {
            words: vec![0; FIELDS.len().div_ceil(4) * 4],
        }
    }
}

impl Params {
    fn index(name: &str) -> usize {
        FIELDS
            .iter()
            .position(|(n, _)| *n == name)
            .unwrap_or_else(|| panic!("paramètre inconnu : {name}"))
    }

    pub fn set_u(&mut self, name: &str, v: u32) {
        debug_assert_eq!(FIELDS[Self::index(name)].1, Ty::U, "{name}");
        self.words[Self::index(name)] = v;
    }

    pub fn set_i(&mut self, name: &str, v: i32) {
        debug_assert_eq!(FIELDS[Self::index(name)].1, Ty::I, "{name}");
        self.words[Self::index(name)] = v as u32;
    }

    pub fn set_f(&mut self, name: &str, v: f32) {
        debug_assert_eq!(FIELDS[Self::index(name)].1, Ty::F, "{name}");
        self.words[Self::index(name)] = v.to_bits();
    }

    pub fn get_u(&self, name: &str) -> u32 {
        self.words[Self::index(name)]
    }

    pub fn bytes(&self) -> Vec<u8> {
        self.words.iter().flat_map(|w| w.to_le_bytes()).collect()
    }
}

/// Largeur de la texture des tables (une table est rangée ligne après ligne).
pub const TABLE_W: usize = 1024;

/// Tables mises bout à bout ; chaque `push` rend le décalage de la table ajoutée.
#[derive(Default)]
struct Tables(Vec<f32>);

impl Tables {
    fn push(&mut self, v: impl IntoIterator<Item = f32>) -> u32 {
        let o = self.0.len() as u32;
        self.0.extend(v);
        o
    }
}

/// Halo : taille des blocs de réduction, image réduite et rayon du flou.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Bloom {
    pub s: u32,
    pub bw: u32,
    pub bh: u32,
    pub r: u32,
}

/// Tout ce que la carte graphique doit recevoir pour une image filtrée.
pub struct Job {
    /// Source RGBA (alpha 255), `sw` × `sh`.
    pub src: Vec<u8>,
    pub sw: u32,
    pub sh: u32,
    pub w: u32,
    pub h: u32,
    pub params: Params,
    /// Tables (flottants ; les entiers y sont exacts), `TABLE_W` par rangée, complétées de zéros.
    pub tables: Vec<f32>,
    pub bloom: Option<Bloom>,
}

/// Prépare une image `w` × `h` filtrée par `fx` (mêmes conventions que `postfx::render`).
pub fn build_job(
    src: &[u32],
    src_w: usize,
    src_h: usize,
    fx: &Fx,
    w: usize,
    h: usize,
    frame: u64,
) -> Job {
    assert!(src.len() >= src_w * src_h && w > 0 && h > 0);
    let s = postfx::prepare_source(src, src_w, src_h, fx);
    let rgba = s
        .pixels
        .iter()
        .flat_map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8, 255])
        .collect();
    let mut p = Params::default();
    let mut t = Tables::default();
    for (k, v) in [("w", w), ("h", h), ("sw", s.w), ("sh", s.h)] {
        p.set_u(k, v as u32);
    }
    beam_part(&mut p, &mut t, fx);
    positions(&mut p, &mut t, fx, (s.w, s.h), (w, h));
    post_part(&mut p, &mut t, fx, (w, h, s.orig_h), frame);
    let bloom = bloom_part(&mut p, fx, (w, h, s.orig_h));
    if let Some(b) = bloom {
        bloom_positions(&mut p, &mut t, &b, w, h);
    }
    finish_part(&mut p, &mut t, fx, w, h);
    let mut tables = t.0;
    tables.resize(tables.len().div_ceil(TABLE_W).max(1) * TABLE_W, 0.0);
    Job {
        src: rgba,
        sw: s.w as u32,
        sh: s.h as u32,
        w: w as u32,
        h: h as u32,
        params: p,
        tables,
        bloom,
    }
}

/// Faisceau : scanlines, interpolation horizontale, gamma.
fn beam_part(p: &mut Params, t: &mut Tables, fx: &Fx) {
    let mode = match fx.scan_mode {
        ScanMode::Off => 0,
        ScanMode::Standard => 1,
        ScanMode::Adaptive => 2,
        ScanMode::Gaussian => 3,
    };
    p.set_u("mode", mode);
    p.set_u("kernel_on", u32::from(fx.interp > 0));
    let f = |v: &[i16]| v.iter().map(|x| f32::from(*x)).collect::<Vec<_>>();
    p.set_u("o_kern", t.push(f(postfx::sharp_table(fx.interp))));
    p.set_u("o_std", t.push(f(postfx::std_table(fx.scan))));
    let (dk, br) = postfx::adaptive_tables(fx.scan_dk, fx.scan_br);
    p.set_u("o_dark", t.push(f(dk)));
    p.set_u("o_bright", t.push(f(br)));
    let beam = if fx.scan_mode == ScanMode::Gaussian {
        crt::beam_table(fx.beam_dk, fx.beam_br)
    } else {
        Vec::new()
    };
    p.set_u("o_beam", t.push(beam));
    let lut = postfx::gamma_lut(fx);
    p.set_u("o_gamma", t.push(lut.iter().map(|v| (*v).min(255) as f32)));
}

/// Positions des filtres de faisceau, calculées comme `postfx::render` (en flottants du processeur :
/// la division des cartes graphiques n'est pas exacte et ferait changer de ligne ou de phase).
/// Par rangée de sortie, 5 valeurs : scanlines standard / adaptatives = (ligne, phase sur 64,
/// phase sur 256, ligne la plus proche, 0) ; faisceau gaussien = (ligne r0, puis l'indice de
/// distance des lignes r0 − 1 à r0 + 2). Par colonne, 2 valeurs : (colonne, phase sur 64) avec
/// interpolation, sinon (colonne, 0).
fn positions(
    p: &mut Params,
    t: &mut Tables,
    fx: &Fx,
    (sw, sh): (usize, usize),
    (w, h): (usize, usize),
) {
    let mut v = Vec::with_capacity(h * 5);
    for y in 0..h {
        if fx.scan_mode == ScanMode::Gaussian {
            let pf = (y as f32 + 0.5) * sh as f32 / h as f32;
            let r0 = (pf - 0.5).floor() as i32;
            v.push(r0 as f32);
            for j in -1..=2 {
                let d = (pf - ((r0 + j) as f32 + 0.5)).abs();
                v.push(((d * crt::BEAM_STEPS as f32) as usize).min(2 * crt::BEAM_STEPS) as f32);
            }
        } else {
            let pos = y as f32 * sh as f32 / h as f32;
            let fl = pos.floor();
            let frac = pos - fl;
            v.extend([
                fl,
                ((frac * 64.0) as usize).min(63) as f32,
                ((frac * 256.0) as usize).min(255) as f32,
                if frac <= 0.5 { 1.0 } else { 2.0 },
                0.0,
            ]);
        }
    }
    p.set_u("o_vpos", t.push(v));
    let mut c = Vec::with_capacity(w * 2);
    for x in 0..w {
        if fx.interp > 0 {
            let pos = (x as f32 + 0.5) * sw as f32 / w as f32 - 0.5;
            let i = pos.floor();
            c.extend([i, (((pos - i) * 64.0) as usize).min(63) as f32]);
        } else {
            c.extend([(x * sw / w).min(sw - 1) as f32, 0.0]);
        }
    }
    p.set_u("o_hpos", t.push(c));
}

/// Halo : pour chaque colonne puis chaque rangée de sortie, les deux points de l'image réduite et
/// le poids du second (sur 256), comme `crt_light::bloom`.
fn bloom_positions(p: &mut Params, t: &mut Tables, b: &Bloom, w: usize, h: usize) {
    let s = b.s as f32;
    let axis = |n: usize, m: u32| -> Vec<f32> {
        (0..n)
            .flat_map(|i| {
                let f = ((i as f32 + 0.5) / s - 0.5).clamp(0.0, (m - 1) as f32);
                let i0 = f as u32;
                [
                    i0 as f32,
                    (i0 + 1).min(m - 1) as f32,
                    ((f.fract() * 256.0) as u32) as f32,
                ]
            })
            .collect()
    };
    p.set_u("o_bcol", t.push(axis(w, b.bw)));
    p.set_u("o_brow", t.push(axis(h, b.bh)));
}

/// Géométrie, instabilité, convergence, gains des canaux, barre de balayage.
fn post_part(
    p: &mut Params,
    t: &mut Tables,
    fx: &Fx,
    (w, h, sh0): (usize, usize, usize),
    frame: u64,
) {
    p.set_u("warp_on", u32::from(fx.curve > 0 || fx.jitter > 0));
    let c = f32::from(fx.curve.min(100)) / 100.0;
    p.set_f("k0", c * 3.0 / 32.0);
    p.set_f("k1", c * 3.0 / 24.0);
    let jit: Vec<f32> = (0..h)
        .map(|y| crt::jitter_dx(fx, y, h, sh0, frame))
        .collect();
    // courbure, comme crt::warp : par rangée (a, b, c, d, 1/ku, |v|), par colonne (u, kv)
    let (wf, hf) = (w as f32, h as f32);
    let (k0, k1) = (c * 3.0 / 32.0, c * 3.0 / 24.0);
    let rows: Vec<f32> = (0..h)
        .flat_map(|y| {
            let v = (y as f32 + 0.5) / hf * 2.0 - 1.0;
            let ku = 1.0 + v * v * k0;
            [
                ku * 0.5 * wf,
                0.5 * wf - 0.5 - jit[y],
                v * 0.5 * hf,
                0.5 * hf - 0.5,
                1.0 / ku,
                v.abs(),
            ]
        })
        .collect();
    p.set_u("o_wrow", t.push(rows));
    let cols: Vec<f32> = (0..w)
        .flat_map(|x| {
            let u = (x as f32 + 0.5) / wf * 2.0 - 1.0;
            [u, 1.0 + u * u * k1]
        })
        .collect();
    p.set_u("o_wcol", t.push(cols));
    p.set_u("o_jitter", t.push(jit));
    let conv = (fx.conv_rx, fx.conv_ry, fx.conv_bx, fx.conv_by) != (0, 0, 0, 0);
    p.set_u("tube_on", u32::from(fx.tube_active()));
    p.set_u("conv_on", u32::from(conv));
    for (k, v) in [
        ("conv_rx", fx.conv_rx),
        ("conv_ry", fx.conv_ry),
        ("conv_bx", fx.conv_bx),
        ("conv_by", fx.conv_by),
    ] {
        p.set_i(k, i32::from(v.clamp(-8, 8)));
    }
    let gain = |pct: u8| (u32::from(pct.min(200)) * 256 + 50) / 100;
    p.set_u("gr", gain(fx.red));
    p.set_u("gg", gain(fx.green));
    p.set_u("gb", gain(fx.blue));
    let sweep = postfx::sweep_gains(fx, h, frame);
    p.set_u("o_sweep", t.push(sweep.iter().map(|v| *v as f32)));
}

/// Halo (mêmes tailles que `crt_light::bloom`).
fn bloom_part(p: &mut Params, fx: &Fx, (w, h, sh0): (usize, usize, usize)) -> Option<Bloom> {
    if fx.bloom == 0 || w < 4 || h < 4 {
        return None;
    }
    let s = (h / 270).max(1);
    let (bw, bh) = (w.div_ceil(s), h.div_ceil(s));
    let radius =
        (f32::from(fx.bloom_size.clamp(1, 10)) * h as f32 / sh0.max(1) as f32 / s as f32).max(1.0);
    let r = ((radius / 1.7) as usize).max(1);
    let b = Bloom {
        s: s as u32,
        bw: bw as u32,
        bh: bh as u32,
        r: r as u32,
    };
    p.set_u("bloom_on", 1);
    p.set_u("bs", b.s);
    p.set_u("bw", b.bw);
    p.set_u("bh", b.bh);
    p.set_u("br", b.r);
    p.set_f(
        "glow_k",
        f32::from(fx.bloom.min(100)) / 100.0 * 255.0 * 256.0,
    );
    p.set_f("glow_n", 1.0 / (s * s) as f32);
    Some(b)
}

/// Passe finale : couleur, taches de pureté, masque, vignettage, coins (comme `crt_light::finish`).
fn finish_part(p: &mut Params, t: &mut Tables, fx: &Fx, w: usize, h: usize) {
    let (pw, ph, cells) = postfx::mask_gains(fx, h);
    let neutral_mask = cells.len() == 1 && cells[0] == [256; 3];
    p.set_u("pw", pw as u32);
    p.set_u("ph", ph as u32);
    p.set_u(
        "o_mask",
        t.push(cells.iter().flat_map(|c| c.map(|v| v as f32))),
    );
    let on = !(fx.temp == NEUTRAL_TEMP
        && fx.black == 0
        && fx.purity == 0
        && fx.vignette == 0
        && fx.corner == 0
        && neutral_mask);
    p.set_u("fin_on", u32::from(on));
    let tg = crt_light::temp_gains(fx.temp);
    let black = f32::from(fx.black.min(20)) / 100.0 * 255.0;
    let lift = 1.0 - black / 255.0;
    let pk = f32::from(fx.purity.min(100)) / 100.0 * 0.25;
    let e = f32::from(fx.vignette.min(100)) / 100.0 * 0.6;
    let r = f32::from(fx.corner.min(100)) / 100.0 * 0.12 * w.min(h) as f32;
    let (cols, rows) = (crt_light::profile(w, e), crt_light::profile(h, e));
    p.set_u("fin_int", u32::from(pk == 0.0));
    p.set_f("black", black);
    p.set_f("lift", lift);
    p.set_f("t0", tg[0]);
    p.set_f("t1", tg[1]);
    p.set_f("t2", tg[2]);
    p.set_f("pk", pk);
    p.set_f("corner_r", r);
    p.set_u("rc", r.ceil() as u32);
    let lut = (0..3).flat_map(|c| {
        (0..256).map(move |v| ((black + v as f32 * tg[c] * lift).min(255.0) as u32) as f32)
    });
    p.set_u("o_lut", t.push(lut));
    let vig = |v: &[[f32; 3]]| {
        v.iter()
            .map(|q| ((q[0] * 256.0 + 0.5) as u32) as f32)
            .collect::<Vec<_>>()
    };
    p.set_u("o_vcol", t.push(vig(&cols)));
    p.set_u("o_vrow", t.push(vig(&rows)));
    p.set_u("o_cols", t.push(cols.iter().flatten().copied()));
    p.set_u("o_rows", t.push(rows.iter().flatten().copied()));
}

#[cfg(test)]
#[path = "gpufx_data_tests.rs"]
mod tests;
