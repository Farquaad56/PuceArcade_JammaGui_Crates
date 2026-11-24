//! Onglet « Son » du débogueur : un oscilloscope par voie (`AudioVoice` fournies par la carte),
//! regroupées par puce, en grille. Générique : les voies, leur nombre, leur nom, leur fréquence
//! d'échantillonnage et le détail affiché viennent entièrement de la carte.
//!
//! Clic sur un oscilloscope : coupe / réactive la voie dans le mixage ; clic droit : solo.

use eframe::egui;
use puce_jamma::debug::AudioVoice;

use crate::debugger::DebugAction;

/// Voies prises en compte par le masque de muets.
pub const MAX_VOICES: usize = 64;

/// Début de la fenêtre affichée : dernier front montant (passage par zéro, avec hystérésis) qui
/// laisse `window` échantillons après lui, cherché sur au plus `window` échantillons en arrière ;
/// à défaut, les `window` derniers échantillons. Stabilise l'image d'un signal périodique.
pub fn trigger_start(samples: &[f32], window: usize) -> usize {
    let len = samples.len();
    if window >= len {
        return 0;
    }
    let latest = len - window;
    let earliest = latest.saturating_sub(window);
    let hyst = peak(samples) * 0.05;
    let mut i = latest;
    while i > earliest {
        let (a, b) = (samples[i - 1], samples[i]);
        if a < 0.0 && b >= 0.0 {
            // hystérésis : le signal est réellement descendu avant ce front
            let low = samples[i.saturating_sub(16)..i].iter().any(|s| *s <= -hyst);
            if low {
                return i;
            }
        }
        i -= 1;
    }
    latest
}

/// Amplitude crête (valeur absolue maximale).
pub fn peak(samples: &[f32]) -> f32 {
    samples.iter().fold(0.0f32, |m, s| m.max(s.abs()))
}

/// Gain d'affichage : 1 en échelle fixe ; en échelle auto, crête ramenée à 90 % de la hauteur,
/// sans amplifier au-delà de 64 fois (un souffle presque nul reste presque plat).
pub fn display_gain(samples: &[f32], auto: bool) -> f32 {
    if !auto {
        return 1.0;
    }
    let p = peak(samples);
    if p <= 0.0 {
        1.0
    } else {
        (0.9 / p).min(64.0)
    }
}

/// Valeurs à tracer, une ou deux par colonne de pixels : tous les échantillons s'ils tiennent,
/// sinon min puis max de chaque colonne (l'enveloppe d'un bruit reste visible). Chaque point est
/// (x en [0, 1], y dans [-1, 1] avant gain).
pub fn scope_points(samples: &[f32], columns: usize) -> Vec<(f32, f32)> {
    let n = samples.len();
    if n == 0 || columns == 0 {
        return Vec::new();
    }
    if n <= columns * 2 {
        let last = (n - 1).max(1) as f32;
        return samples
            .iter()
            .enumerate()
            .map(|(i, s)| (i as f32 / last, *s))
            .collect();
    }
    let mut pts = Vec::with_capacity(columns * 2);
    for c in 0..columns {
        let a = c * n / columns;
        let b = ((c + 1) * n / columns).max(a + 1);
        let chunk = &samples[a..b];
        let lo = chunk.iter().copied().fold(f32::INFINITY, f32::min);
        let hi = chunk.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let x = c as f32 / (columns - 1).max(1) as f32;
        // ordre min/max alterné : la ligne descend et remonte sans croiser la colonne voisine
        if c % 2 == 0 {
            pts.push((x, lo));
            pts.push((x, hi));
        } else {
            pts.push((x, hi));
            pts.push((x, lo));
        }
    }
    pts
}

/// Masque où seule la voie `i` joue parmi `n` voies.
pub fn solo_mask(i: usize, n: usize) -> u64 {
    let n = n.min(MAX_VOICES);
    let all = if n >= 64 { u64::MAX } else { (1u64 << n) - 1 };
    all & !(1u64 << i)
}

pub struct AudioDebug {
    voices: Vec<AudioVoice>,
    /// Durée affichée, en millisecondes.
    window_ms: f32,
    auto_scale: bool,
    columns: usize,
    mute: u64,
}

impl Default for AudioDebug {
    fn default() -> Self {
        AudioDebug {
            voices: Vec::new(),
            window_ms: 20.0,
            auto_scale: true,
            columns: 3,
            mute: 0,
        }
    }
}

impl AudioDebug {
    pub fn set_voices(&mut self, v: Vec<AudioVoice>) {
        self.voices = v;
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> Option<DebugAction> {
        let mut action = None;
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::Slider::new(&mut self.window_ms, 2.0..=100.0)
                    .logarithmic(true)
                    .suffix(" ms")
                    .text("Fenêtre"),
            );
            ui.checkbox(&mut self.auto_scale, "Échelle auto");
            ui.add(egui::Slider::new(&mut self.columns, 1..=4).text("Colonnes"));
            if self.mute != 0 && ui.button("Tout réactiver").clicked() {
                self.mute = 0;
                action = Some(DebugAction::VoiceMute(0));
            }
        });
        ui.weak("Clic : couper / réactiver une voie · clic droit : solo");
        ui.separator();
        if self.voices.is_empty() {
            ui.weak("Cette carte ne fournit pas de voies sonores.");
            return action;
        }
        let n = self.voices.len().min(MAX_VOICES);
        egui::ScrollArea::vertical().show(ui, |ui| {
            let mut i = 0;
            while i < n {
                let chip = self.voices[i].chip.clone();
                let end = (i..n).find(|&j| self.voices[j].chip != chip).unwrap_or(n);
                let rate = self.voices[i].sample_rate;
                ui.strong(format!("{chip}  ·  {} voies  ·  {rate} Hz", end - i));
                if let Some(a) = self.group_ui(ui, i, end, n) {
                    action = Some(a);
                }
                ui.add_space(6.0);
                i = end;
            }
        });
        action
    }

    /// Voies `start..end` en grille ; renvoie le nouveau masque si un clic l'a changé.
    fn group_ui(
        &mut self,
        ui: &mut egui::Ui,
        start: usize,
        end: usize,
        n: usize,
    ) -> Option<DebugAction> {
        let cols = self.columns.max(1);
        let gap = 2.0;
        let width = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).max(60.0);
        let size = egui::vec2(width, (width * 0.62).clamp(56.0, 140.0));
        let mut action = None;
        for row_start in (start..end).step_by(cols) {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for i in row_start..(row_start + cols).min(end) {
                    let resp = self.scope_cell(ui, i, size);
                    if resp.clicked() {
                        self.mute ^= 1u64 << i;
                        action = Some(DebugAction::VoiceMute(self.mute));
                    } else if resp.secondary_clicked() {
                        let solo = solo_mask(i, n);
                        self.mute = if self.mute == solo { 0 } else { solo };
                        action = Some(DebugAction::VoiceMute(self.mute));
                    }
                }
            });
            ui.add_space(gap);
        }
        action
    }

    fn scope_cell(&self, ui: &mut egui::Ui, i: usize, size: egui::Vec2) -> egui::Response {
        let v = &self.voices[i];
        let muted = self.mute & (1u64 << i) != 0;
        let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
        let p = ui.painter_at(rect);
        p.rect_filled(rect, 0.0, egui::Color32::from_gray(18));
        p.rect_stroke(
            rect,
            0.0,
            egui::Stroke::new(1.0, egui::Color32::from_gray(60)),
            egui::StrokeKind::Inside,
        );
        let text = egui::Color32::from_gray(if v.active { 220 } else { 130 });
        let font = egui::FontId::proportional(12.0);
        p.text(
            rect.left_top() + egui::vec2(4.0, 3.0),
            egui::Align2::LEFT_TOP,
            &v.name,
            font.clone(),
            text,
        );
        if muted {
            p.text(
                rect.right_top() + egui::vec2(-4.0, 3.0),
                egui::Align2::RIGHT_TOP,
                "muet",
                font.clone(),
                egui::Color32::from_rgb(230, 90, 70),
            );
        }
        if !v.detail.is_empty() {
            p.text(
                rect.left_bottom() + egui::vec2(4.0, -3.0),
                egui::Align2::LEFT_BOTTOM,
                &v.detail,
                egui::FontId::proportional(10.0),
                egui::Color32::from_gray(120),
            );
        }

        // trace : fenêtre demandée, déclenchée sur un front montant
        let window = ((v.sample_rate as f32 * self.window_ms / 1000.0) as usize)
            .clamp(2, v.samples.len().max(2));
        let start = trigger_start(&v.samples, window);
        let shown = &v.samples[start..(start + window).min(v.samples.len())];
        let gain = display_gain(shown, self.auto_scale);
        let plot = rect.shrink2(egui::vec2(3.0, 18.0));
        let mid = plot.center().y;
        let half = plot.height() / 2.0;
        let color = if muted {
            egui::Color32::from_gray(80)
        } else if v.active {
            egui::Color32::from_gray(235)
        } else {
            egui::Color32::from_gray(150)
        };
        let pts: Vec<egui::Pos2> = scope_points(shown, plot.width().max(1.0) as usize)
            .into_iter()
            .map(|(x, y)| {
                let y = (y * gain).clamp(-1.0, 1.0);
                egui::pos2(plot.left() + x * plot.width(), mid - y * half)
            })
            .collect();
        if pts.len() >= 2 {
            p.add(egui::Shape::line(pts, egui::Stroke::new(1.0, color)));
        } else {
            p.hline(plot.x_range(), mid, egui::Stroke::new(1.0, color));
        }
        resp.on_hover_text(format!("{} · {}", v.chip, v.name))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(n: usize, period: f32, phase: f32) -> Vec<f32> {
        (0..n)
            .map(|i| ((i as f32 + phase) / period * std::f32::consts::TAU).sin())
            .collect()
    }

    #[test]
    fn trigger_lands_on_rising_zero_crossing() {
        let s = sine(1000, 50.0, 13.0);
        let t = trigger_start(&s, 200);
        assert!(t + 200 <= s.len());
        assert!(s[t - 1] < 0.0 && s[t] >= 0.0, "front montant en {t}");
        // le même signal décalé donne la même image à une période près
        let t2 = trigger_start(&s[7..], 200);
        let a: Vec<f32> = s[t..t + 50].to_vec();
        let b: Vec<f32> = s[7 + t2..7 + t2 + 50].to_vec();
        for (x, y) in a.iter().zip(&b) {
            assert!((x - y).abs() < 0.15);
        }
    }

    #[test]
    fn trigger_falls_back_to_latest_window() {
        let flat = vec![0.0f32; 500];
        assert_eq!(trigger_start(&flat, 100), 400);
        assert_eq!(trigger_start(&flat, 600), 0);
        // bruit de quantification sans vraie descente : pas de déclenchement
        let tiny: Vec<f32> = (0..500)
            .map(|i| if i % 2 == 0 { -1e-6 } else { 1e-6 })
            .collect();
        let mut s = tiny;
        s[0] = 1.0;
        assert_eq!(trigger_start(&s, 100), 400);
    }

    #[test]
    fn gain_and_peak() {
        assert_eq!(peak(&[0.1, -0.5, 0.3]), 0.5);
        assert_eq!(display_gain(&[0.1, -0.5], false), 1.0);
        assert!((display_gain(&[0.1, -0.5], true) - 1.8).abs() < 1e-6);
        assert_eq!(display_gain(&[0.0; 4], true), 1.0);
        assert_eq!(display_gain(&[1e-5], true), 64.0);
    }

    #[test]
    fn points_keep_all_samples_or_envelope() {
        let p = scope_points(&[0.0, 1.0, -1.0], 10);
        assert_eq!(p, vec![(0.0, 0.0), (0.5, 1.0), (1.0, -1.0)]);
        let noise: Vec<f32> = (0..1000)
            .map(|i| if i % 3 == 0 { 0.8 } else { -0.6 })
            .collect();
        let p = scope_points(&noise, 100);
        assert_eq!(p.len(), 200);
        assert!(p.iter().any(|(_, y)| *y == 0.8) && p.iter().any(|(_, y)| *y == -0.6));
        assert_eq!(p.first().unwrap().0, 0.0);
        assert_eq!(p.last().unwrap().0, 1.0);
        assert!(scope_points(&[], 10).is_empty());
    }

    #[test]
    fn solo() {
        assert_eq!(solo_mask(0, 3), 0b110);
        assert_eq!(solo_mask(2, 3), 0b011);
        assert_eq!(solo_mask(1, 64), !2);
    }
}
