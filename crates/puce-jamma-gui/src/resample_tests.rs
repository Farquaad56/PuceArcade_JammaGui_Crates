//! Tests du rééchantillonneur sinc fenêtré.

use super::*;

fn run(src: f64, dst: u32, input: &[[f32; 2]]) -> Vec<[f32; 2]> {
    let mut r = AudioResampler::new(src, dst);
    for &s in input {
        r.push(s);
    }
    let mut v = Vec::new();
    r.pop_into(&mut v);
    v.as_chunks::<2>().0.to_vec()
}

fn sine(hz: f64, rate: f64, n: usize) -> Vec<[f32; 2]> {
    (0..n)
        .map(|i| {
            let v = (0.5 * (2.0 * std::f64::consts::PI * hz * i as f64 / rate).sin()) as f32;
            [v, -v]
        })
        .collect()
}

/// Puissance de `x` à la fréquence `hz` (projection sur sinus et cosinus).
fn power_at(x: &[f32], hz: f64, rate: f64) -> f64 {
    let (mut c, mut s) = (0.0, 0.0);
    for (i, v) in x.iter().enumerate() {
        let a = 2.0 * std::f64::consts::PI * hz * i as f64 / rate;
        c += f64::from(*v) * a.cos();
        s += f64::from(*v) * a.sin();
    }
    (c * c + s * s) / (x.len() as f64).powi(2)
}

#[test]
fn output_count_follows_rates() {
    let n = run(55_930.4, 48_000, &[[0.5, 0.5]; 55_930]).len();
    assert!((47_970..=48_001).contains(&n), "{n}");
    // amorçage : 31 échantillons source (1,3 ms) = 62 sorties à 48 kHz
    let n = run(24_038.46, 48_000, &[[0.5, 0.5]; 24_038]).len();
    assert!((47_930..=48_001).contains(&n), "{n}");
}

#[test]
fn src_rate_change_scales_the_output_count() {
    let mut r = AudioResampler::new(55_555.56, 48_000);
    r.set_src_hz(55_555.56 * 0.995);
    for _ in 0..55_556 {
        r.push([0.5, 0.5]);
    }
    let mut v = Vec::new();
    r.pop_into(&mut v);
    let base = run(55_555.56, 48_000, &[[0.5, 0.5]; 55_556]).len();
    assert!(v.len() / 2 > base + 200);
}

#[test]
fn dc_has_unit_gain() {
    let out = run(24_038.46, 48_000, &[[0.5, -0.25]; 4000]);
    assert!(out
        .iter()
        .all(|s| (s[0] - 0.5).abs() < 1e-4 && (s[1] + 0.25).abs() < 1e-4));
}

#[test]
fn passband_sine_is_kept_and_channels_stay_apart() {
    let out = run(24_038.46, 48_000, &sine(1000.0, 24_038.46, 24_038));
    let left: Vec<f32> = out.iter().map(|s| s[0]).collect();
    let p = power_at(&left[100..], 1000.0, 48_000.0);
    // sinus d'amplitude 0,5 : puissance (0,5 / 2)² = 0,0625
    assert!((p - 0.0625).abs() < 0.002, "{p}");
    assert!(
        out[100..].iter().all(|s| (s[0] + s[1]).abs() < 1e-4),
        "voies mélangées"
    );
}

#[test]
fn images_are_rejected() {
    // 10 kHz à 24 038 Hz : l'image de l'interpolation linéaire tombait à 14 038 Hz
    let out = run(24_038.46, 48_000, &sine(10_000.0, 24_038.46, 24_038));
    let left: Vec<f32> = out.iter().map(|s| s[0]).collect();
    let tone = power_at(&left[100..], 10_000.0, 48_000.0);
    let image = power_at(&left[100..], 14_038.46, 48_000.0);
    let db = 10.0 * (image / tone).log10();
    assert!(db < -60.0, "image à {db:.1} dB");
}

#[test]
fn mono_is_duplicated_to_stereo() {
    let mut r = AudioResampler::new(48_000.0, 48_000);
    r.push_interleaved(&[0.5; 100], 1);
    let mut v = Vec::new();
    r.pop_into(&mut v);
    assert!(!v.is_empty() && v.iter().all(|x| (x - 0.5).abs() < 1e-4));
}

#[test]
fn stereo_keeps_channels() {
    let mut r = AudioResampler::new(48_000.0, 48_000);
    let input: Vec<f32> = (0..200).flat_map(|_| [0.1, -0.2]).collect();
    r.push_interleaved(&input, 2);
    let mut v = Vec::new();
    r.pop_into(&mut v);
    assert!(v
        .as_chunks::<2>()
        .0
        .iter()
        .all(|c| (c[0] - 0.1).abs() < 1e-4 && (c[1] + 0.2).abs() < 1e-4));
}
