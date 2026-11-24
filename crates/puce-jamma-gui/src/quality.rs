//! Qualité globale de l'émulation, résumée en un mot pour la barre de performance : vitesse du
//! jeu, images réellement affichées, marge du cœur d'émulation et son. La plus mauvaise mesure
//! l'emporte ; la raison principale est donnée quand tout n'est pas parfait.

use crate::overlay::{speed_health, Health, Perf};

/// Mesures prises en compte.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Measures {
    /// Vitesse du jeu par rapport au temps réel, en %.
    pub speed_pct: f32,
    /// Images affichées par seconde par l'interface.
    pub shown_fps: f32,
    /// Images par seconde du jeu.
    pub game_fps: f32,
    /// Temps d'émulation moyen d'une trame et budget, en ms.
    pub emu_ms: Option<f32>,
    pub budget_ms: f32,
    /// État du son (None : pas de sortie son).
    pub audio: Option<Health>,
}

/// Résultat : santé, mot affiché et raison principale.
#[derive(Clone, Debug, PartialEq)]
pub struct Quality {
    pub health: Health,
    pub reason: Option<String>,
}

impl Quality {
    pub fn label(&self) -> &'static str {
        match self.health {
            Health::Good => "Excellente",
            Health::Warn => "Correcte",
            Health::Bad => "Dégradée",
        }
    }
}

fn rank(h: Health) -> u8 {
    match h {
        Health::Good => 0,
        Health::Warn => 1,
        Health::Bad => 2,
    }
}

/// Évalue la qualité globale.
pub fn assess(m: &Measures) -> Quality {
    let mut issues: Vec<(Health, String)> = Vec::new();
    let speed = speed_health(m.speed_pct);
    if speed != Health::Good {
        issues.push((speed, format!("vitesse {:.0} %", m.speed_pct)));
    }
    if m.game_fps > 0.0 && m.shown_fps > 0.0 {
        let ratio = m.shown_fps / m.game_fps;
        let h = if ratio >= 0.95 {
            Health::Good
        } else if ratio >= 0.75 {
            Health::Warn
        } else {
            Health::Bad
        };
        if h != Health::Good {
            let shown = m.shown_fps.min(m.game_fps);
            issues.push((h, format!("affichage {shown:.0}/{:.0} i/s", m.game_fps)));
        }
    }
    if let Some(ms) = m.emu_ms {
        if ms >= m.budget_ms {
            issues.push((Health::Bad, "émulation trop lente".into()));
        } else if ms >= 0.9 * m.budget_ms {
            issues.push((Health::Warn, "marge d'émulation faible".into()));
        }
    }
    match m.audio {
        Some(Health::Bad) => issues.push((Health::Bad, "craquement du son".into())),
        Some(Health::Warn) => issues.push((Health::Warn, "tampon son irrégulier".into())),
        _ => {}
    }
    // la plus grave d'abord ; à gravité égale, l'ordre ci-dessus
    issues.sort_by_key(|(h, _)| std::cmp::Reverse(rank(*h)));
    match issues.into_iter().next() {
        Some((health, reason)) => Quality {
            health,
            reason: Some(reason),
        },
        None => Quality {
            health: Health::Good,
            reason: None,
        },
    }
}

/// Mesures tirées de la barre de performance.
pub fn measures(perf: &Perf) -> Measures {
    Measures {
        speed_pct: perf.speed_pct(),
        shown_fps: if perf.ui_ms > 0.0 {
            1000.0 / perf.ui_ms
        } else {
            0.0
        },
        game_fps: perf.real_fps,
        emu_ms: perf.avg(30),
        budget_ms: perf.budget_ms,
        audio: perf.audio_on.then(|| perf.audio_health()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> Measures {
        Measures {
            speed_pct: 100.0,
            shown_fps: 60.0,
            game_fps: 59.6,
            emu_ms: Some(9.7),
            budget_ms: 16.8,
            audio: Some(Health::Good),
        }
    }

    #[test]
    fn all_good() {
        let q = assess(&good());
        assert_eq!(
            (q.health, q.label(), q.reason),
            (Health::Good, "Excellente", None)
        );
        // écran 144 Hz : plus d'images d'interface que de trames du jeu
        assert_eq!(
            assess(&Measures {
                shown_fps: 144.0,
                ..good()
            })
            .health,
            Health::Good
        );
        // sans sortie son
        assert_eq!(
            assess(&Measures {
                audio: None,
                ..good()
            })
            .health,
            Health::Good
        );
    }

    #[test]
    fn slow_display_is_degraded() {
        // la capture de l'utilisateur : jeu à 60 i/s, interface à 55,9 ms par image
        let q = assess(&Measures {
            shown_fps: 1000.0 / 55.9,
            ..good()
        });
        assert_eq!(q.health, Health::Bad);
        assert_eq!(q.reason.as_deref(), Some("affichage 18/60 i/s"));
        let q = assess(&Measures {
            shown_fps: 50.0,
            ..good()
        });
        assert_eq!(q.health, Health::Warn);
    }

    #[test]
    fn worst_reason_wins() {
        let q = assess(&Measures {
            speed_pct: 93.0,
            audio: Some(Health::Bad),
            ..good()
        });
        assert_eq!(
            (q.health, q.reason.as_deref()),
            (Health::Bad, Some("craquement du son"))
        );
        let q = assess(&Measures {
            emu_ms: Some(16.0),
            ..good()
        });
        assert_eq!((q.health, q.label()), (Health::Warn, "Correcte"));
        let q = assess(&Measures {
            emu_ms: Some(17.0),
            speed_pct: 80.0,
            ..good()
        });
        assert_eq!(
            (q.health, q.reason.as_deref()),
            (Health::Bad, Some("vitesse 80 %"))
        );
    }
}
