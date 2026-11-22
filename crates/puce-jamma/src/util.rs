//! Utilitaires fournis par `puce-jamma`.

use crate::board::BoardInfo;
use crate::dips::DipSwitch;
use crate::video::Rotation;

/// Tourne une image (sens horaire) et retourne les pixels + dimensions.
///
/// Rot90 : pixel source (x, y) -> destination (h-1-y, x), taille h × w.
/// Rot180 : (w-1-x, h-1-y).
/// Rot270 : (y, w-1-x), taille h × w.
pub fn rotate(src: &[u32], w: u32, h: u32, rot: Rotation) -> (Vec<u32>, u32, u32) {
    let (out_w, out_h) = match rot {
        Rotation::Rot0 | Rotation::Rot180 => (w, h),
        Rotation::Rot90 | Rotation::Rot270 => (h, w),
    };
    let mut dst = vec![0u32; (out_w * out_h) as usize];

    for y in 0..h {
        for x in 0..w {
            let src_idx = ((y * w) + x) as usize;
            let (dx, dy) = match rot {
                Rotation::Rot0 => (x, y),
                Rotation::Rot90 => (h - 1 - y, x),
                Rotation::Rot180 => (w - 1 - x, h - 1 - y),
                Rotation::Rot270 => (y, w - 1 - x),
            };
            let dst_idx = ((dy * out_w) + dx) as usize;
            dst[dst_idx] = src[src_idx];
        }
    }

    (dst, out_w, out_h)
}

/// Valeurs par défaut des DIP switches (un octet par banque).
pub fn default_dips(info: &BoardInfo) -> Vec<u8> {
    info.dip_banks.iter().map(|b| b.default).collect()
}

/// Lit la valeur d'un switch DIP.
pub fn dip_value(banks: &[u8], sw: &DipSwitch) -> u8 {
    banks[sw.bank] & sw.mask
}

/// Définit la valeur d'un switch DIP.
pub fn set_dip(banks: &mut [u8], sw: &DipSwitch, value: u8) {
    let byte = banks[sw.bank];
    banks[sw.bank] = (byte & !sw.mask) | (value & sw.mask);
}

/// Convertit une entrée active haute en active basse.
/// 0 si appuyé, `bit` sinon.
pub fn active_low(b: bool, bit: u8) -> u8 {
    if b {
        0
    } else {
        bit
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::JammaBoard;
    use crate::JammaInputs;

    #[test]
    fn test_rotate() {
        // Image 3x2 : [1,2,3,4,5,6]
        let src = vec![1, 2, 3, 4, 5, 6];

        // Rot0 -> identique (3,2)
        let (dst, w, h) = rotate(&src, 3, 2, Rotation::Rot0);
        assert_eq!(w, 3);
        assert_eq!(h, 2);
        assert_eq!(dst, vec![1, 2, 3, 4, 5, 6]);

        // Rot90 -> ([4,1,5,2,6,3], 2, 3)
        let (dst, w, h) = rotate(&src, 3, 2, Rotation::Rot90);
        assert_eq!(w, 2);
        assert_eq!(h, 3);
        assert_eq!(dst, vec![4, 1, 5, 2, 6, 3]);

        // Rot180 -> ([6,5,4,3,2,1], 3, 2)
        let (dst, w, h) = rotate(&src, 3, 2, Rotation::Rot180);
        assert_eq!(w, 3);
        assert_eq!(h, 2);
        assert_eq!(dst, vec![6, 5, 4, 3, 2, 1]);

        // Rot270 -> ([3,6,2,5,1,4], 2, 3)
        let (dst, w, h) = rotate(&src, 3, 2, Rotation::Rot270);
        assert_eq!(w, 2);
        assert_eq!(h, 3);
        assert_eq!(dst, vec![3, 6, 2, 5, 1, 4]);
    }

    #[test]
    fn test_default_dips() {
        let info = BoardInfo {
            system: "Test".to_string(),
            name: "test".to_string(),
            title: "Test Game".to_string(),
            players: 2,
            buttons: 3,
            video: crate::video::VideoInfo {
                width: 384,
                height: 224,
                refresh_hz: 60.0,
                aspect: (4, 3),
                rotation: Rotation::Rot0,
            },
            audio: crate::audio::AudioInfo {
                sample_rate: 44100.0,
                channels: 1,
            },
            dip_banks: vec![
                crate::dips::DipBank {
                    name: "SW(A)".to_string(),
                    default: 0xFF,
                },
                crate::dips::DipBank {
                    name: "SW(B)".to_string(),
                    default: 0xFD,
                },
            ],
            dips: vec![],
            has_analog: false,
        };

        let defaults = default_dips(&info);
        assert_eq!(defaults, vec![0xFF, 0xFD]);
    }

    #[test]
    fn test_dip_value() {
        let banks = [0xFD];
        let sw = DipSwitch {
            bank: 0,
            mask: 0x07,
            label: "Test".to_string(),
            default: 0x05,
            settings: vec![],
        };
        assert_eq!(dip_value(&banks, &sw), 0x05);
    }

    #[test]
    fn test_set_dip() {
        let mut banks = [0xFF];
        let sw = DipSwitch {
            bank: 0,
            mask: 0x07,
            label: "Test".to_string(),
            default: 0x05,
            settings: vec![],
        };
        set_dip(&mut banks, &sw, 0x02);
        assert_eq!(banks[0], 0xFA);
    }

    #[test]
    fn test_active_low() {
        assert_eq!(active_low(true, 0x10), 0);
        assert_eq!(active_low(false, 0x10), 0x10);
    }

    #[test]
    fn test_jamma_inputs_default() {
        let inputs = JammaInputs::default();
        // Tout relâché
        for p in &inputs.players {
            assert!(!p.up);
            assert!(!p.down);
            assert!(!p.left);
            assert!(!p.right);
            assert!(!p.start);
            for b in &p.buttons {
                assert!(!b);
            }
        }
        for c in &inputs.coins {
            assert!(!c);
        }
        assert!(!inputs.service);
        assert!(!inputs.test);
        assert!(!inputs.tilt);
        // Analog == [0; 4]
        assert_eq!(inputs.analog, [0, 0, 0, 0]);
    }

    #[test]
    fn test_box_dyn_jamma_board() {
        // Test de compilation : un objet Box<dyn JammaBoard> peut être construit
        struct MinimalBoard;
        impl JammaBoard for MinimalBoard {
            fn info(&self) -> &BoardInfo {
                panic!("not implemented");
            }
            fn reset(&mut self) {}
            fn run_frame(&mut self) {}
            fn frame_count(&self) -> u64 {
                0
            }
            fn video(&self) -> crate::video::VideoOut<'_> {
                panic!("not implemented");
            }
            fn take_audio(&mut self, _out: &mut Vec<f32>) {}
            fn set_inputs(&mut self, _inputs: &JammaInputs) {}
            fn dips(&self) -> Vec<u8> {
                vec![]
            }
            fn set_dips(&mut self, _banks: &[u8]) {}
            fn save_state(&mut self) -> Vec<u8> {
                vec![]
            }
            fn load_state(&mut self, _data: &[u8]) -> Result<(), String> {
                Ok(())
            }
        }

        let mut board: Box<dyn JammaBoard> = Box::new(MinimalBoard);
        board.reset();
    }
}
