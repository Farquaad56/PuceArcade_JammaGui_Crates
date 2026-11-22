//! Sortie audio (cpal) : tampon circulaire stéréo entrelacé, alimenté par l'interface.
//! Reprise de ref/puce16-neogeo/ng-gui/audio.rs ; ajout : format natif du périphérique (f32, i16, u16)
//! et sortie mono (moyenne des deux voies).

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SizedSample};

pub type SharedBuf = Arc<Mutex<VecDeque<f32>>>;

/// Remplit `out` depuis `deque` ; complète par du silence si le tampon est vide.
pub fn fill_buffer(deque: &mut VecDeque<f32>, out: &mut [f32]) {
    for o in out.iter_mut() {
        *o = deque.pop_front().unwrap_or(0.0);
    }
}

/// Ajoute des échantillons en plafonnant le tampon à `cap` valeurs (les plus anciennes sont jetées).
pub fn push_capped(deque: &mut VecDeque<f32>, samples: &[f32], cap: usize) {
    deque.extend(samples.iter().copied());
    while deque.len() > cap {
        deque.pop_front();
    }
}

/// Latence visée du tampon audio, en ms : de quoi absorber les irrégularités d'ordonnancement.
pub const TARGET_MS: usize = 60;

/// Ajustement relatif de la fréquence source pour ramener le tampon vers sa cible : `fill` est le
/// remplissage en fraction de la cible (1 = cible). Tampon trop vide -> source déclarée plus lente
/// (plus d'échantillons produits), trop plein -> plus rapide ; borné à ±0,5 % (inaudible).
pub fn drc_adjust(fill: f32) -> f64 {
    (f64::from(fill - 1.0) * 0.01).clamp(-0.005, 0.005)
}

/// Gain linéaire appliqué au signal pour un volume de 0 à 100 % (courbe quadratique, plus proche
/// de la perception) ; 0 si le son est coupé.
pub fn volume_gain(volume: u8, mute: bool) -> f32 {
    if mute {
        return 0.0;
    }
    let v = f32::from(volume.min(100)) / 100.0;
    v * v
}

/// État partagé avec le rappel audio.
#[derive(Clone)]
struct Shared {
    buf: SharedBuf,
    underruns: Arc<AtomicU64>,
    active: Arc<AtomicBool>,
    primed: Arc<AtomicBool>,
    target: usize,
}

/// Recopie des échantillons stéréo de `q` vers `data` (entrelacé sur `channels` voies) au format `T`.
fn fill_frames<T: SizedSample + FromSample<f32>>(q: &mut VecDeque<f32>, data: &mut [T], channels: usize) {
    for frame in data.chunks_mut(channels) {
        let l = q.pop_front().unwrap_or(0.0);
        let r = q.pop_front().unwrap_or(0.0);
        if channels == 1 {
            frame[0] = T::from_sample((l + r) * 0.5);
            continue;
        }
        frame[0] = T::from_sample(l);
        frame[1] = T::from_sample(r);
        for c in frame.iter_mut().skip(2) {
            *c = T::from_sample(0.0f32);
        }
    }
}

/// Flux de sortie au format `T` : n'émet qu'une fois la latence visée accumulée.
fn build<T: SizedSample + FromSample<f32>>(
    device: &cpal::Device,
    config: cpal::StreamConfig,
    st: Shared,
) -> Result<cpal::Stream, String> {
    let channels = config.channels as usize;
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _| {
                let mut q = st.buf.lock().unwrap_or_else(|e| e.into_inner());
                let active = st.active.load(Ordering::Relaxed);
                if !st.primed.load(Ordering::Relaxed) {
                    if active && q.len() >= st.target {
                        st.primed.store(true, Ordering::Relaxed);
                    } else {
                        data.fill(T::from_sample(0.0f32)); // accumulation de la latence, ou pause
                        return;
                    }
                }
                if q.len() < data.len() / channels * 2 && active {
                    st.underruns.fetch_add(1, Ordering::Relaxed);
                    // on rejoue ce qui reste puis on reconstitue le coussin
                    st.primed.store(false, Ordering::Relaxed);
                }
                fill_frames(&mut q, data, channels);
            },
            |e| eprintln!("erreur audio : {e}"),
            None,
        )
        .map_err(|e| format!("ouverture du flux audio : {e}"))
}

pub struct AudioOut {
    _stream: cpal::Stream,
    shared: SharedBuf,
    pub rate: u32,
    /// Sous-alimentations du périphérique tant que l'émulation tourne.
    underruns: Arc<AtomicU64>,
    /// Faux à l'arrêt et en pause : le silence n'est alors pas une sous-alimentation.
    active: Arc<AtomicBool>,
    /// Vrai une fois la latence visée accumulée ; repasse à faux après un manque ou en pause.
    primed: Arc<AtomicBool>,
    /// Latence visée, en valeurs entrelacées.
    target: usize,
}

impl AudioOut {
    /// Ouvre le périphérique de sortie par défaut, dans son format natif.
    pub fn new() -> Result<AudioOut, String> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("aucun périphérique audio de sortie")?;
        let config = device
            .default_output_config()
            .map_err(|e| format!("configuration audio : {e}"))?;
        let rate = config.sample_rate();
        let format = config.sample_format();
        let st = Shared {
            buf: Arc::new(Mutex::new(VecDeque::new())),
            underruns: Arc::new(AtomicU64::new(0)),
            active: Arc::new(AtomicBool::new(false)),
            primed: Arc::new(AtomicBool::new(false)),
            target: rate as usize * 2 * TARGET_MS / 1000,
        };
        let stream = match format {
            cpal::SampleFormat::F32 => build::<f32>(&device, config.into(), st.clone()),
            cpal::SampleFormat::I16 => build::<i16>(&device, config.into(), st.clone()),
            cpal::SampleFormat::U16 => build::<u16>(&device, config.into(), st.clone()),
            f => return Err(format!("format audio {f:?} non pris en charge")),
        }?;
        stream
            .play()
            .map_err(|e| format!("démarrage du flux audio : {e}"))?;
        Ok(AudioOut {
            _stream: stream,
            shared: st.buf,
            rate,
            underruns: st.underruns,
            active: st.active,
            primed: st.primed,
            target: st.target,
        })
    }

    /// Remplissage du tampon en fraction de la latence visée (1 = cible).
    pub fn fill(&self) -> f32 {
        let q = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        q.len() as f32 / self.target as f32
    }

    pub fn underruns(&self) -> u64 {
        self.underruns.load(Ordering::Relaxed)
    }

    /// Indique si l'émulation produit du son (sinon le silence n'est pas compté).
    pub fn set_active(&self, on: bool) {
        self.active.store(on, Ordering::Relaxed);
        if !on {
            self.primed.store(false, Ordering::Relaxed);
        }
    }

    /// Ajoute des échantillons stéréo entrelacés (plafond : 0,25 s).
    pub fn push(&self, samples: &[f32]) {
        let cap = self.rate as usize / 2;
        let mut q = self.shared.lock().unwrap_or_else(|e| e.into_inner());
        push_capped(&mut q, samples, cap);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rate_control_is_bounded_and_signed() {
        assert_eq!(drc_adjust(1.0), 0.0);
        // tampon vide : source plus lente (négatif) ; plein : plus rapide ; toujours dans ±0,5 %
        assert_eq!(drc_adjust(0.0), -0.005);
        assert_eq!(drc_adjust(5.0), 0.005);
        assert!(drc_adjust(0.8) < 0.0 && drc_adjust(1.2) > 0.0);
        assert!(drc_adjust(0.8).abs() < 0.005);
    }

    #[test]
    fn volume_curve() {
        assert_eq!(volume_gain(100, false), 1.0);
        assert_eq!(volume_gain(0, false), 0.0);
        assert_eq!(volume_gain(100, true), 0.0);
        assert!((volume_gain(50, false) - 0.25).abs() < 1e-6);
        assert_eq!(volume_gain(200, false), 1.0);
    }

    #[test]
    fn underrun_is_filled_with_silence() {
        let mut d: VecDeque<f32> = VecDeque::from(vec![0.1, 0.2]);
        let mut buf = [9.0f32; 4];
        fill_buffer(&mut d, &mut buf);
        assert_eq!(buf, [0.1, 0.2, 0.0, 0.0]);
    }

    #[test]
    fn buffer_is_capped_to_the_latest_samples() {
        let rate = 48_000usize;
        let mut d = VecDeque::new();
        let all: Vec<f32> = (0..rate).map(|i| i as f32).collect();
        push_capped(&mut d, &all, rate / 2);
        assert_eq!(d.len(), rate / 2);
        assert_eq!(d.front().copied(), Some((rate / 2) as f32));
        assert_eq!(d.back().copied(), Some((rate - 1) as f32));
    }

    #[test]
    #[ignore = "dépend du périphérique audio de la machine"]
    fn open_default_device() {
        let a = AudioOut::new();
        eprintln!("audio : {:?}", a.as_ref().err());
    }

    #[test]
    fn frames_are_converted_to_device_format() {
        let mut q: VecDeque<f32> = VecDeque::from(vec![1.0, -1.0, 0.5, 0.5]);
        let mut out = [0i16; 4];
        fill_frames(&mut q, &mut out, 2);
        assert_eq!(out, [i16::MAX, i16::MIN, 16384, 16384]);
        let mut q: VecDeque<f32> = VecDeque::from(vec![0.2, 0.4]);
        let mut mono = [0f32; 1];
        fill_frames(&mut q, &mut mono, 1);
        assert!((mono[0] - 0.3).abs() < 1e-6);
    }
}
