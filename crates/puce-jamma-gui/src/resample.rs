//! Rééchantillonneur audio : filtre sinc fenêtré (Kaiser) polyphasé, déterministe. Entrée : fréquence
//! quelconque, 1 ou 2 voies (mono dupliqué en stéréo, Q-STEREO) ; sortie : stéréo entrelacé à la
//! fréquence du périphérique.
//!
//! Remplace l'interpolation linéaire (v0.6) : à partir d'une source à 24 kHz, elle laissait passer les
//! images spectrales au-dessus de 12 kHz (+6 à +17 dB au-dessus de la référence entre 16 et 24 kHz),
//! perçues comme un grésillement. Le filtre coupe à 90 % de la plus petite des deux fréquences de Nyquist.

/// Demi-longueur du noyau, en échantillons source (32 coefficients au total).
const HALF_TAPS: usize = 16;
const TAPS: usize = 2 * HALF_TAPS;
/// Nombre de phases tabulées par intervalle source (interpolation linéaire entre deux phases).
const PHASES: usize = 512;
/// Bande passante : fraction de la plus petite fréquence de Nyquist.
const PASSBAND: f64 = 0.90;
/// Paramètre β de la fenêtre de Kaiser (≈ 80 dB de réjection).
const KAISER_BETA: f64 = 8.0;

/// Fonction de Bessel modifiée I0 (série), pour la fenêtre de Kaiser.
fn bessel_i0(x: f64) -> f64 {
    let (mut sum, mut term, mut k) = (1.0, 1.0, 1.0);
    while term > 1e-12 * sum {
        term *= (x / (2.0 * k)) * (x / (2.0 * k));
        sum += term;
        k += 1.0;
    }
    sum
}

/// Table des coefficients : `PHASES + 1` lignes de `TAPS` valeurs, chaque ligne de somme 1 (gain
/// unité en continu). Ligne `p` : coefficient du tap `k` pour une position fractionnaire `p / PHASES`,
/// le tap `k` pesant l'échantillon d'indice `k - HALF_TAPS + 1` relativement à l'échantillon de base.
fn kernel_table(cutoff: f64) -> Vec<f32> {
    let norm = bessel_i0(KAISER_BETA);
    let mut table = Vec::with_capacity((PHASES + 1) * TAPS);
    for p in 0..=PHASES {
        let frac = p as f64 / PHASES as f64;
        let row: Vec<f64> = (0..TAPS)
            .map(|k| {
                let x = k as f64 - (HALF_TAPS as f64 - 1.0) - frac;
                let sinc = if x == 0.0 {
                    1.0
                } else {
                    let a = std::f64::consts::PI * cutoff * x;
                    a.sin() / a
                };
                let w = x / HALF_TAPS as f64;
                let win = if w.abs() >= 1.0 {
                    0.0
                } else {
                    bessel_i0(KAISER_BETA * (1.0 - w * w).sqrt()) / norm
                };
                sinc * win
            })
            .collect();
        let sum: f64 = row.iter().sum();
        table.extend(row.iter().map(|c| (c / sum) as f32));
    }
    table
}

/// Rééchantillonneur audio à filtre sinc fenêtré.
///
/// Déterministe : pas d'accès au temps système. État fractionnaire en f64.
pub struct AudioResampler {
    src_hz: f64,
    dst_hz: u32,
    table: Vec<f32>,
    /// Derniers échantillons source (fenêtre glissante de `TAPS` trames).
    hist: [[f32; 2]; TAPS],
    /// Échantillons source reçus (plafonné à `TAPS` : sert à l'amorçage).
    filled: usize,
    /// Position de la prochaine sortie, en échantillons source, relative à l'échantillon de base
    /// `hist[HALF_TAPS - 1]` (0 <= pos < 1 quand une sortie est due).
    pos: f64,
    /// Tampon de sortie interne (stéréo entrelacé).
    output_buf: Vec<f32>,
}

impl AudioResampler {
    pub fn new(src_hz: f64, dst_hz: u32) -> Self {
        let cutoff = PASSBAND * (f64::from(dst_hz) / src_hz).min(1.0);
        Self {
            src_hz,
            dst_hz,
            table: kernel_table(cutoff),
            hist: [[0.0; 2]; TAPS],
            filled: 0,
            pos: 0.0,
            output_buf: Vec::new(),
        }
    }

    /// Change la fréquence source (régulation du débit audio) sans toucher à la phase ni au filtre
    /// (les écarts de la régulation restent sous 0,5 %).
    pub fn set_src_hz(&mut self, hz: f64) {
        self.src_hz = hz;
    }

    /// Pousse un échantillon stéréo source vers le rééchantillonneur.
    pub fn push(&mut self, s: [f32; 2]) {
        self.hist.copy_within(1.., 0);
        self.hist[TAPS - 1] = s;
        if self.filled < TAPS {
            self.filled += 1;
            if self.filled < TAPS {
                return; // amorçage : la fenêtre n'est pas encore pleine
            }
        } else {
            self.pos -= 1.0; // l'échantillon de base avance d'un pas source
        }
        // Pas de sortie, en échantillons source.
        let step = self.src_hz / f64::from(self.dst_hz);
        while self.pos < 1.0 {
            self.emit(self.pos.max(0.0));
            self.pos += step;
        }
    }

    /// Une sortie à la position fractionnaire `frac` (0 <= frac < 1) après l'échantillon de base.
    fn emit(&mut self, frac: f64) {
        let fp = frac * PHASES as f64;
        let p = (fp as usize).min(PHASES - 1);
        let t = (fp - p as f64) as f32;
        let a = &self.table[p * TAPS..(p + 1) * TAPS];
        let b = &self.table[(p + 1) * TAPS..(p + 2) * TAPS];
        let (mut l, mut r) = (0.0f32, 0.0f32);
        for k in 0..TAPS {
            let c = a[k] + t * (b[k] - a[k]);
            l += c * self.hist[k][0];
            r += c * self.hist[k][1];
        }
        self.output_buf.push(l);
        self.output_buf.push(r);
    }

    /// Pop des échantillons stéréo entrelacés vers out.
    pub fn pop_into(&mut self, out: &mut Vec<f32>) {
        out.append(&mut self.output_buf);
    }

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
#[path = "resample_tests.rs"]
mod tests;
