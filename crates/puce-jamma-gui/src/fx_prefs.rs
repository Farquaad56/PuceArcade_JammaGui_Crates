//! Préférences des effets CRT ajoutés en v0.4 (section `[fx]` de `prefs.toml`) : lecture et
//! écriture des réglages du faisceau, de la couleur, du halo, de la géométrie et des défauts.

use crate::postfx::Fx;

/// Réglage entier : clé, bornes, accès au champ.
struct Field {
    key: &'static str,
    min: u8,
    max: u8,
    get: fn(&mut Fx) -> &mut u8,
}

const FIELDS: [Field; 13] = [
    Field {
        key: "beam_dark",
        min: 20,
        max: 150,
        get: |f| &mut f.beam_dk,
    },
    Field {
        key: "beam_bright",
        min: 20,
        max: 150,
        get: |f| &mut f.beam_br,
    },
    Field {
        key: "mask_pitch",
        min: 0,
        max: 4,
        get: |f| &mut f.mask_pitch,
    },
    Field {
        key: "temperature",
        min: 50,
        max: 93,
        get: |f| &mut f.temp,
    },
    Field {
        key: "black",
        min: 0,
        max: 20,
        get: |f| &mut f.black,
    },
    Field {
        key: "bloom",
        min: 0,
        max: 100,
        get: |f| &mut f.bloom,
    },
    Field {
        key: "bloom_size",
        min: 1,
        max: 10,
        get: |f| &mut f.bloom_size,
    },
    Field {
        key: "overscan",
        min: 0,
        max: 10,
        get: |f| &mut f.overscan,
    },
    Field {
        key: "curve",
        min: 0,
        max: 100,
        get: |f| &mut f.curve,
    },
    Field {
        key: "vignette",
        min: 0,
        max: 100,
        get: |f| &mut f.vignette,
    },
    Field {
        key: "corner",
        min: 0,
        max: 100,
        get: |f| &mut f.corner,
    },
    Field {
        key: "jitter",
        min: 0,
        max: 100,
        get: |f| &mut f.jitter,
    },
    Field {
        key: "purity",
        min: 0,
        max: 100,
        get: |f| &mut f.purity,
    },
];

/// Lignes `clé = valeur` des réglages (à la suite de la section `[fx]`).
pub fn write(f: &Fx) -> String {
    let mut f = *f;
    let mut s = String::new();
    for d in &FIELDS {
        s.push_str(&format!("{} = {}\n", d.key, (d.get)(&mut f)));
    }
    s.push_str(&format!("bfi = {}\n", f.bfi));
    s
}

/// Lit `clé = valeur` ; `true` si la clé est l'un de ces réglages (valeur invalide ignorée).
pub fn read(f: &mut Fx, key: &str, v: &str) -> bool {
    if key == "bfi" {
        match v {
            "true" => f.bfi = true,
            "false" => f.bfi = false,
            _ => {}
        }
        return true;
    }
    let Some(d) = FIELDS.iter().find(|d| d.key == key) else {
        return false;
    };
    if let Ok(n) = v.parse::<u8>() {
        *(d.get)(f) = n.clamp(d.min, d.max);
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_clamp() {
        let f = Fx {
            beam_dk: 30,
            temp: 93,
            bloom: 40,
            curve: 25,
            purity: 10,
            bfi: true,
            ..Fx::default()
        };
        let mut g = Fx::default();
        for line in write(&f).lines() {
            let (k, v) = line.split_once(" = ").unwrap();
            assert!(read(&mut g, k, v), "{k}");
        }
        assert_eq!(g, f);
        assert!(read(&mut g, "temperature", "200"));
        assert_eq!(g.temp, 93);
        assert!(!read(&mut g, "inconnu", "1"));
    }
}
