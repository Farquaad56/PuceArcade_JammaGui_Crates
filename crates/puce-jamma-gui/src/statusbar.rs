//! Barre d'état : jeu, système, résolution, temps par trame, fps, échelle, marche/pause/arrêt.

/// Ce qu'affiche la barre d'état ; `None` = information indisponible.
#[derive(Default)]
pub struct StatusInfo<'a> {
    pub system: Option<&'a str>,
    pub game: Option<&'a str>,
    /// Résolution de l'image source (largeur, hauteur).
    pub resolution: Option<(u32, u32)>,
    /// Cadence du système en Hz.
    pub refresh_hz: Option<f64>,
    /// Temps d'émulation moyen par trame, en ms.
    pub ms: Option<f32>,
    pub fps: Option<f32>,
    pub scale: Option<&'a str>,
    /// Son de la carte : fréquence (Hz) et nombre de voies.
    pub audio: Option<(f64, u32)>,
    pub paused: bool,
    pub stopped: bool,
}

/// Valeur affichée quand une information manque.
const MISSING: &str = "–";

pub fn format_status(s: &StatusInfo) -> String {
    let or = |v: Option<&str>| v.unwrap_or(MISSING).to_string();
    let res = s
        .resolution
        .map_or_else(|| MISSING.into(), |(w, h)| format!("{}×{}", w, h));
    [
        or(s.system),
        or(s.game),
        res,
        s.ms.map_or_else(|| MISSING.into(), |m| format!("{m:.2} ms/trame")),
        s.fps
            .map_or_else(|| MISSING.into(), |f| format!("{f:.1} fps")),
        or(s.scale),
        s.audio.map_or_else(
            || MISSING.into(),
            |(hz, ch)| {
                format!(
                    "son {:.0} Hz {}",
                    hz,
                    if ch >= 2 { "stéréo" } else { "mono" }
                )
            },
        ),
        match (s.stopped, s.paused) {
            (true, _) => "ARRÊT",
            (false, true) => "PAUSE",
            (false, false) => "MARCHE",
        }
        .to_string(),
    ]
    .join(" · ")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_status() {
        let s = StatusInfo {
            system: Some("Capcom CPS1"),
            game: Some("ghouls"),
            resolution: Some((384, 224)),
            refresh_hz: Some(59.637),
            ms: Some(4.103),
            fps: Some(59.19),
            scale: Some("x3 (960x672)"),
            audio: Some((24038.46, 2)),
            paused: false,
            stopped: false,
        };
        assert_eq!(
            format_status(&s),
            "Capcom CPS1 · ghouls · 384×224 · 4.10 ms/trame · 59.2 fps · x3 (960x672) · son 24038 Hz stéréo · MARCHE"
        );
    }

    #[test]
    fn stopped_state() {
        let s = StatusInfo {
            stopped: true,
            paused: true,
            ..StatusInfo::default()
        };
        assert_eq!(format_status(&s), "– · – · – · – · – · – · – · ARRÊT");
    }

    #[test]
    fn missing_values_show_a_dash() {
        let s = StatusInfo {
            paused: true,
            ..StatusInfo::default()
        };
        assert_eq!(format_status(&s), "– · – · – · – · – · – · – · PAUSE");
    }

    #[test]
    fn resolution_and_refresh_displayed() {
        let s = StatusInfo {
            system: Some("Capcom CPS1"),
            game: Some("ghouls"),
            resolution: Some((384, 224)),
            refresh_hz: Some(59.637),
            ..StatusInfo::default()
        };
        let status = format_status(&s);
        assert!(status.contains("ghouls"));
        assert!(status.contains("384×224"));
    }
}
