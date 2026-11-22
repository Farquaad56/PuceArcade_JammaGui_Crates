//! Placement de l'image dans la fenêtre : facteur entier, pixels nets, image centrée sur un pixel physique entier.

use eframe::egui::{pos2, vec2, Rect};

use crate::prefs::{Aspect, ScaleMode};
use puce_jamma::video::{Rotation, VideoInfo};

/// Résultat de `layout`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplayRect {
    /// Rectangle de l'image, en points egui.
    pub rect: Rect,
    /// Facteur d'agrandissement entier.
    pub factor: u32,
    /// Taille de l'image en pixels physiques.
    pub px: (u32, u32),
}

/// Taille logique (pixels) de l'image au facteur 1, APRÈS rotation.
pub fn display_size(info: &VideoInfo, aspect: Aspect) -> (u32, u32) {
    let (w, h) = match info.rotation {
        Rotation::Rot0 | Rotation::Rot180 => (info.width, info.height),
        Rotation::Rot90 | Rotation::Rot270 => (info.height, info.width),
    };
    match aspect {
        Aspect::Square => (w, h),
        Aspect::Tv => {
            // Largeur conservée, hauteur recalculée selon le rapport d'aspect du moniteur.
            let tv_h =
                ((w as f64) * (info.aspect.1 as f64) / (info.aspect.0 as f64)).round() as u32;
            (w, tv_h)
        }
    }
}

/// Place l'image dans `avail` (points egui) ; `ppp` = pixels physiques par point.
pub fn layout(
    avail: Rect,
    ppp: f32,
    mode: ScaleMode,
    aspect: Aspect,
    info: &VideoInfo,
) -> DisplayRect {
    let ppp = if ppp > 0.0 { ppp } else { 1.0 };
    let to_px = |v: f32| (v * ppp + 0.001).floor() as i64;
    let (aw, ah) = (to_px(avail.width()).max(0), to_px(avail.height()).max(0));
    let (bw, bh) = display_size(info, aspect);
    let factor = match mode {
        ScaleMode::Auto => ((aw / i64::from(bw)).min(ah / i64::from(bh))).max(1) as u32,
        ScaleMode::Fixed(n) => n.max(1),
    };
    let (w, h) = (bw * factor, bh * factor);
    let x = (avail.left() * ppp).round() as i64 + (aw - i64::from(w)).div_euclid(2);
    let y = (avail.top() * ppp).round() as i64 + (ah - i64::from(h)).div_euclid(2);
    let min = pos2(x as f32 / ppp, y as f32 / ppp);
    DisplayRect {
        rect: Rect::from_min_size(min, vec2(w as f32 / ppp, h as f32 / ppp)),
        factor,
        px: (w, h),
    }
}

/// Texte court de l'échelle courante : « x3 (960x672) ».
pub fn scale_label(d: &DisplayRect) -> String {
    format!("x{} ({}x{})", d.factor, d.px.0, d.px.1)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn zone(w: f32, h: f32) -> Rect {
        Rect::from_min_size(pos2(0.0, 0.0), vec2(w, h))
    }

    fn info_320x224() -> VideoInfo {
        VideoInfo {
            width: 320,
            height: 224,
            refresh_hz: 60.0,
            aspect: (4, 3),
            rotation: Rotation::Rot0,
        }
    }

    #[test]
    fn auto_picks_the_largest_integer_factor() {
        let info = info_320x224();
        let d = layout(
            zone(1920.0, 1080.0),
            1.0,
            ScaleMode::Auto,
            Aspect::Square,
            &info,
        );
        assert_eq!((d.factor, d.px), (4, (1280, 896)));
        assert_eq!(d.rect.min, pos2(320.0, 92.0));
        let d = layout(
            zone(1280.0, 720.0),
            1.0,
            ScaleMode::Auto,
            Aspect::Square,
            &info,
        );
        assert_eq!((d.factor, d.px), (3, (960, 672)));
        assert_eq!(scale_label(&d), "x3 (960x672)");
    }

    #[test]
    fn hidpi_counts_physical_pixels() {
        let info = info_320x224();
        let d = layout(
            zone(960.0, 540.0),
            2.0,
            ScaleMode::Auto,
            Aspect::Square,
            &info,
        );
        assert_eq!((d.factor, d.px), (4, (1280, 896)));
        assert_eq!(d.rect.size(), vec2(640.0, 448.0));
    }

    #[test]
    fn too_small_zone_keeps_factor_one() {
        let info = info_320x224();
        let d = layout(
            zone(200.0, 100.0),
            1.0,
            ScaleMode::Auto,
            Aspect::Square,
            &info,
        );
        assert_eq!((d.factor, d.px), (1, (320, 224)));
        let d = layout(zone(0.0, 0.0), 0.0, ScaleMode::Auto, Aspect::Square, &info);
        assert_eq!(d.factor, 1);
    }

    #[test]
    fn position_is_on_a_physical_pixel() {
        let info = info_320x224();
        for ppp in [1.0f32, 1.25, 1.5, 2.0] {
            let avail = Rect::from_min_size(pos2(0.0, 23.2), vec2(1001.3, 707.7));
            let d = layout(avail, ppp, ScaleMode::Auto, Aspect::Square, &info);
            for v in [d.rect.left(), d.rect.top(), d.rect.right(), d.rect.bottom()] {
                let px = v * ppp;
                assert!((px - px.round()).abs() < 1e-3, "{v} à ppp={ppp}");
            }
        }
    }

    #[test]
    fn fixed_factor_is_respected_even_when_cropped() {
        let info = info_320x224();
        let d = layout(
            zone(640.0, 480.0),
            1.0,
            ScaleMode::Fixed(5),
            Aspect::Square,
            &info,
        );
        assert_eq!((d.factor, d.px), (5, (1600, 1120)));
        assert_eq!(d.rect.min, pos2(-480.0, -320.0));
        let d = layout(
            zone(640.0, 480.0),
            1.0,
            ScaleMode::Fixed(0),
            Aspect::Square,
            &info,
        );
        assert_eq!(d.factor, 1);
    }

    #[test]
    fn tv_aspect_is_320x240_per_factor() {
        let info = info_320x224();
        let d = layout(zone(1280.0, 720.0), 1.0, ScaleMode::Auto, Aspect::Tv, &info);
        assert_eq!((d.factor, d.px), (3, (960, 720)));
    }

    // Nouveaux tests : dimensions non-Neo Geo et rotation

    #[test]
    fn display_size_384x224_square_and_tv() {
        let info = VideoInfo {
            width: 384,
            height: 224,
            refresh_hz: 60.0,
            aspect: (4, 3),
            rotation: Rotation::Rot0,
        };
        assert_eq!(display_size(&info, Aspect::Square), (384, 224));
        assert_eq!(display_size(&info, Aspect::Tv), (384, 288)); // 384 * 3/4 = 288
    }

    #[test]
    fn display_size_vertical_rotated() {
        let info = VideoInfo {
            width: 288,
            height: 224,
            refresh_hz: 60.0,
            aspect: (3, 4),
            rotation: Rotation::Rot270,
        };
        // Rot270 : w/h échangés -> 224x288
        assert_eq!(display_size(&info, Aspect::Square), (224, 288));
        // Tv : largeur conservée (224), hauteur = 224 * 4/3 = 298.67 -> 299
        assert_eq!(display_size(&info, Aspect::Tv), (224, 299));
    }

    #[test]
    fn layout_uses_display_size() {
        let info = VideoInfo {
            width: 384,
            height: 224,
            refresh_hz: 60.0,
            aspect: (4, 3),
            rotation: Rotation::Rot0,
        };
        // Tv : base 384x288 -> facteur 1 dans 640x480
        let d = layout(zone(640.0, 480.0), 1.0, ScaleMode::Auto, Aspect::Tv, &info);
        assert_eq!((d.factor, d.px), (1, (384, 288)));
    }
}
