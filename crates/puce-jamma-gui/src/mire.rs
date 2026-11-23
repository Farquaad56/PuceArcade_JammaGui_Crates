//! Mires de réglage de l'écran et des filtres, générées à la définition de l'image du jeu
//! (après rotation) : géométrie, convergence, overscan, barres de couleur, pureté, contraste,
//! luminosité, dégradés, gamma, netteté, mouvement. Fonctions pures, pixels 0x00RRGGBB.

/// Mires disponibles.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Mire {
    #[default]
    Geometry,
    Convergence,
    Overscan,
    ColorBars,
    Purity,
    Contrast,
    Brightness,
    Ramps,
    Gamma,
    Sharpness,
    Motion,
}

impl Mire {
    pub const ALL: [Mire; 11] = [
        Mire::Geometry,
        Mire::Convergence,
        Mire::Overscan,
        Mire::ColorBars,
        Mire::Purity,
        Mire::Contrast,
        Mire::Brightness,
        Mire::Ramps,
        Mire::Gamma,
        Mire::Sharpness,
        Mire::Motion,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Mire::Geometry => "Géométrie (grille et cercles)",
            Mire::Convergence => "Convergence",
            Mire::Overscan => "Overscan",
            Mire::ColorBars => "Barres de couleur",
            Mire::Purity => "Pureté (plein écran)",
            Mire::Contrast => "Contraste",
            Mire::Brightness => "Luminosité (noirs)",
            Mire::Ramps => "Dégradés RVB",
            Mire::Gamma => "Gamma",
            Mire::Sharpness => "Netteté",
            Mire::Motion => "Mouvement",
        }
    }

    /// Variantes proposées (au moins une).
    pub fn variants(self) -> &'static [&'static str] {
        match self {
            Mire::Geometry => &["Blanc sur noir", "Gris sur noir"],
            Mire::Convergence => &["Points", "Croix", "Grille fine"],
            Mire::ColorBars => &["75 %", "100 %"],
            Mire::Purity => &["Rouge", "Vert", "Bleu", "Blanc", "Gris 50 %", "Noir"],
            Mire::Ramps => &["Continus", "16 paliers"],
            Mire::Motion => &["Lent", "Rapide"],
            _ => &["Standard"],
        }
    }

    /// Ce que la mire permet de régler.
    pub fn help(self) -> &'static str {
        match self {
            Mire::Geometry => {
                "Les cases doivent être carrées et de même taille, les cercles ronds, les lignes \
                 droites. Règle la courbure, le format d'image et le vignettage."
            }
            Mire::Convergence => {
                "Points et lignes blancs, sans franges rouges ou bleues. Règle la convergence \
                 (onglet Tube) : 0 px pour un tube parfait, 1 px pour un tube fatigué."
            }
            Mire::Overscan => {
                "Chaque cadre indique la part coupée (0 à 10 %). Le cadre visible le plus \
                 extérieur donne l'overscan actuel ; un vrai moniteur cachait 3 à 6 %."
            }
            Mire::ColorBars => {
                "Blanc, jaune, cyan, vert, magenta, rouge, bleu : couleurs franches et \
                 lumineuses. Règle la température, les canaux RVB et la force du masque."
            }
            Mire::Purity => {
                "Un aplat doit être uniforme sur tout l'écran. Règle les taches de pureté, le \
                 vignettage, et vérifie le masque et les scanlines."
            }
            Mire::Contrast => {
                "Les onze paliers du haut doivent tous se distinguer, et les six blancs du bas \
                 (90 à 100 %) ne doivent pas se confondre. Règle gamma, halo et masque."
            }
            Mire::Brightness => {
                "Sur le noir, la barre à 2 % doit être à peine visible et celle à 5 % nette. \
                 Règle le niveau du noir et le gamma (et la luminosité de ton écran)."
            }
            Mire::Ramps => {
                "Dégradés sans cassure ni dominante. Règle la température du blanc, les canaux \
                 RVB et le gamma."
            }
            Mire::Gamma => {
                "Recule-toi : le carré dont la teinte se fond dans la trame noir et blanc \
                 indique le gamma de la chaîne (1.8, 2.2 ou 2.5)."
            }
            Mire::Sharpness => {
                "Rayures de 1 à 4 pixels : règle l'interpolation, l'agrandissement pixel art, \
                 la taille des luminophores (moiré) et le faisceau."
            }
            Mire::Motion => {
                "Barre et carré en mouvement : vérifie les images noires (netteté du mouvement), \
                 l'instabilité et la barre de balayage."
            }
        }
    }

    /// La mire change-t-elle d'une trame à l'autre ?
    pub fn animated(self) -> bool {
        self == Mire::Motion
    }
}

use crate::mire_canvas::{font, gray, Canvas, WHITE};

/// Dessine la mire `m` (variante `v`) en `w` × `h` ; `frame` anime la mire de mouvement.
pub fn render(m: Mire, v: usize, w: usize, h: usize, frame: u64) -> Vec<u32> {
    let (w, h) = (w.max(16), h.max(16));
    let mut c = Canvas::new(w, h, 0);
    match m {
        Mire::Geometry => geometry(&mut c, if v == 1 { gray(128) } else { WHITE }),
        Mire::Convergence => convergence(&mut c, v),
        Mire::Overscan => overscan(&mut c),
        Mire::ColorBars => color_bars(&mut c, v == 1),
        Mire::Purity => {
            let col = [0xFF_0000, 0x00_FF00, 0x00_00FF, WHITE, gray(128), 0];
            c.px.fill(col[v.min(5)]);
        }
        Mire::Contrast => contrast(&mut c),
        Mire::Brightness => brightness(&mut c),
        Mire::Ramps => ramps(&mut c, v == 1),
        Mire::Gamma => gamma(&mut c),
        Mire::Sharpness => sharpness(&mut c),
        Mire::Motion => motion(&mut c, frame, if v == 1 { 8 } else { 2 }),
    }
    c.px
}

/// Positions de `n` + 1 lignes régulièrement espacées de 0 à `len` − 1.
fn ticks(len: usize, n: usize) -> Vec<usize> {
    (0..=n).map(|i| (i * (len - 1) + n / 2) / n).collect()
}

/// Nombre de cases : 16 dans la grande dimension, cases au plus près du carré.
fn cells(w: usize, h: usize) -> (usize, usize) {
    if w >= h {
        (16, ((16 * h + w / 2) / w).max(2))
    } else {
        (((16 * w + h / 2) / h).max(2), 16)
    }
}

fn geometry(c: &mut Canvas, col: u32) {
    let (w, h) = (c.w, c.h);
    let (nx, ny) = cells(w, h);
    for x in ticks(w, nx) {
        c.rect(x, 0, x + 1, h, col);
    }
    for y in ticks(h, ny) {
        c.rect(0, y, w, y + 1, col);
    }
    let (cx, cy) = (w / 2, h / 2);
    c.rect(cx - 1, 0, cx + 2, h, col);
    c.rect(0, cy - 1, w, cy + 2, col);
    let m = w.min(h) as f32;
    c.circle(w as f32 / 2.0, h as f32 / 2.0, m * 0.45, col);
    let r = m * 0.12;
    for (x, y) in [
        (r, r),
        (w as f32 - r, r),
        (r, h as f32 - r),
        (w as f32 - r, h as f32 - r),
    ] {
        c.circle(x, y, r * 0.9, col);
    }
}

fn convergence(c: &mut Canvas, v: usize) {
    let (w, h) = (c.w, c.h);
    let (nx, ny) = cells(w, h);
    match v {
        0 | 1 => {
            for &y in &ticks(h, ny) {
                for &x in &ticks(w, nx) {
                    let (x, y) = (x as i64, y as i64);
                    c.set(x, y, WHITE);
                    if v == 1 {
                        for d in 1..=3 {
                            for (dx, dy) in [(d, 0), (-d, 0), (0, d), (0, -d)] {
                                c.set(x + dx, y + dy, WHITE);
                            }
                        }
                    }
                }
            }
        }
        _ => {
            for x in (0..w).step_by(8) {
                c.rect(x, 0, x + 1, h, WHITE);
            }
            for y in (0..h).step_by(8) {
                c.rect(0, y, w, y + 1, WHITE);
            }
        }
    }
}

fn overscan(c: &mut Canvas) {
    let (w, h) = (c.w, c.h);
    let colors = [
        0xFF_0000, 0xFF_FF00, 0x00_FF00, 0x00_FFFF, 0x40_60FF, 0xFF_00FF,
    ];
    let s = font(h);
    for (i, col) in colors.into_iter().enumerate() {
        // cadre à 2·i % (moitié de chaque côté)
        let (dx, dy) = (w * i / 100, h * i / 100);
        c.frame(dx, dy, w - 1 - dx, h - 1 - dy, col);
        // légende au centre : carré de la couleur du cadre et part coupée
        let (lx, ly) = (w / 2 - 10 * s, h / 2 - 21 * s + i * 7 * s);
        c.rect(lx, ly, lx + 5 * s, ly + 5 * s, col);
        c.text(lx + 8 * s, ly, s, &format!("{}%", 2 * i), col);
    }
    // graduations au milieu des bords : un trait par 2 % (1 % de chaque côté)
    for i in 0..=5 {
        let (dx, dy) = (w * i / 100, h * i / 100);
        let len = if i % 2 == 0 { 4 * s } else { 2 * s };
        c.rect(dx, h / 2 - len, dx + 1, h / 2 + len, WHITE);
        c.rect(w - 1 - dx, h / 2 - len, w - dx, h / 2 + len, WHITE);
        c.rect(w / 2 - len, dy, w / 2 + len, dy + 1, WHITE);
        c.rect(w / 2 - len, h - 1 - dy, w / 2 + len, h - dy, WHITE);
    }
}

/// Bandes verticales égales : `cols[i]` sur [y0, y1).
fn bands(c: &mut Canvas, y0: usize, y1: usize, cols: &[u32]) {
    let n = cols.len();
    for (i, col) in cols.iter().enumerate() {
        c.rect(i * c.w / n, y0, (i + 1) * c.w / n, y1, *col);
    }
}

fn color_bars(c: &mut Canvas, full: bool) {
    let (w, h) = (c.w, c.h);
    let l = if full { 255 } else { 191 };
    let rgb = |r: u32, g: u32, b: u32| ((r * l) << 16) | ((g * l) << 8) | (b * l);
    let main = [
        rgb(1, 1, 1),
        rgb(1, 1, 0),
        rgb(0, 1, 1),
        rgb(0, 1, 0),
        rgb(1, 0, 1),
        rgb(1, 0, 0),
        rgb(0, 0, 1),
    ];
    let (y1, y2) = (h * 2 / 3, h * 3 / 4);
    bands(c, 0, y1, &main);
    let rev = [main[6], 0, main[4], 0, main[2], 0, main[0]];
    bands(c, y1, y2, &rev);
    // bas : bleu foncé, blanc 100 %, violet, noir, puis noir 0 %, 4 % et 8 % (réglage du noir)
    let low = [0x00_214C, WHITE, 0x32_006A, 0, 0, gray(10), gray(20)];
    bands(c, y2, h, &low);
    let _ = w;
}

fn contrast(c: &mut Canvas) {
    let (w, h) = (c.w, c.h);
    let steps: Vec<u32> = (0..=10).map(|i| gray((i * 255 + 5) / 10)).collect();
    bands(c, 0, h / 2, &steps);
    let s = font(h);
    for i in 0..=10 {
        let x = i * w / 11 + 2 * s;
        let col = if i < 5 { WHITE } else { 0 };
        c.text(x, h / 2 - 7 * s, s, &format!("{}", i * 10), col);
    }
    let whites: Vec<u32> = (0..6).map(|i| gray((230 + i * 5).min(255))).collect();
    bands(c, h / 2, h, &whites);
    for i in 0..6 {
        c.text(
            i * w / 6 + 2 * s,
            h - 7 * s,
            s,
            &format!("{}", 90 + 2 * i),
            0,
        );
    }
}

fn brightness(c: &mut Canvas) {
    let (w, h) = (c.w, c.h);
    let s = font(h);
    let levels = [(2u32, "2%"), (5, "5%"), (10, "10%")];
    for (i, (pct, label)) in levels.into_iter().enumerate() {
        let x0 = w * (2 + 3 * i) / 12;
        let x1 = x0 + w / 6;
        c.rect(x0, h / 6, x1, h * 2 / 3, gray(pct * 255 / 100));
        c.text(x0, h * 2 / 3 + 3 * s, s, label, gray(128));
    }
    // références : blanc et gris moyen
    c.rect(w * 2 / 5, h * 5 / 6, w * 3 / 5, h * 5 / 6 + h / 12, WHITE);
    c.rect(w / 5, h * 5 / 6, w * 2 / 5, h * 5 / 6 + h / 12, gray(128));
}

fn ramps(c: &mut Canvas, stepped: bool) {
    let (w, h) = (c.w, c.h);
    let masks = [WHITE, 0xFF_0000, 0x00_FF00, 0x00_00FF];
    for (b, mask) in masks.into_iter().enumerate() {
        for x in 0..w {
            let mut v = (x * 255 / (w - 1)) as u32;
            if stepped {
                v = (v / 16) * 17;
            }
            let col = gray(v) & mask;
            c.rect(x, b * h / 4, x + 1, (b + 1) * h / 4, col);
        }
    }
}

fn gamma(c: &mut Canvas) {
    let (w, h) = (c.w, c.h);
    // trame : damier noir et blanc d'un pixel (50 % de lumière)
    for y in 0..h {
        for x in 0..w {
            c.px[y * w + x] = if (x + y) % 2 == 0 { WHITE } else { 0 };
        }
    }
    let s = font(h);
    for (i, g) in [1.8f32, 2.2, 2.5].into_iter().enumerate() {
        let v = (255.0 * 0.5f32.powf(1.0 / g)).round() as u32;
        let x0 = w * (1 + 3 * i) / 10 + w / 40;
        let (bw, by) = (w / 5, h / 3);
        c.rect(x0, by, x0 + bw, by + h / 3, gray(v));
        c.rect(x0, by + h / 3, x0 + bw, by + h / 3 + 9 * s, 0);
        c.text(x0 + 2 * s, by + h / 3 + 2 * s, s, &format!("{g:.1}"), WHITE);
    }
}

fn sharpness(c: &mut Canvas) {
    let (w, h) = (c.w, c.h);
    c.px.fill(gray(128));
    let s = font(h);
    for (i, p) in [1usize, 2, 3, 4].into_iter().enumerate() {
        let (x0, x1) = (i * w / 4 + w / 32, (i + 1) * w / 4 - w / 32);
        // rayures verticales en haut, horizontales en bas, de période 2p
        for x in x0..x1 {
            let col = if (x - x0) / p % 2 == 0 { WHITE } else { 0 };
            c.rect(x, h / 10, x + 1, h * 9 / 20, col);
        }
        for y in h * 11 / 20..h * 9 / 10 {
            let col = if (y - h * 11 / 20) / p % 2 == 0 {
                WHITE
            } else {
                0
            };
            c.rect(x0, y, x1, y + 1, col);
        }
        c.text(x0, h / 10 - 7 * s, s, &format!("{p}"), WHITE);
    }
}

fn motion(c: &mut Canvas, frame: u64, speed: usize) {
    let (w, h) = (c.w, c.h);
    c.px.fill(gray(64));
    let bar = w / 24;
    let x = (frame as usize * speed) % (w + bar);
    c.rect(x.saturating_sub(bar), 0, x.min(w), h / 2, WHITE);
    // carré qui rebondit en bas
    let side = h / 6;
    let span = w - side;
    let t = (frame as usize * speed) % (2 * span);
    let sx = if t < span { t } else { 2 * span - t };
    let sy = h * 2 / 3;
    c.rect(sx, sy, sx + side, sy + side, 0xFF_FF00);
    // graduations fixes pour juger le flou
    for gx in (0..w).step_by(w / 16) {
        c.rect(gx, h / 2, gx + 1, h / 2 + h / 12, WHITE);
    }
}

#[cfg(test)]
#[path = "mire_tests.rs"]
mod tests;
