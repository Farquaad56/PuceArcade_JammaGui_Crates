//! Voies sonores de la carte de test pour l'oscilloscope du débogueur.

use puce_jamma::debug::AudioVoice;

/// Échantillons par voie (à 48 kHz).
const N: u64 = 1024;

/// Trois voies calculées à partir du compteur audio : le bip réellement joué (bouton 1), un sinus
/// 440 Hz et une dent de scie dont la fréquence suit le curseur (affichage seulement).
pub fn voices(audio_counter: u64, bip: bool, cursor_x: i32) -> Vec<AudioVoice> {
    let start = audio_counter.saturating_sub(N);
    let saw_hz = 100 + cursor_x.max(0) as u64 * 4;
    let voice = |name: &str, active: bool, detail: String, f: &dyn Fn(u64) -> f32| AudioVoice {
        name: name.to_string(),
        chip: "Mire".to_string(),
        sample_rate: 48_000,
        samples: (start..start + N).map(f).collect(),
        active,
        detail,
    };
    vec![
        voice("Bip", bip, "1000 Hz".into(), &|n| {
            if !bip {
                0.0
            } else if n % 48 < 24 {
                0.25
            } else {
                -0.25
            }
        }),
        voice("Sinus", true, "440 Hz".into(), &|n| {
            (n as f32 * 440.0 / 48_000.0 * std::f32::consts::TAU).sin() * 0.8
        }),
        voice("Scie", true, format!("{saw_hz} Hz"), &|n| {
            ((n * saw_hz) % 48_000) as f32 / 24_000.0 - 1.0
        }),
    ]
}

#[cfg(test)]
mod tests {
    use crate::TestBoard;
    use puce_jamma::board::JammaBoard;
    use puce_jamma::debug::BoardDebug;
    use puce_jamma::inputs::JammaInputs;
    use puce_jamma::video::Rotation;

    #[test]
    fn voices_and_mute() {
        let mut board = TestBoard::new(Rotation::Rot0);
        let mut out = Vec::new();
        board.take_audio(&mut out);
        let v = board.voices();
        assert_eq!(v.len(), 3);
        assert!(v
            .iter()
            .all(|v| v.samples.len() == 1024 && v.sample_rate == 48_000));
        assert!(!v[0].active && v[0].samples.iter().all(|s| *s == 0.0));
        assert!(v[1].samples.iter().any(|s| *s > 0.7));

        let mut inputs = JammaInputs::default();
        inputs.players[0].buttons[0] = true;
        board.set_inputs(&inputs);
        board.take_audio(&mut out);
        assert!(out.iter().any(|s| *s != 0.0));
        board.set_voice_mute(1);
        board.take_audio(&mut out);
        assert!(out.iter().all(|s| *s == 0.0));
        // la voie coupée reste visible à l'oscilloscope
        assert!(board.voices()[0].samples.iter().any(|s| *s != 0.0));
    }
}
