//! Filtres vidéo logiciels, calculés à la taille finale de l'image (après le facteur
//! d'agrandissement) : interpolation horizontale, scanlines (standard ou adaptatives), gamma et
//! masque d'écran.
//!
//! Les coefficients viennent des fichiers de filtres de la communauté MiSTer, regroupés dans
//! `data/fx.bin` par `tools/gen_fx_data.py` ; les masques sont dans `data/masks/`.

use std::sync::OnceLock;

use eframe::egui::Color32;

use crate::pixel2x::{self, Scaler};

/// Scanlines.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum ScanMode {
    #[default]
    Off,
    /// Même profondeur sur toute l'image.
    Standard,
    /// Profondeur variable selon la luminosité du pixel.
    Adaptive,
}

/// Masque d'écran.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mask {
    #[default]
    None,
    /// Grille d'ouverture type Sony Trinitron.
    Trinitron,
    /// Tube fané : teinte jaunâtre.
    FadedYellow,
}

/// Réglages des filtres (tous les pourcentages sont des entiers).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Fx {
    /// Agrandissement ×2 pour le pixel art, appliqué avant tous les autres filtres.
    pub scaler: Scaler,
    /// Netteté de l'interpolation horizontale en % (0 = aucune ; sinon multiple de 5, 5 à 100).
    pub interp: u8,
    pub scan_mode: ScanMode,
    /// Profondeur des scanlines standard, en % (0 à 95, pas de 5).
    pub scan: u8,
    /// Scanlines adaptatives : luminosité conservée au milieu de la ligne pour les pixels sombres,
    /// en % (multiple de 10, au plus `scan_br`).
    pub scan_dk: u8,
    /// Idem pour les pixels clairs, en % (multiple de 10).
    pub scan_br: u8,
    /// Gamma en centièmes (100 = désactivé, 145 = 1,45).
    pub gamma: u8,
    pub mask: Mask,
    /// Force du masque en %.
    pub mask_pct: u8,
    /// Tube : gain des canaux rouge, vert, bleu en % (100 = neutre, 0 à 200).
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    /// Tube : convergence, décalage du rouge et du bleu par rapport au vert, en pixels écran
    /// (−8 à 8 ; x positif vers la droite, y positif vers le bas).
    pub conv_rx: i8,
    pub conv_ry: i8,
    pub conv_bx: i8,
    pub conv_by: i8,
    /// Tube : barre de balayage, intensité en % (0 = désactivée).
    pub sweep: u8,
    /// Durée d'un passage de la barre, en dixièmes de seconde (5 à 100).
    pub sweep_speed: u8,
    /// Hauteur de la barre en % de l'image (2 à 50).
    pub sweep_width: u8,
}

impl Default for Fx {
    fn default() -> Self {
        Fx {
            scaler: Scaler::Off,
            interp: 0,
            scan_mode: ScanMode::Off,
            scan: 50,
            scan_dk: 40,
            scan_br: 80,
            gamma: 100,
            mask: Mask::None,
            mask_pct: 100,
            red: 100,
            green: 100,
            blue: 100,
            conv_rx: 0,
            conv_ry: 0,
            conv_bx: 0,
            conv_by: 0,
            sweep: 0,
            sweep_speed: 40,
            sweep_width: 12,
        }
    }
}

/// Préréglages d'écran.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Preset {
    None,
    SonyTrinitron,
    FadedTube,
}

impl Preset {
    pub const ALL: [Preset; 3] = [Preset::None, Preset::SonyTrinitron, Preset::FadedTube];

    pub fn label(self) -> &'static str {
        match self {
            Preset::None => "Aucun",
            Preset::SonyTrinitron => "Sony Trinitron",
            Preset::FadedTube => "Tube fané (Faded Tube)",
        }
    }
}

/// Netteté des trois interpolations proposées.
pub const SOFT: u8 = 55;
pub const MEDIUM: u8 = 70;
pub const SHARP: u8 = 85;

impl Fx {
    pub fn preset(p: Preset) -> Fx {
        let base = Fx::default();
        match p {
            Preset::None => base,
            Preset::SonyTrinitron => Fx {
                interp: 35,
                scan_mode: ScanMode::Adaptive,
                scan_dk: 40,
                scan_br: 80,
                gamma: 110,
                mask: Mask::Trinitron,
                ..base
            },
            Preset::FadedTube => Fx {
                interp: 15,
                scan_mode: ScanMode::Adaptive,
                scan_dk: 20,
                scan_br: 60,
                gamma: 145,
                mask: Mask::FadedYellow,
                ..base
            },
        }
    }

    /// Préréglage correspondant exactement aux réglages, s'il y en a un.
    pub fn detect(&self) -> Option<Preset> {
        Preset::ALL.into_iter().find(|p| {
            let q = Fx::preset(*p);
            // les curseurs inutilisés ne comptent pas
            self.norm() == q.norm()
        })
    }

    /// Réglages avec les valeurs sans effet ramenées à leur défaut (pour comparer).
    fn norm(&self) -> Fx {
        let d = Fx::default();
        let mut f = self.without_tube();
        f.scaler = d.scaler;
        if f.scan_mode != ScanMode::Standard {
            f.scan = d.scan;
        }
        if f.scan_mode != ScanMode::Adaptive {
            f.scan_dk = d.scan_dk;
            f.scan_br = d.scan_br;
        }
        if f.mask == Mask::None {
            f.mask_pct = d.mask_pct;
        }
        f
    }

    /// Au moins un filtre agit-il sur l'image ?
    pub fn is_active(&self) -> bool {
        self.scaler != Scaler::Off
            || self.interp > 0
            || self.scan_mode != ScanMode::Off
            || self.gamma != 100
            || self.mask != Mask::None
            || self.tube_active()
    }

    /// Un réglage de tube (canaux, convergence, balayage) agit-il ?
    pub fn tube_active(&self) -> bool {
        (self.red, self.green, self.blue) != (100, 100, 100)
            || (self.conv_rx, self.conv_ry, self.conv_bx, self.conv_by) != (0, 0, 0, 0)
            || self.sweep > 0
    }

    /// Mêmes réglages, tube remis à neutre (les préréglages d'écran ne touchent pas au tube).
    pub fn without_tube(&self) -> Fx {
        let d = Fx::default();
        Fx {
            red: d.red,
            green: d.green,
            blue: d.blue,
            conv_rx: 0,
            conv_ry: 0,
            conv_bx: 0,
            conv_by: 0,
            sweep: 0,
            sweep_speed: d.sweep_speed,
            sweep_width: d.sweep_width,
            ..*self
        }
    }

    /// Copie les réglages de tube de `other` (choix d'un préréglage d'écran).
    pub fn with_tube_of(&self, other: &Fx) -> Fx {
        Fx {
            red: other.red,
            green: other.green,
            blue: other.blue,
            conv_rx: other.conv_rx,
            conv_ry: other.conv_ry,
            conv_bx: other.conv_bx,
            conv_by: other.conv_by,
            sweep: other.sweep,
            sweep_speed: other.sweep_speed,
            sweep_width: other.sweep_width,
            ..*self
        }
    }
}

/// Tables de coefficients (4 par phase, somme 128 pour les tables à 64 phases, 256 pour SLA).
fn data() -> &'static [i16] {
    static D: OnceLock<Vec<i16>> = OnceLock::new();
    D.get_or_init(|| {
        include_bytes!("../data/fx.bin")
            .chunks_exact(2)
            .map(|b| i16::from_le_bytes([b[0], b[1]]))
            .collect()
    })
}

const N_SHARP: usize = 21;
const N_STD: usize = 20;
const STD_OFF: usize = N_SHARP * 256;
const DARK_OFF: usize = STD_OFF + N_STD * 256;
const BRIGHT_OFF: usize = DARK_OFF + 11 * 1024;

/// Table (64 phases) du filtre d'interpolation de netteté `sharp` (%).
fn sharp_table(sharp: u8) -> &'static [i16] {
    let i = ((usize::from(sharp) + 2) / 5).min(N_SHARP - 1);
    &data()[i * 256..(i + 1) * 256]
}

/// Table (64 phases) des scanlines standard de profondeur `depth` (%).
fn std_table(depth: u8) -> &'static [i16] {
    let i = (usize::from(depth.min(95)) / 5).min(N_STD - 1);
    &data()[STD_OFF + i * 256..STD_OFF + (i + 1) * 256]
}

/// Tables (256 phases) sombre et claire des scanlines adaptatives.
fn adaptive_tables(dk: u8, br: u8) -> (&'static [i16], &'static [i16]) {
    let dk10 = ((usize::from(dk) + 5) / 10).min(10);
    let br10 = ((usize::from(br) + 5) / 10).min(10).max(dk10);
    let dark = DARK_OFF + dk10 * 1024;
    let before: usize = (0..dk10).map(|d| 11 - d).sum();
    let bright = BRIGHT_OFF + (before + br10 - dk10) * 1024;
    (&data()[dark..dark + 1024], &data()[bright..bright + 1024])
}

/// Motif de masque : taille et gain RVB de chaque cellule.
struct Pattern {
    res: usize,
    w: usize,
    h: usize,
    cells: Vec<[f32; 3]>,
}

/// Lit un fichier de masque (format « v2 » : une cellule = couleur, intensité allumée, éteinte).
fn parse_mask(text: &str) -> Vec<Pattern> {
    let mut out: Vec<Pattern> = Vec::new();
    let mut res = 0usize;
    let mut lines = text.lines().map(str::trim).filter(|l| !l.starts_with('#'));
    while let Some(l) = lines.next() {
        if let Some(r) = l.strip_prefix("Resolution=") {
            res = r.parse().unwrap_or(0);
        } else if l == "v2" {
            let Some((w, h)) = lines
                .next()
                .and_then(|d| d.split_once(','))
                .and_then(|(a, b)| Some((a.parse::<usize>().ok()?, b.parse::<usize>().ok()?)))
            else {
                break;
            };
            let mut cells = Vec::with_capacity(w * h);
            for _ in 0..h {
                for c in lines.next().unwrap_or("").split(',') {
                    let d: Vec<u32> = c.trim().chars().filter_map(|x| x.to_digit(16)).collect();
                    if d.len() != 3 {
                        cells.push([1.0; 3]);
                        continue;
                    }
                    let (on, off) = (1.0 + 0.0625 * d[1] as f32, 0.0625 * d[2] as f32);
                    // bits de couleur : bleu 1, vert 2, rouge 4
                    let g = |bit: u32| if d[0] & bit != 0 { on } else { off };
                    cells.push([g(4), g(2), g(1)]);
                }
            }
            cells.resize(w * h, [1.0; 3]);
            out.push(Pattern { res, w, h, cells });
        }
    }
    out
}

fn mask_patterns(m: Mask) -> Option<&'static [Pattern]> {
    static T: OnceLock<[Vec<Pattern>; 2]> = OnceLock::new();
    let t = T.get_or_init(|| {
        [
            parse_mask(include_str!("../data/masks/trinitron.txt")),
            parse_mask(include_str!("../data/masks/faded_tube_yellow.txt")),
        ]
    });
    match m {
        Mask::None => None,
        Mask::Trinitron => Some(&t[0]),
        Mask::FadedYellow => Some(&t[1]),
    }
}

/// Données communes à toutes les rangées de sortie.
struct Ctx<'a> {
    src: &'a [[u8; 3]],
    /// Taille de la source (doublée si un agrandissement ×2 est actif).
    sw: usize,
    sh: usize,
    fx: &'a Fx,
    w: usize,
    h: usize,
    /// Colonnes : indices source (4 points) et poids ; sans interpolation, un seul indice.
    cols: Vec<([u16; 4], [i32; 4])>,
    kernel_on: bool,
    lut: Vec<u32>,
    /// Gains du motif de masque en 8.8 (256 = inchangé).
    pw: usize,
    ph: usize,
    cells: Vec<[u32; 3]>,
    xmod: Vec<u16>,
    std_t: &'static [i16],
    dark_t: &'static [i16],
    bright_t: &'static [i16],
}

/// Calcule les rangées de sortie `y0..y0 + out.len() / w`.
fn render_rows(c: &Ctx, y0: usize, out: &mut [Color32]) {
    let (sw, sh) = (c.sw, c.sh);
    let (w, h) = (c.w, c.h);
    let mut vrow = vec![[0u8; 3]; sw];
    for (dy, orow) in out.chunks_mut(w).enumerate() {
        let y = y0 + dy;
        // 1. vertical, à la largeur de la source : filtre à 4 lignes selon la phase
        let pos = y as f32 * sh as f32 / h as f32;
        let fl = pos.floor();
        let frac = pos - fl;
        let i0 = fl as i32;
        let row = |j: usize| -> &[[u8; 3]] {
            let r = (i0 - 1 + j as i32).clamp(0, sh as i32 - 1) as usize;
            &c.src[r * sw..(r + 1) * sw]
        };
        match c.fx.scan_mode {
            ScanMode::Off => {
                let r = ((y * sh) / h).min(sh - 1);
                vrow.copy_from_slice(&c.src[r * sw..(r + 1) * sw]);
            }
            ScanMode::Standard => {
                let ph = ((frac * 64.0) as usize).min(63);
                let wg: [i32; 4] = std::array::from_fn(|j| i32::from(c.std_t[ph * 4 + j]));
                let rows: [&[[u8; 3]]; 4] = std::array::from_fn(row);
                for (x, o) in vrow.iter_mut().enumerate() {
                    for ch in 0..3 {
                        let mut a = 64;
                        for j in 0..4 {
                            a += wg[j] * i32::from(rows[j][x][ch]);
                        }
                        o[ch] = (a >> 7).clamp(0, 255) as u8;
                    }
                }
            }
            ScanMode::Adaptive => {
                let ph = ((frac * 256.0) as usize).min(255);
                let near = if frac <= 0.5 { 1 } else { 2 };
                let rows: [&[[u8; 3]]; 4] = std::array::from_fn(row);
                let d: [f32; 4] = std::array::from_fn(|j| f32::from(c.dark_t[ph * 4 + j]));
                let b: [f32; 4] = std::array::from_fn(|j| f32::from(c.bright_t[ph * 4 + j]));
                for (x, o) in vrow.iter_mut().enumerate() {
                    let n = rows[near][x];
                    let l = (0.299 * f32::from(n[0])
                        + 0.587 * f32::from(n[1])
                        + 0.114 * f32::from(n[2]))
                        / 255.0;
                    let wg: [f32; 4] = std::array::from_fn(|j| (d[j] + (b[j] - d[j]) * l) / 256.0);
                    for ch in 0..3 {
                        let mut a = 0.5;
                        for j in 0..4 {
                            a += wg[j] * f32::from(rows[j][x][ch]);
                        }
                        o[ch] = a.clamp(0.0, 255.0) as u8;
                    }
                }
            }
        }
        // 2. horizontal (interpolation ou répétition), gamma, masque
        let mrow = (y % c.ph) * c.pw;
        for (x, o) in orow.iter_mut().enumerate() {
            let (idx, wg) = &c.cols[x];
            let mut v = [0i32; 3];
            if c.kernel_on {
                for ch in 0..3 {
                    let mut a = 64;
                    for j in 0..4 {
                        a += wg[j] * i32::from(vrow[usize::from(idx[j])][ch]);
                    }
                    v[ch] = (a >> 7).clamp(0, 255);
                }
            } else {
                let p = vrow[usize::from(idx[0])];
                v = [i32::from(p[0]), i32::from(p[1]), i32::from(p[2])];
            }
            let g = &c.cells[mrow + usize::from(c.xmod[x])];
            let f = |ch: usize| ((c.lut[v[ch] as usize] * g[ch] + 128) >> 8).min(255) as u8;
            *o = Color32::from_rgb(f(0), f(1), f(2));
        }
    }
}

/// Applique les filtres : `src` = image (0x00RRGGBB), sortie `w` x `h`.
/// `frame` = numéro de trame (position de la barre de balayage).
pub fn render(
    src: &[u32],
    src_w: usize,
    src_h: usize,
    fx: &Fx,
    w: usize,
    h: usize,
    frame: u64,
) -> Vec<Color32> {
    assert!(src.len() >= src_w * src_h && w > 0 && h > 0);
    let big = pixel2x::upscale(src, src_w, src_h, fx.scaler);
    let (sw, sh, pix) = match &big {
        Some(v) => (2 * src_w, 2 * src_h, &v[..]),
        None => (src_w, src_h, &src[..src_w * src_h]),
    };
    let src: Vec<[u8; 3]> = pix
        .iter()
        .map(|p| [(p >> 16) as u8, (p >> 8) as u8, *p as u8])
        .collect();

    // colonnes
    let kernel_on = fx.interp > 0;
    let k = sharp_table(fx.interp);
    let cols: Vec<([u16; 4], [i32; 4])> = (0..w)
        .map(|x| {
            if !kernel_on {
                return ([(x * sw / w).min(sw - 1) as u16, 0, 0, 0], [0; 4]);
            }
            let pos = (x as f32 + 0.5) * sw as f32 / w as f32 - 0.5;
            let i = pos.floor();
            let ph = (((pos - i) * 64.0) as usize).min(63);
            let idx =
                std::array::from_fn(|j| (i as i32 - 1 + j as i32).clamp(0, sw as i32 - 1) as u16);
            let wg = std::array::from_fn(|j| i32::from(k[ph * 4 + j]));
            (idx, wg)
        })
        .collect();

    // gamma
    let g = f32::from(fx.gamma) / 100.0;
    let lut: Vec<u32> = (0..256)
        .map(|v| {
            if fx.gamma == 100 {
                v as u32
            } else {
                (255.0 * (v as f32 / 255.0).powf(g)).round() as u32
            }
        })
        .collect();

    // masque (motif choisi selon la hauteur, comme le mode « 1x »), gains en 8.8
    let mp = f32::from(fx.mask_pct) / 100.0;
    let pat =
        mask_patterns(fx.mask).and_then(|ps| ps.iter().rev().find(|p| p.res <= h).or(ps.first()));
    let (pw, ph, cells) = match pat {
        Some(p) => (
            p.w,
            p.h,
            p.cells
                .iter()
                .map(|c| {
                    std::array::from_fn(|ch| ((1.0 - mp * (1.0 - c[ch])) * 256.0 + 0.5) as u32)
                })
                .collect(),
        ),
        None => (1, 1, vec![[256u32; 3]]),
    };
    let xmod = (0..w).map(|x| (x % pw) as u16).collect();

    let (dark_t, bright_t) = adaptive_tables(fx.scan_dk, fx.scan_br);
    let ctx = Ctx {
        src: &src,
        sw,
        sh,
        fx,
        w,
        h,
        cols,
        kernel_on,
        lut,
        pw,
        ph,
        cells,
        xmod,
        std_t: std_table(fx.scan),
        dark_t,
        bright_t,
    };

    let mut out = vec![Color32::BLACK; w * h];
    let threads = if w * h < 200_000 {
        1
    } else {
        std::thread::available_parallelism()
            .map_or(1, |n| n.get())
            .min(8)
    };
    if threads <= 1 {
        render_rows(&ctx, 0, &mut out);
    } else {
        let per = h.div_ceil(threads);
        std::thread::scope(|s| {
            for (i, chunk) in out.chunks_mut(per * w).enumerate() {
                let ctx = &ctx;
                s.spawn(move || render_rows(ctx, i * per, chunk));
            }
        });
    }
    if fx.tube_active() {
        tube(&mut out, w, h, fx, frame);
    }
    out
}

/// Trames par seconde supposées pour la vitesse de la barre de balayage.
const SWEEP_FPS: f32 = 60.0;

/// Gain de la barre de balayage pour chaque ligne, en 8.8 (256 = neutre).
/// La barre descend de haut en bas en `sweep_speed / 10` secondes ; profil en cosinus, plus
/// clair au centre, avec une traînée sombre légère au-dessus (rémanence du balayage).
pub fn sweep_gains(fx: &Fx, h: usize, frame: u64) -> Vec<u32> {
    let mut g = vec![256u32; h];
    if fx.sweep == 0 || h == 0 {
        return g;
    }
    let period = (f32::from(fx.sweep_speed.clamp(5, 100)) / 10.0 * SWEEP_FPS).max(1.0);
    let phase = (frame as f32 % period) / period;
    let band = (f32::from(fx.sweep_width.clamp(2, 50)) / 100.0 * h as f32).max(2.0);
    // la barre entre par le haut et sort par le bas : parcours h + bande
    let center = phase * (h as f32 + band) - band / 2.0;
    let k = f32::from(fx.sweep.min(100)) / 100.0;
    for (y, gy) in g.iter_mut().enumerate() {
        let d = (y as f32 + 0.5 - center) / (band / 2.0);
        let v = if d.abs() < 1.0 {
            // éclaircissement jusqu'à +60 % au centre pour une intensité de 100 %
            1.0 + 0.6 * k * 0.5 * (1.0 + (d * std::f32::consts::PI).cos())
        } else if d < -1.0 && d > -3.0 {
            // traînée au-dessus : jusqu'à −15 %
            1.0 - 0.15 * k * 0.5 * (1.0 + ((d + 2.0) * std::f32::consts::PI).cos())
        } else {
            1.0
        };
        *gy = (v * 256.0).round() as u32;
    }
    g
}

/// Effets de tube sur l'image finale : convergence, gains des canaux, barre de balayage.
fn tube(out: &mut [Color32], w: usize, h: usize, fx: &Fx, frame: u64) {
    let conv = (fx.conv_rx, fx.conv_ry, fx.conv_bx, fx.conv_by) != (0, 0, 0, 0);
    let src = if conv { out.to_vec() } else { Vec::new() };
    let sample = |x: usize, y: usize, dx: i8, dy: i8, ch: usize| -> u8 {
        let sx = x as i32 - i32::from(dx.clamp(-8, 8));
        let sy = y as i32 - i32::from(dy.clamp(-8, 8));
        if sx < 0 || sy < 0 || sx >= w as i32 || sy >= h as i32 {
            0
        } else {
            let p = src[sy as usize * w + sx as usize];
            [p.r(), p.g(), p.b()][ch]
        }
    };
    let gain = |pct: u8| (u32::from(pct.min(200)) * 256 + 50) / 100;
    let (gr, gg, gb) = (gain(fx.red), gain(fx.green), gain(fx.blue));
    let sweep = sweep_gains(fx, h, frame);
    for y in 0..h {
        let sg = sweep[y];
        for x in 0..w {
            let p = out[y * w + x];
            let (r, g, b) = if conv {
                (
                    sample(x, y, fx.conv_rx, fx.conv_ry, 0),
                    p.g(),
                    sample(x, y, fx.conv_bx, fx.conv_by, 2),
                )
            } else {
                (p.r(), p.g(), p.b())
            };
            let f = |v: u8, cg: u32| ((u32::from(v) * cg * sg + (1 << 15)) >> 16).min(255) as u8;
            out[y * w + x] = Color32::from_rgb(f(r, gr), f(g, gg), f(b, gb));
        }
    }
}

#[cfg(test)]
mod tests {

    #[test]
    fn tube_rgb_gains() {
        let fx = Fx {
            red: 200,
            green: 50,
            blue: 0,
            ..Fx::default()
        };
        assert!(fx.is_active() && fx.tube_active());
        let o = render(&[0x0040_8080; 16], 4, 4, &fx, 4, 4, 0);
        assert_eq!(o[0], Color32::from_rgb(0x80, 0x40, 0));
        // saturation à 255
        let o = render(&[0x00F0_0000; 16], 4, 4, &fx, 4, 4, 0);
        assert_eq!(o[5].r(), 255);
    }

    #[test]
    fn tube_convergence_shifts_red_and_blue() {
        // un seul pixel blanc en (2, 2) sur fond noir
        let mut src = vec![0u32; 25];
        src[12] = 0x00FF_FFFF;
        let fx = Fx {
            conv_rx: 1,
            conv_by: -2,
            ..Fx::default()
        };
        let o = render(&src, 5, 5, &fx, 5, 5, 0);
        assert_eq!(o[12], Color32::from_rgb(0, 255, 0), "vert seul à sa place");
        assert_eq!(o[2 * 5 + 3].r(), 255, "rouge 1 px à droite");
        assert_eq!(o[2].b(), 255, "bleu 2 px plus haut (ligne 0)");
        assert_eq!(o.iter().filter(|p| **p != Color32::BLACK).count(), 3);
    }

    #[test]
    fn tube_sweep_moves_down() {
        let fx = Fx {
            sweep: 100,
            sweep_speed: 10,
            sweep_width: 20,
            ..Fx::default()
        };
        // période = 1 s = 60 trames ; à la moitié la barre est au milieu de l'image
        let peak = |frame| {
            let g = sweep_gains(&fx, 100, frame);
            (0..100).max_by_key(|&y| g[y]).unwrap()
        };
        let (a, b) = (peak(15), peak(30));
        assert!(a < b && (45..=55).contains(&b), "{a} {b}");
        let g = sweep_gains(&fx, 100, 30);
        assert!(g[b] > 256 + 128, "{}", g[b]);
        assert_eq!(g[99], 256, "hors de la barre : neutre");
        assert!(sweep_gains(&Fx::default(), 10, 0).iter().all(|&v| v == 256));
    }

    #[test]
    fn presets_ignore_tube() {
        let t = Fx {
            red: 120,
            conv_rx: 1,
            sweep: 40,
            ..Fx::preset(Preset::SonyTrinitron)
        };
        assert_eq!(t.detect(), Some(Preset::SonyTrinitron));
        let f = Fx::preset(Preset::FadedTube).with_tube_of(&t);
        assert_eq!((f.red, f.conv_rx, f.sweep), (120, 1, 40));
        assert_eq!(f.detect(), Some(Preset::FadedTube));
        assert!(!t.without_tube().tube_active());
    }
    use super::*;

    fn flat(w: usize, h: usize, v: u32) -> Vec<u32> {
        vec![v; w * h]
    }

    #[test]
    fn inactive_by_default() {
        assert!(!Fx::default().is_active());
        assert!(Fx {
            interp: 10,
            ..Fx::default()
        }
        .is_active());
    }

    #[test]
    fn identity_without_filters() {
        let mut src = flat(320, 224, 0);
        src[5] = 0x102030;
        let o = render(&src, 320, 224, &Fx::default(), 320, 224, 0);
        assert_eq!(o[5], Color32::from_rgb(0x10, 0x20, 0x30));
        assert_eq!(o[6], Color32::from_rgb(0, 0, 0));
    }

    #[test]
    fn data_file_is_complete() {
        assert_eq!(data().len(), (N_SHARP + N_STD) * 256 + 77 * 1024);
        // interpolation, 64 phases : somme 128 ; premières lignes des trois filtres livrés
        for s in (5..=100).step_by(5) {
            for r in sharp_table(s).chunks(4) {
                assert_eq!(r.iter().map(|v| i32::from(*v)).sum::<i32>(), 128);
            }
        }
        assert_eq!(&sharp_table(MEDIUM)[..4], &[2, 124, 2, 0]);
        assert_eq!(&sharp_table(SHARP)[..4], &[0, 128, 0, 0]);
        assert_eq!(&sharp_table(SOFT)[..4], &[10, 108, 10, 0]);
        // scanlines standard 50 % : milieu = 64
        assert_eq!(&std_table(50)[32 * 4..32 * 4 + 4], &[0, 32, 32, 0]);
        // adaptatives 40/80 : milieu sombre 120, clair 204
        let (d, b) = adaptive_tables(40, 80);
        let sum = |t: &[i16]| {
            t[128 * 4..128 * 4 + 4]
                .iter()
                .map(|v| i32::from(*v))
                .sum::<i32>()
        };
        assert_eq!((sum(d), sum(b)), (120, 204));
    }

    #[test]
    fn masks_parse() {
        let t = mask_patterns(Mask::Trinitron).unwrap();
        assert_eq!(
            t.iter().map(|p| (p.res, p.w, p.h)).collect::<Vec<_>>(),
            [(0, 1, 1), (720, 3, 2), (1080, 3, 5), (1440, 3, 6)]
        );
        // 72a : blanc allumé à 112,5 % ; 00c : gris éteint à 75 %
        assert_eq!(t[1].cells[0], [1.0; 3]);
        assert_eq!(t[1].cells[3], [1.125; 3]);
        assert_eq!(t[1].cells[2], [0.75; 3]);
        let f = mask_patterns(Mask::FadedYellow).unwrap();
        assert_eq!(f.len(), 3);
        // 695 : jaune (R, V allumés à 156,25 %, B éteint à 31,25 %)
        assert_eq!(f[1].cells[3], [1.5625, 1.5625, 0.3125]);
    }

    #[test]
    fn interpolation_keeps_flat_colors() {
        let src = flat(320, 224, 0x808080);
        let fx = Fx {
            interp: SOFT,
            ..Fx::default()
        };
        for c in render(&src, 320, 224, &fx, 640, 448, 0) {
            assert_eq!(c, Color32::from_rgb(128, 128, 128));
        }
    }

    #[test]
    fn standard_scanlines_alternate_at_x2() {
        let src = flat(320, 224, 0xFFFFFF);
        let fx = Fx {
            scan_mode: ScanMode::Standard,
            scan: 50,
            ..Fx::default()
        };
        let o = render(&src, 320, 224, &fx, 320, 448, 0);
        assert_eq!(o[0], Color32::from_rgb(255, 255, 255));
        assert_eq!(o[320], Color32::from_rgb(128, 128, 128));
        assert_eq!(o[640], Color32::from_rgb(255, 255, 255));
    }

    #[test]
    fn adaptive_scanlines_depend_on_brightness() {
        let fx = Fx {
            scan_mode: ScanMode::Adaptive,
            scan_dk: 40,
            scan_br: 80,
            ..Fx::default()
        };
        let dark = render(&flat(320, 224, 0x000000), 320, 224, &fx, 320, 448, 0);
        assert_eq!(dark[320], Color32::from_rgb(0, 0, 0));
        // blanc : 255 * 204 / 256 au milieu de la ligne
        let bright = render(&flat(320, 224, 0xFFFFFF), 320, 224, &fx, 320, 448, 0);
        assert_eq!(bright[320].r(), 203);
        let grey = render(&flat(320, 224, 0x808080), 320, 224, &fx, 320, 448, 0);
        assert!(grey[320].r() < 100 && grey[320].r() > 60);
    }

    #[test]
    fn gamma_and_mask() {
        let fx = Fx {
            gamma: 145,
            ..Fx::default()
        };
        let o = render(&flat(320, 224, 0x808080), 320, 224, &fx, 320, 224, 0);
        // 255 * (128/255)^1.45 ~ 94
        assert_eq!(o[0].r(), 94);
        // tube fané à l'échelle x5 (1120 -> motif 1080) : le jaune réduit le bleu
        let fx = Fx {
            mask: Mask::FadedYellow,
            ..Fx::default()
        };
        let o = render(&flat(320, 224, 0x808080), 320, 224, &fx, 1600, 1120, 0);
        assert!(o.iter().any(|c| c.b() < c.r()));
    }

    #[test]
    fn presets_round_trip() {
        for p in Preset::ALL {
            assert_eq!(Fx::preset(p).detect(), Some(p));
        }
        assert_eq!(
            Fx {
                gamma: 123,
                ..Fx::default()
            }
            .detect(),
            None
        );
    }

    #[test]
    fn render_384x224_identity() {
        let src = flat(384, 224, 0x123456);
        let o = render(&src, 384, 224, &Fx::default(), 384, 224, 0);
        assert_eq!(o.len(), 384 * 224);
        for c in &o {
            assert_eq!(*c, Color32::from_rgb(0x12, 0x34, 0x56));
        }
    }

    #[test]
    fn render_384x224_scanlines() {
        let src = flat(384, 224, 0xFFFFFF);
        let fx = Fx {
            scan_mode: ScanMode::Standard,
            scan: 50,
            ..Fx::default()
        };
        let o = render(&src, 384, 224, &fx, 768, 448, 0);
        assert_eq!(o.len(), 768 * 448);
    }

    /// Banc du coût des filtres par trame, à toutes les échelles, ratios et résolutions plein écran :
    /// `cargo test --release -p ng-gui bench_fx -- --ignored --nocapture`
    #[test]
    #[ignore]
    fn bench_fx() {
        use crate::display::layout;
        use crate::prefs::{Aspect, ScaleMode};
        use eframe::egui::{pos2, vec2, Rect};
        use puce_jamma::video::{Rotation, VideoInfo};
        use std::time::Instant;

        let info = VideoInfo {
            width: 320,
            height: 224,
            refresh_hz: 60.0,
            aspect: (4, 3),
            rotation: Rotation::Rot0,
        };

        // image pseudo-aléatoire (pire cas : pas de zones plates)
        let mut seed = 12345u32;
        let src: Vec<u32> = (0..320 * 224)
            .map(|_| {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                seed >> 8
            })
            .collect();
        let worst = Fx {
            interp: SHARP,
            scan_mode: ScanMode::Adaptive,
            mask: Mask::Trinitron,
            gamma: 145,
            ..Fx::default()
        };
        let presets = [
            (
                "Interp seule",
                Fx {
                    interp: MEDIUM,
                    ..Fx::default()
                },
            ),
            (
                "Scanl. std",
                Fx {
                    scan_mode: ScanMode::Standard,
                    ..Fx::default()
                },
            ),
            ("Trinitron", Fx::preset(Preset::SonyTrinitron)),
            ("Faded Tube", Fx::preset(Preset::FadedTube)),
            ("Tout", worst),
        ];
        let mut cases: Vec<(String, usize, usize)> = Vec::new();
        for (an, aspect) in [("1:1", Aspect::Square), ("4:3", Aspect::Tv)] {
            for n in 1..=8 {
                let l = layout(
                    Rect::from_min_size(pos2(0.0, 0.0), vec2(8000.0, 8000.0)),
                    1.0,
                    ScaleMode::Fixed(n),
                    aspect,
                    &info,
                );
                cases.push((format!("{an} x{n}"), l.px.0 as usize, l.px.1 as usize));
            }
            for (rn, rw, rh) in [
                ("720p", 1280.0, 720.0),
                ("1080p", 1920.0, 1080.0),
                ("1440p", 2560.0, 1440.0),
                ("4K", 3840.0, 2160.0),
            ] {
                let l = layout(
                    Rect::from_min_size(pos2(0.0, 0.0), vec2(rw, rh)),
                    1.0,
                    ScaleMode::Auto,
                    aspect,
                    &info,
                );
                cases.push((
                    format!("{an} plein écran {rn}"),
                    l.px.0 as usize,
                    l.px.1 as usize,
                ));
            }
        }
        println!("budget total : 16,89 ms/trame ; temps de filtrage (ms) par trame");
        print!("{:28} {:>11}", "cas", "taille");
        for (n, _) in &presets {
            print!(" {n:>13}");
        }
        println!();
        for (name, w, h) in cases {
            print!("{name:28} {:>11}", format!("{w}x{h}"));
            for (_, fx) in &presets {
                let _ = render(&src, 320, 224, fx, w, h, 0); // chauffe + tables
                let n = 8;
                let t0 = Instant::now();
                for _ in 0..n {
                    std::hint::black_box(render(&src, 320, 224, fx, w, h, 0));
                }
                print!(
                    " {:13.2}",
                    t0.elapsed().as_secs_f64() * 1000.0 / f64::from(n)
                );
            }
            println!();
        }
    }
}
