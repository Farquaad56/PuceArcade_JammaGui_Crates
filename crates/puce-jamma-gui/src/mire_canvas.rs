//! Petit moteur de dessin des mires : canevas 0x00RRGGBB, rectangles, cercles, police 3 × 5.

/// Gris de niveau `v` (0 à 255).
pub const fn gray(v: u32) -> u32 {
    v << 16 | v << 8 | v
}

pub const WHITE: u32 = 0xFF_FFFF;

/// Image en cours de dessin.
pub struct Canvas {
    pub w: usize,
    pub h: usize,
    pub px: Vec<u32>,
}

impl Canvas {
    pub fn new(w: usize, h: usize, bg: u32) -> Self {
        Canvas {
            w,
            h,
            px: vec![bg; w * h],
        }
    }

    pub fn set(&mut self, x: i64, y: i64, c: u32) {
        if x >= 0 && y >= 0 && (x as usize) < self.w && (y as usize) < self.h {
            self.px[y as usize * self.w + x as usize] = c;
        }
    }

    pub fn rect(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, c: u32) {
        for y in y0.min(self.h)..y1.min(self.h) {
            self.px[y * self.w + x0.min(self.w)..y * self.w + x1.min(self.w)].fill(c);
        }
    }

    /// Contour d'un rectangle (bords inclus), épaisseur 1.
    pub fn frame(&mut self, x0: usize, y0: usize, x1: usize, y1: usize, c: u32) {
        self.rect(x0, y0, x1 + 1, y0 + 1, c);
        self.rect(x0, y1, x1 + 1, y1 + 1, c);
        self.rect(x0, y0, x0 + 1, y1 + 1, c);
        self.rect(x1, y0, x1 + 1, y1 + 1, c);
    }

    pub fn circle(&mut self, cx: f32, cy: f32, r: f32, c: u32) {
        let n = (r * 8.0).max(16.0) as usize;
        for i in 0..n {
            let a = i as f32 / n as f32 * std::f32::consts::TAU;
            self.set(
                (cx + r * a.cos()).round() as i64,
                (cy + r * a.sin()).round() as i64,
                c,
            );
        }
    }

    /// Texte en police 3 × 5 (chiffres, « % », « . », « - »), agrandi `s` fois.
    pub fn text(&mut self, x: usize, y: usize, s: usize, t: &str, c: u32) {
        for (i, ch) in t.chars().enumerate() {
            let g = glyph(ch);
            for (row, bits) in g.iter().enumerate() {
                for col in 0..3 {
                    if bits & (4 >> col) != 0 {
                        let (px, py) = (x + (i * 4 + col) * s, y + row * s);
                        self.rect(px, py, px + s, py + s, c);
                    }
                }
            }
        }
    }
}

/// Police 3 × 5 : une rangée = 3 bits (gauche = 4).
fn glyph(c: char) -> [u8; 5] {
    match c {
        '0' => [7, 5, 5, 5, 7],
        '1' => [2, 6, 2, 2, 7],
        '2' => [7, 1, 7, 4, 7],
        '3' => [7, 1, 3, 1, 7],
        '4' => [5, 5, 7, 1, 1],
        '5' => [7, 4, 7, 1, 7],
        '6' => [7, 4, 7, 5, 7],
        '7' => [7, 1, 2, 2, 2],
        '8' => [7, 5, 7, 5, 7],
        '9' => [7, 5, 7, 1, 7],
        '%' => [5, 1, 2, 4, 5],
        '.' => [0, 0, 0, 0, 2],
        '-' => [0, 0, 7, 0, 0],
        _ => [0; 5],
    }
}

/// Taille de police adaptée à la hauteur de l'image.
pub fn font(h: usize) -> usize {
    (h / 100).max(1)
}
