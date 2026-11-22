//! Rééchantillonneur audio (repris de ref/puce16-neogeo/audio_mix.rs, `AudioResampler`) :
//! interpolation linéaire, déterministe. Entrée : fréquence quelconque, 1 ou 2 voies (mono dupliqué
//! en stéréo, Q-STEREO) ; sortie : stéréo entrelacé à la fréquence du périphérique.

/// Rééchantillonneur audio à interpolation linéaire.
///
/// Déterministe : pas d'accès au temps système. État fractionnaire en f64.
pub struct AudioResampler {
    src_hz: f64,
    dst_hz: u32,
    /// Phase de sortie (en unités d'échantillon de sortie).
    phase: f64,
    /// Échantillon précédent pour interpolation linéaire.
    prev: [f32; 2],
    /// Indique si un échantillon précédent est disponible.
    has_prev: bool,
    /// Tampon de sortie interne (stéréo entrelacé).
    output_buf: Vec<f32>,
}

impl AudioResampler {
    pub fn new(src_hz: f64, dst_hz: u32) -> Self {
        Self {
            src_hz,
            dst_hz,
            phase: 0.0,
            prev: [0.0, 0.0],
            has_prev: false,
            output_buf: Vec::new(),
        }
    }

    /// Change la fréquence source (régulation du débit audio) sans toucher à la phase.
    pub fn set_src_hz(&mut self, hz: f64) {
        self.src_hz = hz;
    }

    /// Pousse un échantillon stéréo source vers le rééchantillonneur.
    pub fn push(&mut self, s: [f32; 2]) {
        if !self.has_prev {
            // Premier échantillon : pas d'interpolation possible.
            self.prev = s;
            self.has_prev = true;
            return;
        }

        // Ratio de rééchantillonnage : combien d'échantillons de sortie par échantillon source.
        let ratio = self.dst_hz as f64 / self.src_hz;

        // Avancer la phase d'un intervalle source (en unités de sortie).
        self.phase += ratio;

        while self.phase >= 1.0 {
            // Interpolation linéaire entre prev et s à la position (phase - 1).
            let t = (self.phase - 1.0) as f32;
            let out_l = self.prev[0] + t * (s[0] - self.prev[0]);
            let out_r = self.prev[1] + t * (s[1] - self.prev[1]);
            self.output_buf.push(out_l);
            self.output_buf.push(out_r);
            self.phase -= 1.0;
        }

        self.prev = s;
    }

    /// Pop des échantillons stéréo entrelacés vers out.
    pub fn pop_into(&mut self, out: &mut Vec<f32>) {
        out.append(&mut self.output_buf);
    }
}

impl AudioResampler {
    /// Pousse des échantillons entrelacés selon `channels` (1 = mono dupliqué, 2 = stéréo ;
    /// au-delà, seules les deux premières voies sont gardées).
    pub fn push_interleaved(&mut self, samples: &[f32], channels: u8) {
        match channels {
            0 => {}
            1 => {
                for &s in samples {
                    self.push([s, s]);
                }
            }
            n => {
                for f in samples.chunks_exact(n as usize) {
                    self.push([f[0], f[1]]);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn src_rate_change_scales_the_output_count() {
        let run = |hz: f64| {
            let mut r = AudioResampler::new(hz, 48000);
            for _ in 0..55_556 {
                r.push([0.5, 0.5]);
            }
            let mut v = Vec::new();
            r.pop_into(&mut v);
            v.len() / 2
        };
        let base = run(55_555.56);
        assert!((47_990..=48_010).contains(&base), "{base}");
        // une source déclarée plus lente produit plus d'échantillons de sortie
        let mut r = AudioResampler::new(55_555.56, 48000);
        r.set_src_hz(55_555.56 * 0.995);
        for _ in 0..55_556 {
            r.push([0.5, 0.5]);
        }
        let mut v = Vec::new();
        r.pop_into(&mut v);
        assert!(v.len() / 2 > base + 200);
    }

    #[test]
    fn same_rate_is_identity_with_one_sample_delay() {
        let mut r = AudioResampler::new(48_000.0, 48_000);
        let input: Vec<f32> = (0..1000).map(|i| i as f32 / 1000.0).collect();
        r.push_interleaved(&input, 1);
        let mut v = Vec::new();
        r.pop_into(&mut v);
        // l'interpolation sort l'échantillon précédent : sortie = entrée sans le dernier échantillon
        let left: Vec<f32> = v.iter().step_by(2).copied().collect();
        assert_eq!(left, input[..999].to_vec());
    }

    #[test]
    fn cps1_rate_ramp_to_48k() {
        let mut r = AudioResampler::new(55_930.4, 48_000);
        let ramp: Vec<f32> = (0..55_930).map(|i| i as f32 / 55_930.0).collect();
        r.push_interleaved(&ramp, 1);
        let mut v = Vec::new();
        r.pop_into(&mut v);
        let left: Vec<f32> = v.iter().step_by(2).copied().collect();
        // 55 929 intervalles × 48 000 / 55 930,4 = 47 998,8 (le premier échantillon n'est qu'amorcé) :
        // 48 000 ± 1 attendu par la carte, moins ce délai d'un échantillon.
        assert!((47_998..=48_001).contains(&left.len()), "{}", left.len());
        assert!(left.windows(2).all(|w| w[1] >= w[0]), "sortie non monotone");
    }

    #[test]
    fn mono_is_duplicated_to_stereo() {
        let mut r = AudioResampler::new(48_000.0, 48_000);
        r.push_interleaved(&[0.5, 0.5], 1);
        let mut v = Vec::new();
        r.pop_into(&mut v);
        assert_eq!(v, vec![0.5, 0.5]);
    }

    #[test]
    fn stereo_keeps_channels() {
        let mut r = AudioResampler::new(48_000.0, 48_000);
        r.push_interleaved(&[0.1, -0.2, 0.3, -0.4], 2);
        let mut v = Vec::new();
        r.pop_into(&mut v);
        assert_eq!(v, vec![0.1, -0.2]);
    }
}
