//! Bezels au format RetroArch (overlay plein écran) : `<jeu>.cfg` (ou celui du parent, ou
//! `_generic.cfg`) désigne une image PNG dont le centre transparent reçoit l'image du jeu.
//! Affichés en plein écran seulement, si l'option est activée.
//!
//! Format lu (sous-ensemble des overlays RetroArch) :
//! ```text
//! overlays = 1
//! overlay0_overlay = sf2.png
//! overlay0_full_screen = true
//! overlay0_descs = 0
//! ```

use std::path::{Path, PathBuf};

use eframe::egui;

/// Image de bezel décodée (RVBA non prémultiplié) et rectangle transparent qui reçoit le jeu,
/// en pixels de l'image : (x0, y0, x1, y1), x1 et y1 exclus.
#[derive(Clone, Debug)]
pub struct Bezel {
    pub w: usize,
    pub h: usize,
    pub rgba: Vec<u8>,
    pub hole: (usize, usize, usize, usize),
}

/// Fichier .cfg du jeu : `<jeu>.cfg`, sinon celui du parent, sinon `_generic.cfg`.
pub fn find_cfg(dir: &Path, game: &str, parent: &str) -> Option<PathBuf> {
    [game, parent, "_generic"]
        .iter()
        .filter(|n| !n.is_empty())
        .map(|n| dir.join(format!("{n}.cfg")))
        .find(|p| p.is_file())
}

/// Image de la première couche (`overlay0_overlay`), chemin relatif au .cfg.
pub fn parse_cfg(text: &str) -> Option<String> {
    text.lines().find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == "overlay0_overlay").then(|| v.trim().trim_matches('"').to_string())
    })
}

/// Seuil d'alpha sous lequel un pixel appartient à l'écran : les bezels RetroArch y posent souvent
/// une vitre semi-transparente (reflets, alpha d'environ 36), dessinée par-dessus le jeu.
const HOLE_ALPHA: u8 = 128;

/// Rectangle transparent autour du centre de l'image : on part du centre et on s'étend tant que
/// les pixels de la ligne et de la colonne centrales restent transparents.
pub fn find_hole(w: usize, h: usize, rgba: &[u8]) -> Option<(usize, usize, usize, usize)> {
    let alpha = |x: usize, y: usize| rgba[(y * w + x) * 4 + 3];
    let (cx, cy) = (w / 2, h / 2);
    if w == 0 || h == 0 || alpha(cx, cy) >= HOLE_ALPHA {
        return None;
    }
    let mut x0 = cx;
    while x0 > 0 && alpha(x0 - 1, cy) < HOLE_ALPHA {
        x0 -= 1;
    }
    let mut x1 = cx + 1;
    while x1 < w && alpha(x1, cy) < HOLE_ALPHA {
        x1 += 1;
    }
    let mut y0 = cy;
    while y0 > 0 && alpha(cx, y0 - 1) < HOLE_ALPHA {
        y0 -= 1;
    }
    let mut y1 = cy + 1;
    while y1 < h && alpha(cx, y1) < HOLE_ALPHA {
        y1 += 1;
    }
    Some((x0, y0, x1, y1))
}

/// Charge le bezel décrit par `cfg`.
pub fn load(cfg: &Path) -> Result<Bezel, String> {
    let text = std::fs::read_to_string(cfg).map_err(|e| format!("{} : {e}", cfg.display()))?;
    let name =
        parse_cfg(&text).ok_or_else(|| format!("{} : overlay0_overlay absent", cfg.display()))?;
    let png = cfg.parent().unwrap_or(Path::new(".")).join(name);
    let file = std::fs::File::open(&png).map_err(|e| format!("{} : {e}", png.display()))?;
    let mut dec = png::Decoder::new(std::io::BufReader::new(file));
    dec.set_transformations(
        png::Transformations::EXPAND | png::Transformations::STRIP_16 | png::Transformations::ALPHA,
    );
    let mut reader = dec
        .read_info()
        .map_err(|e| format!("{} : {e}", png.display()))?;
    let mut buf = vec![0; reader.output_buffer_size().ok_or("PNG trop grand")?];
    let info = reader
        .next_frame(&mut buf)
        .map_err(|e| format!("{} : {e}", png.display()))?;
    let (w, h) = (info.width as usize, info.height as usize);
    let rgba = match info.color_type {
        png::ColorType::Rgba => buf[..w * h * 4].to_vec(),
        png::ColorType::GrayscaleAlpha => buf[..w * h * 2]
            .chunks(2)
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        c => {
            return Err(format!(
                "{} : format {c:?} non pris en charge",
                png.display()
            ))
        }
    };
    let hole = find_hole(w, h, &rgba)
        .ok_or_else(|| format!("{} : pas de zone transparente au centre", png.display()))?;
    Ok(Bezel { w, h, rgba, hole })
}

/// Dimensions d'un bezel chargé (l'image elle-même part dans une texture).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub w: usize,
    pub h: usize,
    pub hole: (usize, usize, usize, usize),
}

impl Bezel {
    pub fn geometry(&self) -> Geometry {
        Geometry {
            w: self.w,
            h: self.h,
            hole: self.hole,
        }
    }
}

/// Placement à l'écran : image du bezel ajustée dans `screen` (proportions gardées, centrée) et
/// rectangle du trou correspondant.
pub fn layout(screen: egui::Rect, b: &Geometry) -> (egui::Rect, egui::Rect) {
    let s = (screen.width() / b.w as f32).min(screen.height() / b.h as f32);
    let size = egui::vec2(b.w as f32 * s, b.h as f32 * s);
    let rect = egui::Rect::from_center_size(screen.center(), size);
    let (x0, y0, x1, y1) = b.hole;
    let p = |x: usize, y: usize| rect.min + egui::vec2(x as f32 * s, y as f32 * s);
    (rect, egui::Rect::from_min_max(p(x0, y0), p(x1, y1)))
}

/// Écart de proportions toléré pour remplir exactement l'écran du bezel (courbure du tube).
const FILL_TOLERANCE: f32 = 0.05;

/// Image du jeu dans l'écran du bezel : l'écran entier si ses proportions sont à moins de 5 % de
/// `aspect` (largeur, hauteur), sinon la plus grande taille aux proportions `aspect`, centrée.
pub fn fit(hole: egui::Rect, aspect: (u32, u32)) -> egui::Rect {
    let a = aspect.0.max(1) as f32 / aspect.1.max(1) as f32;
    if ((hole.width() / hole.height()) / a - 1.0).abs() < FILL_TOLERANCE {
        return hole;
    }
    let (w, h) = if hole.width() / hole.height() > a {
        (hole.height() * a, hole.height())
    } else {
        (hole.width(), hole.width() / a)
    };
    egui::Rect::from_center_size(hole.center(), egui::vec2(w, h))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Image 40 × 20 opaque avec un écran en (10..30, 5..15) : vitre semi-transparente (alpha 36).
    fn frame() -> Vec<u8> {
        let (w, h) = (40, 20);
        let mut v = vec![255u8; w * h * 4];
        for y in 5..15 {
            for x in 10..30 {
                v[(y * w + x) * 4 + 3] = 36;
            }
        }
        v
    }

    #[test]
    fn game_fits_in_hole() {
        let hole = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(200.0, 100.0));
        let r = fit(hole, (4, 3));
        assert!(
            (r.width() - 400.0 / 3.0).abs() < 1e-3 && r.height() == 100.0,
            "{r:?}"
        );
        assert_eq!(r.center(), hole.center());
        assert_eq!(fit(hole, (3, 4)).size(), egui::vec2(75.0, 100.0));
        // écran de bezel 2165 × 1584 (rapport 1,367) : rempli par une image 4:3
        let crt = egui::Rect::from_min_size(egui::pos2(833.0, 210.0), egui::vec2(2165.0, 1584.0));
        assert_eq!(fit(crt, (4, 3)), crt);
    }

    #[test]
    fn hole_detection() {
        assert_eq!(find_hole(40, 20, &frame()), Some((10, 5, 30, 15)));
        assert_eq!(
            find_hole(4, 4, &[255; 64]),
            None,
            "centre opaque : pas de bezel"
        );
    }

    #[test]
    fn cfg_parsing_and_lookup() {
        let t = "overlays = 1\n\noverlay0_overlay = \"sf2.png\"\n\noverlay0_full_screen = true\n";
        assert_eq!(parse_cfg(t).as_deref(), Some("sf2.png"));
        assert_eq!(parse_cfg("overlays = 1"), None);
        let dir = std::env::temp_dir().join(format!("bezel_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("_generic.cfg"), "").unwrap();
        std::fs::write(dir.join("sf2.cfg"), "").unwrap();
        assert_eq!(find_cfg(&dir, "sf2", ""), Some(dir.join("sf2.cfg")));
        assert_eq!(
            find_cfg(&dir, "sf2ua", "sf2"),
            Some(dir.join("sf2.cfg")),
            "parent"
        );
        assert_eq!(find_cfg(&dir, "ghouls", ""), Some(dir.join("_generic.cfg")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_png_and_layout() {
        let dir = std::env::temp_dir().join(format!("bezel_png_{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let f = std::fs::File::create(dir.join("t.png")).unwrap();
        let mut e = png::Encoder::new(std::io::BufWriter::new(f), 40, 20);
        e.set_color(png::ColorType::Rgba);
        e.write_header()
            .unwrap()
            .write_image_data(&frame())
            .unwrap();
        std::fs::write(
            dir.join("t.cfg"),
            "overlays = 1\noverlay0_overlay = t.png\n",
        )
        .unwrap();
        let b = load(&dir.join("t.cfg")).unwrap();
        assert_eq!((b.w, b.h, b.hole), (40, 20, (10, 5, 30, 15)));
        // écran 80 × 80 : bezel ×2 centré verticalement (80 × 40 à y = 20)
        let (r, hole) = layout(
            egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(80.0, 80.0)),
            &b.geometry(),
        );
        assert_eq!(
            r,
            egui::Rect::from_min_max(egui::pos2(0.0, 20.0), egui::pos2(80.0, 60.0))
        );
        assert_eq!(
            hole,
            egui::Rect::from_min_max(egui::pos2(20.0, 30.0), egui::pos2(60.0, 50.0))
        );
        assert!(load(&dir.join("absent.cfg")).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
