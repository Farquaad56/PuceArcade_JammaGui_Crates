//! Filtres sur carte graphique comparés aux filtres processeur (`postfx::render`), réglage par
//! réglage, sur une image de synthèse (dégradés, bords francs, bruit). Sans carte graphique
//! utilisable (aucun adaptateur), les tests ne font rien.

use eframe::wgpu;

use crate::gpufx_data::build_job;
use crate::gpufx_offscreen::{offscreen_fx, render_offscreen};
use crate::pixel2x::Scaler;
use crate::postfx::{render, Fx, Mask, Preset, ScanMode};

pub(crate) fn block_on<F: std::future::Future>(f: F) -> F::Output {
    let mut f = std::pin::pin!(f);
    let mut cx = std::task::Context::from_waker(std::task::Waker::noop());
    loop {
        if let std::task::Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        std::thread::yield_now();
    }
}

/// Carte graphique de test (la première trouvée), avec ses propres limites.
pub(crate) fn device() -> Option<(wgpu::Device, wgpu::Queue, wgpu::TextureFormat)> {
    let inst = wgpu::Instance::new(wgpu::InstanceDescriptor {
        backends: wgpu::Backends::all(),
        ..wgpu::InstanceDescriptor::new_without_display_handle()
    });
    let adapter = block_on(inst.request_adapter(&wgpu::RequestAdapterOptions::default())).ok()?;
    let limits = adapter.limits();
    // OpenGL : les cibles en flottants 32 bits demandent les formats propres à l'adaptateur
    let features = adapter.features() & wgpu::Features::TEXTURE_ADAPTER_SPECIFIC_FORMAT_FEATURES;
    let (d, q) = block_on(adapter.request_device(&wgpu::DeviceDescriptor {
        required_limits: limits,
        required_features: features,
        ..Default::default()
    }))
    .ok()?;
    let f = crate::gpufx::float_format(&adapter, &d)?;
    Some((d, q, f))
}

/// Image de synthèse 384 × 224 : dégradés, damier, bords francs et bruit.
pub(crate) fn picture() -> Vec<u32> {
    let (w, h) = (384u32, 224u32);
    let mut seed = 0x1234_5678u32;
    (0..w * h)
        .map(|i| {
            let (x, y) = (i % w, i / w);
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            let noise = seed >> 24;
            let (r, g, b) = match (x / 96, y / 56) {
                (0, _) => (x * 255 / 95, y * 255 / 223, 128),
                (1, _) if (x / 4 + y / 4) % 2 == 0 => (255, 255, 255),
                (1, _) => (0, 0, 0),
                (2, 0..=1) => (255, 40, 20),
                (2, _) => (20, 60, 240),
                _ => (noise, (noise * 7) & 255, (noise * 13) & 255),
            };
            (r << 16) | (g << 8) | b
        })
        .collect()
}

/// Écarts : (plus grand écart d'un canal, part des pixels dont un canal s'écarte de plus de 2).
fn compare(fx: &Fx, w: usize, h: usize, frame: u64) -> Option<(u8, f64)> {
    let (dev, q, float) = device()?;
    let src = picture();
    let cpu = render(&src, 384, 224, fx, w, h, frame);
    let job = build_job(&src, 384, 224, fx, w, h, frame);
    let gpu = render_offscreen(&dev, &q, &mut offscreen_fx(&dev, float), &job);
    assert_eq!(cpu.len(), gpu.len());
    let (mut max, mut far) = (0u8, 0usize);
    for (c, g) in cpu.iter().zip(&gpu) {
        let d = [
            c.r().abs_diff(g[0]),
            c.g().abs_diff(g[1]),
            c.b().abs_diff(g[2]),
        ];
        let m = d.into_iter().max().unwrap_or(0);
        max = max.max(m);
        far += usize::from(m > 2);
    }
    Some((max, far as f64 / cpu.len() as f64))
}

/// Cas comparés : nom, réglages, taille de sortie, numéro de trame.
fn cases() -> Vec<(&'static str, Fx, usize, usize, u64)> {
    let d = Fx::default();
    let arcade = Fx::preset(Preset::Arcade);
    vec![
        ("sans filtre x3", d, 1152, 672, 0),
        ("Arcade x4", arcade, 1536, 896, 0),
        ("Arcade x3", arcade, 1152, 672, 0),
        (
            "Trinitron x4",
            Fx::preset(Preset::SonyTrinitron),
            1536,
            896,
            0,
        ),
        ("Tube fané x4", Fx::preset(Preset::FadedTube), 1536, 896, 0),
        (
            "scanlines standard",
            Fx {
                scan_mode: ScanMode::Standard,
                scan: 70,
                ..d
            },
            1152,
            672,
            0,
        ),
        ("interpolation nette", Fx { interp: 85, ..d }, 1000, 600, 0),
        (
            "gamma + masque à points",
            Fx {
                gamma: 145,
                mask: Mask::Shadow,
                mask_pct: 80,
                ..d
            },
            1536,
            896,
            0,
        ),
        (
            "hq2x + overscan",
            Fx {
                scaler: Scaler::Hq2x,
                overscan: 6,
                interp: 55,
                ..d
            },
            1536,
            896,
            0,
        ),
        (
            "courbure, vignette, coins",
            Fx {
                curve: 60,
                vignette: 50,
                corner: 70,
                ..arcade
            },
            1536,
            896,
            0,
        ),
        (
            "instabilité",
            Fx {
                jitter: 80,
                ..arcade
            },
            1536,
            896,
            12_345,
        ),
        (
            "convergence, canaux, balayage",
            Fx {
                conv_rx: 3,
                conv_ry: -2,
                conv_bx: -4,
                conv_by: 1,
                red: 140,
                blue: 60,
                sweep: 80,
                ..arcade
            },
            1536,
            896,
            37,
        ),
        (
            "pureté, température, noir",
            Fx {
                purity: 70,
                temp: 50,
                black: 15,
                ..arcade
            },
            1536,
            896,
            0,
        ),
        (
            "halo large",
            Fx {
                bloom: 100,
                bloom_size: 10,
                ..d
            },
            1152,
            672,
            0,
        ),
        (
            "faisceau large",
            Fx {
                scan_mode: ScanMode::Gaussian,
                beam_dk: 20,
                beam_br: 150,
                ..d
            },
            1536,
            896,
            0,
        ),
    ]
}

#[test]
fn gpu_matches_cpu() {
    let mut bad = Vec::new();
    for (name, fx, w, h, frame) in cases() {
        let Some((max, far)) = compare(&fx, w, h, frame) else {
            eprintln!("pas de carte graphique : test ignoré");
            return;
        };
        eprintln!(
            "{name:32} écart max {max:3}  pixels > 2 : {:.4} %",
            far * 100.0
        );
        // quelques arrondis de flottants (division, racine) peuvent changer un pixel isolé
        if far > 0.0005 || max > 6 {
            bad.push(name);
        }
    }
    assert!(bad.is_empty(), "rendus différents : {bad:?}");
}

/// Temps par image en x8 (3072 × 1792, Arcade) : processeur contre carte graphique
/// (préparation + passes + relecture, donc plus que dans la fenêtre où rien n'est relu).
/// `cargo test --release -p puce-jamma-gui --lib bench_gpu_fx -- --ignored --nocapture`
#[test]
#[ignore]
fn bench_gpu_fx() {
    let Some((dev, q, float)) = device() else {
        return;
    };
    let src = picture();
    let fx = Fx::preset(Preset::Arcade);
    let (w, h) = (3072, 1792);
    let n = 20;
    let t = std::time::Instant::now();
    for f in 0..n {
        std::hint::black_box(render(&src, 384, 224, &fx, w, h, f));
    }
    let cpu = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let job = build_job(&src, 384, 224, &fx, w, h, 0);
    let mut gfx = offscreen_fx(&dev, float);
    render_offscreen(&dev, &q, &mut gfx, &job);
    let t = std::time::Instant::now();
    for f in 0..n {
        let job = build_job(&src, 384, 224, &fx, w, h, f);
        std::hint::black_box(render_offscreen(&dev, &q, &mut gfx, &job));
    }
    let gpu = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    let t = std::time::Instant::now();
    for f in 0..n {
        std::hint::black_box(build_job(&src, 384, 224, &fx, w, h, f));
    }
    let prep = t.elapsed().as_secs_f64() * 1000.0 / n as f64;
    eprintln!("x8 Arcade : processeur {cpu:.1} ms, carte graphique {gpu:.1} ms avec relecture (préparation {prep:.2} ms)");
}
