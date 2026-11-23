//! Contenu des onglets de la fenêtre « Filtres vidéo » (voir `fx_menu.rs`). Chaque section a un
//! titre en gras et une grille « libellé | réglage » ; les réglages sans effet sont masqués.

use eframe::egui;

use crate::fx_menu::FxEnv;
use crate::pixel2x::Scaler;
use crate::postfx::{Fx, Mask, ScanMode, MEDIUM, NEUTRAL_TEMP, SHARP, SOFT};

/// Titre de section, avec un peu d'espace au-dessus.
fn section(ui: &mut egui::Ui, title: &str) {
    ui.add_space(4.0);
    ui.strong(title);
}

/// Grille à deux colonnes alignées.
fn grid(ui: &mut egui::Ui, id: &str, add: impl FnOnce(&mut egui::Ui)) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([12.0, 4.0])
        .min_col_width(130.0)
        .show(ui, add);
}

/// Ligne « libellé | curseur en % ».
fn pct(ui: &mut egui::Ui, label: &str, v: &mut u8, range: std::ops::RangeInclusive<u8>, step: f64) {
    ui.label(label);
    ui.add(egui::Slider::new(v, range).suffix(" %").step_by(step));
    ui.end_row();
}

/// Ligne « libellé | curseur » avec un affichage personnalisé.
fn custom(
    ui: &mut egui::Ui,
    label: &str,
    v: &mut u8,
    range: std::ops::RangeInclusive<u8>,
    fmt: impl Fn(f64) -> String + 'static,
) {
    ui.label(label);
    ui.add(egui::Slider::new(v, range).custom_formatter(move |x, _| fmt(x)));
    ui.end_row();
}

/// Choix dans une liste déroulante.
fn choice<T: Copy + PartialEq>(ui: &mut egui::Ui, id: &str, v: &mut T, items: &[(&str, T)]) {
    let current = items
        .iter()
        .find(|(_, x)| *x == *v)
        .map_or("?", |(l, _)| *l);
    egui::ComboBox::from_id_salt(id)
        .selected_text(current)
        .width(230.0)
        .show_ui(ui, |ui| {
            for (label, x) in items {
                ui.selectable_value(v, *x, *label);
            }
        });
}

pub fn image(ui: &mut egui::Ui, fx: &mut Fx) {
    section(ui, "Agrandissement pixel art");
    let items: Vec<(&str, Scaler)> = Scaler::ALL.iter().map(|k| (k.label(), *k)).collect();
    choice(ui, "fx_scaler", &mut fx.scaler, &items);
    if fx.scaler != Scaler::Off {
        ui.weak("Double la définition avant les autres filtres ; utile à partir d'une échelle ×2.");
    }
    if fx.scaler.approximate() {
        ui.weak("Approximation : le code d'origine n'est pas public.");
    }

    section(ui, "Interpolation horizontale");
    ui.horizontal(|ui| {
        for (label, v) in [
            ("Aucune", 0),
            ("Douce", SOFT),
            ("Moyenne", MEDIUM),
            ("Nette", SHARP),
        ] {
            ui.selectable_value(&mut fx.interp, v, label);
        }
    });
    if fx.interp > 0 {
        grid(ui, "fx_interp", |ui| {
            pct(ui, "Netteté", &mut fx.interp, 5..=100, 5.0)
        });
    }

    section(ui, "Overscan");
    grid(ui, "fx_overscan", |ui| {
        pct(ui, "Bords coupés", &mut fx.overscan, 0..=10, 1.0)
    });
    ui.weak("Un vrai moniteur cachait quelques pixels sur chaque bord.");
}

pub fn lines(ui: &mut egui::Ui, fx: &mut Fx) {
    section(ui, "Scanlines");
    choice(
        ui,
        "fx_scan",
        &mut fx.scan_mode,
        &[
            ("Désactivées", ScanMode::Off),
            ("Standard", ScanMode::Standard),
            ("Adaptatives", ScanMode::Adaptive),
            ("Faisceau gaussien (réaliste)", ScanMode::Gaussian),
        ],
    );
    grid(ui, "fx_scan_grid", |ui| match fx.scan_mode {
        ScanMode::Off => {}
        ScanMode::Standard => pct(ui, "Profondeur", &mut fx.scan, 0..=95, 5.0),
        ScanMode::Adaptive => {
            pct(ui, "Pixels sombres", &mut fx.scan_dk, 0..=100, 10.0);
            pct(ui, "Pixels clairs", &mut fx.scan_br, 0..=100, 10.0);
        }
        ScanMode::Gaussian => {
            pct(ui, "Ligne sombre", &mut fx.beam_dk, 20..=150, 5.0);
            pct(ui, "Ligne claire", &mut fx.beam_br, 20..=150, 5.0);
        }
    });
    // garder sombre ≤ clair
    fx.scan_br = fx.scan_br.max(fx.scan_dk);
    fx.beam_br = fx.beam_br.max(fx.beam_dk);
    let hint = match fx.scan_mode {
        ScanMode::Off => "",
        ScanMode::Adaptive => "Luminosité conservée entre deux lignes, selon la clarté du pixel.",
        ScanMode::Gaussian => {
            "Épaisseur de la ligne en % de l'interligne : fine et séparée de noir quand elle est \
             sombre, épaisse quand elle est claire."
        }
        ScanMode::Standard => "Visibles à partir d'une échelle ×2.",
    };
    if !hint.is_empty() {
        ui.weak(hint);
    }

    section(ui, "Masque d'écran");
    choice(
        ui,
        "fx_mask",
        &mut fx.mask,
        &[
            ("Aucun", Mask::None),
            ("Fentes (moniteur d'arcade)", Mask::Slot),
            ("Points (shadow mask)", Mask::Shadow),
            ("Grille Sony Trinitron", Mask::Trinitron),
            ("Tube fané (jaune)", Mask::FadedYellow),
        ],
    );
    if fx.mask != Mask::None {
        grid(ui, "fx_mask_grid", |ui| {
            pct(ui, "Force", &mut fx.mask_pct, 0..=100, 5.0);
            if fx.mask.procedural() {
                custom(
                    ui,
                    "Taille des luminophores",
                    &mut fx.mask_pitch,
                    0..=4,
                    |v| {
                        if v < 0.5 {
                            "auto".into()
                        } else {
                            format!("{v:.0} px")
                        }
                    },
                );
            }
        });
    }
}

pub fn color(ui: &mut egui::Ui, fx: &mut Fx) {
    section(ui, "Lumière");
    grid(ui, "fx_light", |ui| {
        custom(ui, "Gamma", &mut fx.gamma, 100..=200, |v| {
            if v <= 100.0 {
                "désactivé".into()
            } else {
                format!("{:.2}", v / 100.0)
            }
        });
        pct(ui, "Niveau du noir", &mut fx.black, 0..=20, 1.0);
    });

    section(ui, "Température du blanc");
    grid(ui, "fx_temp", |ui| {
        custom(ui, "Blanc", &mut fx.temp, 50..=93, |v| {
            let k = v as u32 * 100;
            if k == u32::from(NEUTRAL_TEMP) * 100 {
                format!("{k} K (neutre)")
            } else {
                format!("{k} K")
            }
        });
    });
    ui.horizontal(|ui| {
        if ui.small_button("6 500 K (neutre)").clicked() {
            fx.temp = NEUTRAL_TEMP;
        }
        if ui.small_button("9 300 K (moniteur japonais)").clicked() {
            fx.temp = 93;
        }
    });

    section(ui, "Canaux RVB");
    grid(ui, "fx_rgb", |ui| {
        pct(ui, "Rouge", &mut fx.red, 0..=200, 1.0);
        pct(ui, "Vert", &mut fx.green, 0..=200, 1.0);
        pct(ui, "Bleu", &mut fx.blue, 0..=200, 1.0);
    });
    if ui.small_button("Canaux neutres").clicked() {
        (fx.red, fx.green, fx.blue) = (100, 100, 100);
    }
}

pub fn tube(ui: &mut egui::Ui, fx: &mut Fx, env: &FxEnv) {
    section(ui, "Géométrie");
    ui.add_enabled_ui(!env.bezel, |ui| {
        grid(ui, "fx_geom", |ui| {
            pct(ui, "Courbure", &mut fx.curve, 0..=100, 5.0);
            pct(ui, "Vignettage", &mut fx.vignette, 0..=100, 5.0);
            pct(ui, "Coins arrondis", &mut fx.corner, 0..=100, 5.0);
        });
    });
    if env.bezel {
        ui.weak("Désactivée : le bezel affiché dessine déjà le tube.");
    } else {
        ui.weak("Désactivée automatiquement en plein écran quand un bezel est affiché.");
    }

    section(ui, "Halo (lumière diffusée dans le verre)");
    grid(ui, "fx_bloom", |ui| {
        pct(ui, "Intensité", &mut fx.bloom, 0..=100, 5.0);
        if fx.bloom > 0 {
            custom(ui, "Taille", &mut fx.bloom_size, 1..=10, |v| {
                format!("{v:.0} lignes")
            });
        }
    });

    section(ui, "Convergence (décalage par rapport au vert)");
    egui::Grid::new("fx_conv").num_columns(4).show(ui, |ui| {
        for (label, v) in [("Rouge X", &mut fx.conv_rx), ("Y", &mut fx.conv_ry)] {
            ui.label(label);
            ui.add(egui::Slider::new(v, -8..=8).suffix(" px"));
        }
        ui.end_row();
        for (label, v) in [("Bleu X", &mut fx.conv_bx), ("Y", &mut fx.conv_by)] {
            ui.label(label);
            ui.add(egui::Slider::new(v, -8..=8).suffix(" px"));
        }
        ui.end_row();
    });
    ui.horizontal(|ui| {
        if ui.small_button("Convergence parfaite").clicked() {
            (fx.conv_rx, fx.conv_ry, fx.conv_bx, fx.conv_by) = (0, 0, 0, 0);
        }
        if ui.small_button("Tube fatigué").clicked() {
            (fx.conv_rx, fx.conv_ry, fx.conv_bx, fx.conv_by) = (1, 0, -1, 1);
        }
    });
    ui.weak("En pixels de l'image affichée : 1 px suffit souvent en ×4.");
}

pub fn effects(ui: &mut egui::Ui, fx: &mut Fx, env: &FxEnv) {
    section(ui, "Barre de balayage");
    grid(ui, "fx_sweep", |ui| {
        pct(ui, "Intensité", &mut fx.sweep, 0..=100, 5.0);
        if fx.sweep > 0 {
            custom(
                ui,
                "Durée d'un passage",
                &mut fx.sweep_speed,
                5..=100,
                |v| format!("{:.1} s", v / 10.0),
            );
            pct(ui, "Hauteur", &mut fx.sweep_width, 2..=50, 1.0);
        }
    });

    section(ui, "Défauts du tube");
    grid(ui, "fx_defects", |ui| {
        pct(ui, "Instabilité de l'image", &mut fx.jitter, 0..=100, 5.0);
        pct(ui, "Taches de pureté", &mut fx.purity, 0..=100, 5.0);
    });
    ui.weak("Taches de couleur dans les coins, comme sur un tube aimanté.");

    section(ui, "Mouvement");
    ui.checkbox(
        &mut fx.bfi,
        "Images noires entre les images (netteté du mouvement)",
    );
    let status = if env.game_hz <= 0.0 {
        "Chargez un jeu pour vérifier la fréquence de l'écran.".to_string()
    } else if env.bfi_possible() {
        format!(
            "Écran à {:.0} Hz : possible (jeu à {:.2} Hz). L'image paraît moins lumineuse.",
            env.refresh_hz, env.game_hz
        )
    } else {
        format!(
            "Écran à {:.0} Hz : il faut au moins {:.0} Hz (deux fois la fréquence du jeu) ; \
             option sans effet.",
            env.refresh_hz,
            (1.8 * env.game_hz).ceil()
        )
    };
    ui.weak(status);
}
