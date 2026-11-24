use super::*;
use crate::postfx::Preset;

#[test]
fn wgsl_struct_matches_fields() {
    let s = uniform_wgsl();
    assert!(s.starts_with("struct P {") && s.contains("    k0: f32,\n"));
    assert!(s.contains("conv_rx: i32"));
    assert_eq!(Params::default().bytes().len() % 16, 0);
    let fields = s.matches(": ").count();
    assert_eq!(fields % 4, 0);
}

#[test]
fn params_round_trip() {
    let mut p = Params::default();
    p.set_u("w", 3072);
    p.set_f("k0", 0.5);
    p.set_i("conv_rx", -3);
    let b = p.bytes();
    assert_eq!(&b[0..4], &3072u32.to_le_bytes());
    let i = FIELDS.iter().position(|(n, _)| *n == "conv_rx").unwrap() * 4;
    assert_eq!(&b[i..i + 4], &(-3i32).to_le_bytes());
    assert_eq!(p.get_u("w"), 3072);
}

#[test]
#[should_panic(expected = "paramètre inconnu")]
fn unknown_param() {
    Params::default().set_u("nope", 1);
}

#[test]
fn arcade_job() {
    let src = vec![0x00FF_8040u32; 384 * 224];
    let fx = Fx::preset(Preset::Arcade);
    let j = build_job(&src, 384, 224, &fx, 1536, 896, 0);
    assert_eq!((j.sw, j.sh, j.w, j.h), (384, 224, 1536, 896));
    assert_eq!(j.src.len(), 384 * 224 * 4);
    assert_eq!(&j.src[..4], &[0xFF, 0x80, 0x40, 255]);
    assert_eq!(j.params.get_u("mode"), 3);
    assert_eq!(j.tables.len() % TABLE_W, 0);
    let b = j.bloom.unwrap();
    assert_eq!((b.s, b.bw, b.bh), (3, 512, 299));
    assert_eq!(j.params.get_u("fin_on"), 1);
    assert_eq!(j.params.get_u("fin_int"), 1);
    // tables lisibles aux décalages annoncés : gamma de 0 = 0, de 255 = 255
    let g = j.params.get_u("o_gamma") as usize;
    assert_eq!((j.tables[g], j.tables[g + 255]), (0.0, 255.0));
    let (pw, ph) = (j.params.get_u("pw"), j.params.get_u("ph"));
    let m = j.params.get_u("o_mask") as usize;
    assert!(m + (pw * ph * 3) as usize <= j.tables.len());
}

#[test]
fn neutral_job_has_no_post() {
    let src = vec![0u32; 16 * 16];
    let j = build_job(&src, 16, 16, &Fx::default(), 64, 64, 0);
    for k in [
        "warp_on",
        "tube_on",
        "bloom_on",
        "fin_on",
        "kernel_on",
        "mode",
    ] {
        assert_eq!(j.params.get_u(k), 0, "{k}");
    }
    assert!(j.bloom.is_none());
}
