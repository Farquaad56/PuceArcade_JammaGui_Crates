//! puce-jamma-testboard — carte de test JAMMA (mire, entrées, bip)
//!
//! Implémente `JammaBoard` et `BoardDebug` pour tester les frontaux
//! sans émulateur réel. Dessine une mire avec barres de couleur, grille,
//! témoins d'entrées et curseur contrôlable.

#![forbid(unsafe_code)]

use puce_jamma::audio::AudioInfo;
use puce_jamma::board::{BoardInfo, JammaBoard};
use puce_jamma::debug::{BoardDebug, CpuView, DebugImage};
use puce_jamma::dips::{DipBank, DipSetting, DipSwitch};
use puce_jamma::inputs::JammaInputs;
use puce_jamma::video::{Rotation, VideoInfo, VideoOut};

mod draw;
pub mod driver;

/// Carte de test JAMMA (mire).
pub struct TestBoard {
    info: BoardInfo,
    frame: u64,
    cursor_x: i32,
    cursor_y: i32,
    inputs: JammaInputs,
    dips: Vec<u8>,
    layer_mask: u32,
    pixels: Vec<u32>,
    audio_counter: u64,
}

impl TestBoard {
    /// Crée une nouvelle carte de test avec la rotation spécifiée.
    pub fn new(rotation: Rotation) -> Self {
        let (name, aspect) = match rotation {
            Rotation::Rot0 | Rotation::Rot180 => ("mire", (4, 3)),
            _ => ("mire_v", (3, 4)),
        };

        let info = BoardInfo {
            system: "Test".to_string(),
            name: name.to_string(),
            title: "Mire de test JAMMA".to_string(),
            players: 2,
            buttons: 6,
            video: VideoInfo {
                width: 288,
                height: 224,
                refresh_hz: 60.0,
                aspect,
                rotation,
            },
            audio: AudioInfo {
                sample_rate: 48000.0,
                channels: 1,
            },
            dip_banks: vec![DipBank {
                name: "DSW".to_string(),
                default: 0xFF,
            }],
            dips: vec![
                DipSwitch {
                    bank: 0,
                    mask: 0x01,
                    label: "Grille".to_string(),
                    default: 0x01,
                    settings: vec![
                        DipSetting {
                            label: "Non".to_string(),
                            value: 0x00,
                        },
                        DipSetting {
                            label: "Oui".to_string(),
                            value: 0x01,
                        },
                    ],
                },
                DipSwitch {
                    bank: 0,
                    mask: 0x06,
                    label: "Vitesse du curseur".to_string(),
                    default: 0x06,
                    settings: vec![
                        DipSetting {
                            label: "1".to_string(),
                            value: 0x06,
                        },
                        DipSetting {
                            label: "2".to_string(),
                            value: 0x04,
                        },
                        DipSetting {
                            label: "3".to_string(),
                            value: 0x02,
                        },
                        DipSetting {
                            label: "4".to_string(),
                            value: 0x00,
                        },
                    ],
                },
            ],
            has_analog: false,
        };

        let dips = vec![info.dip_banks[0].default];

        Self {
            info,
            frame: 0,
            cursor_x: 140,
            cursor_y: 108,
            inputs: JammaInputs::default(),
            dips,
            layer_mask: 0b111,
            pixels: vec![0x202020; 288 * 224],
            audio_counter: 0,
        }
    }

    #[allow(dead_code)]
    fn get_pixel(&self, x: i32, y: i32) -> u32 {
        if !(0..288).contains(&x) || !(0..224).contains(&y) {
            return 0;
        }
        self.pixels[(y * 288 + x) as usize]
    }

    fn set_pixel(&mut self, x: i32, y: i32, color: u32) {
        if !(0..288).contains(&x) || !(0..224).contains(&y) {
            return;
        }
        self.pixels[(y * 288 + x) as usize] = color;
    }

    fn draw_frame(&mut self) {
        // Fond
        for y in 0..224 {
            for x in 0..288 {
                self.set_pixel(x, y, 0x202020);
            }
        }

        // Couche 0 : barres de couleur (toujours)
        if self.layer_mask & 1 != 0 {
            let colors = [
                0xFFFFFF, 0xFFFF00, 0x00FFFF, 0x00FF00, 0xFF00FF, 0xFF0000, 0x0000FF, 0x000000,
            ];
            for (i, &color) in colors.iter().enumerate() {
                let x_start = i * 36;
                for y in 0..112 {
                    for x in x_start..(x_start + 36) {
                        self.set_pixel(x as i32, y, color);
                    }
                }
            }
        }

        // Couche 1 : grille (si DIP bit 0 est actif)
        if self.layer_mask & 2 != 0 && (self.dips[0] & 0x01) != 0 {
            for y in 112..224 {
                for x in 0..288 {
                    if x % 16 == 0 || y % 16 == 0 {
                        self.set_pixel(x, y, 0x808080);
                    }
                }
            }
        }

        // Témoins des boutons (toujours)
        for n in 0..12 {
            let x: i32 = 8 + 12 * n as i32;
            let y: i32 = 200;
            let pressed = if n < 6 {
                self.inputs.players[0].buttons[n]
            } else {
                self.inputs.players[1].buttons[n - 6]
            };
            let color = if pressed { 0x00FF00 } else { 0x004000 };
            for dy in 0..8 {
                for dx in 0..8 {
                    self.set_pixel(x + dx, y + dy, color);
                }
            }
        }

        // Couche 2 : curseur
        if self.layer_mask & 4 != 0 {
            for dy in 0..8 {
                for dx in 0..8 {
                    self.set_pixel(self.cursor_x + dx, self.cursor_y + dy, 0xFFFFFF);
                }
            }
        }
    }

    fn move_cursor(&mut self) {
        let speed = match (self.dips[0] >> 1) & 3 {
            0 => 4, // 0x00 -> vitesse 4
            1 => 3, // 0x02 -> vitesse 3
            2 => 2, // 0x04 -> vitesse 2
            _ => 1, // 0x06 -> vitesse 1
        };

        let mut dx = 0;
        let mut dy = 0;
        if self.inputs.players[0].left {
            dx -= speed;
        }
        if self.inputs.players[0].right {
            dx += speed;
        }
        if self.inputs.players[0].up {
            dy -= speed;
        }
        if self.inputs.players[0].down {
            dy += speed;
        }

        self.cursor_x = (self.cursor_x + dx).clamp(0, 280);
        self.cursor_y = (self.cursor_y + dy).clamp(0, 216);
    }
}

impl JammaBoard for TestBoard {
    fn info(&self) -> &BoardInfo {
        &self.info
    }

    fn reset(&mut self) {
        self.frame = 0;
        self.cursor_x = 140;
        self.cursor_y = 108;
        self.inputs = JammaInputs::default();
        self.dips[0] = self.info.dip_banks[0].default;
        self.layer_mask = 0b111;
        self.audio_counter = 0;
    }

    fn run_frame(&mut self) {
        self.move_cursor();
        self.frame += 1;
        self.draw_frame();
    }

    fn frame_count(&self) -> u64 {
        self.frame
    }

    fn video(&self) -> VideoOut<'_> {
        VideoOut {
            info: self.info.video,
            pixels: &self.pixels,
        }
    }

    fn take_audio(&mut self, out: &mut Vec<f32>) {
        // 800 échantillons par trame (48000 / 60)
        let samples = 800;
        out.clear();
        for _ in 0..samples {
            if self.inputs.players[0].buttons[0] {
                // Carré 1 kHz : période de 48 échantillons (48000/1000)
                let phase = (self.audio_counter % 48) < 24;
                out.push(if phase { 0.25 } else { -0.25 });
            } else {
                out.push(0.0);
            }
            self.audio_counter += 1;
        }
    }

    fn set_inputs(&mut self, inputs: &JammaInputs) {
        self.inputs = *inputs;
    }

    fn dips(&self) -> Vec<u8> {
        self.dips.clone()
    }

    fn set_dips(&mut self, banks: &[u8]) {
        if !banks.is_empty() {
            self.dips[0] = banks[0];
        }
    }

    fn save_state(&mut self) -> Vec<u8> {
        let mut data = vec![b'M', b'I', b'R', b'E']; // En-tête "MIRE"
        data.extend_from_slice(&self.frame.to_le_bytes());
        data.extend_from_slice(&self.cursor_x.to_le_bytes());
        data.extend_from_slice(&self.cursor_y.to_le_bytes());
        data.push(self.dips[0]);
        data.extend_from_slice(&self.audio_counter.to_le_bytes());
        data
    }

    fn load_state(&mut self, data: &[u8]) -> Result<(), String> {
        if data.len() < 4 || &data[0..4] != b"MIRE" {
            return Err("En-tête invalide".to_string());
        }
        if data.len() != 29 {
            return Err("Taille incorrecte".to_string());
        }

        self.frame = u64::from_le_bytes(data[4..12].try_into().unwrap());
        self.cursor_x = i32::from_le_bytes(data[12..16].try_into().unwrap());
        self.cursor_y = i32::from_le_bytes(data[16..20].try_into().unwrap());
        self.dips[0] = data[20];
        self.audio_counter = u64::from_le_bytes(data[21..29].try_into().unwrap());

        Ok(())
    }

    fn debug(&mut self) -> Option<&mut dyn BoardDebug> {
        Some(self)
    }
}

impl BoardDebug for TestBoard {
    fn cpus(&self) -> Vec<CpuView> {
        vec![CpuView {
            name: "MIRE".to_string(),
            pc: self.frame as u32,
            registers: vec![
                ("X".to_string(), self.cursor_x as u32),
                ("Y".to_string(), self.cursor_y as u32),
            ],
            address_bits: 16,
        }]
    }

    fn peek(&self, _cpu: usize, addr: u32) -> u8 {
        // Retourne l'octet (addr % 8) de frame en LE
        let bytes = self.frame.to_le_bytes();
        bytes[(addr % 8) as usize]
    }

    fn step_instruction(&mut self, _cpu: usize) {
        self.run_frame();
    }

    fn registers(&self) -> Vec<(String, Vec<(String, String)>)> {
        vec![(
            "Mire".to_string(),
            vec![("Trame".to_string(), self.frame.to_string())],
        )]
    }

    fn images(&self) -> Vec<DebugImage> {
        let colors = [
            0xFFFFFF, 0xFFFF00, 0x00FFFF, 0x00FF00, 0xFF00FF, 0xFF0000, 0x0000FF, 0x000000,
        ];
        vec![DebugImage {
            name: "Barres".to_string(),
            width: 8,
            height: 1,
            pixels: colors.to_vec(),
        }]
    }

    fn layer_names(&self) -> Vec<String> {
        vec![
            "Barres".to_string(),
            "Grille".to_string(),
            "Curseur".to_string(),
        ]
    }

    fn set_layer_mask(&mut self, mask: u32) {
        self.layer_mask = mask;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma::inputs::PlayerControls;

    #[test]
    fn test_basic_drawing() {
        let mut board = TestBoard::new(Rotation::Rot0);
        board.run_frame();

        // Barres de couleur : pixel (0,0) doit être blanc
        assert_eq!(board.get_pixel(0, 0), 0xFFFFFF);
        // Pixel (287,0) doit être noir (dernière barre)
        assert_eq!(board.get_pixel(287, 0), 0x000000);
        // Pixel (36,0) doit être jaune (deuxième barre)
        assert_eq!(board.get_pixel(36, 0), 0xFFFF00);

        // Grille : pixel (0,112) doit être gris (ligne de grille)
        assert_eq!(board.get_pixel(0, 112), 0x808080);
        // Pixel (1,113) doit être fond (pas sur la grille)
        assert_eq!(board.get_pixel(1, 113), 0x202020);

        // Curseur initial à (140, 108)
        assert_eq!(board.get_pixel(140, 108), 0xFFFFFF);

        assert_eq!(board.frame_count(), 1);
    }

    #[test]
    fn test_cursor_movement() {
        let mut board = TestBoard::new(Rotation::Rot0);

        // Déplacer à droite pendant 3 trames (vitesse 1)
        for _ in 0..3 {
            let inputs = JammaInputs {
                players: [
                    PlayerControls {
                        right: true,
                        ..Default::default()
                    },
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ],
                ..Default::default()
            };
            board.set_inputs(&inputs);
            board.run_frame();
        }

        // Le curseur devrait être à x = 140 + 3 = 143
        assert_eq!(board.cursor_x, 143);
    }

    #[test]
    fn test_dip_speed() {
        let mut board = TestBoard::new(Rotation::Rot0);
        // Vitesse 4 (DIP 0x00)
        board.set_dips(&[0x00]);

        for _ in 0..3 {
            let inputs = JammaInputs {
                players: [
                    PlayerControls {
                        right: true,
                        ..Default::default()
                    },
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ],
                ..Default::default()
            };
            board.set_inputs(&inputs);
            board.run_frame();
        }

        // Le curseur devrait être à x = 140 + 3*4 = 152
        assert_eq!(board.cursor_x, 152);
    }

    #[test]
    fn test_cursor_bounds() {
        let mut board = TestBoard::new(Rotation::Rot0);

        // Déplacer à gauche pendant 200 trames (vitesse 1) -> dépasse x=0, clampé à 0
        for _ in 0..200 {
            let inputs = JammaInputs {
                players: [
                    PlayerControls {
                        left: true,
                        ..Default::default()
                    },
                    Default::default(),
                    Default::default(),
                    Default::default(),
                ],
                ..Default::default()
            };
            board.set_inputs(&inputs);
            board.run_frame();
        }

        // Le curseur devrait être à x = 0 (borné)
        assert_eq!(board.cursor_x, 0);
    }

    #[test]
    fn test_audio() {
        let mut board = TestBoard::new(Rotation::Rot0);

        // Appuyer sur J1 B1
        let inputs = JammaInputs {
            players: [
                PlayerControls {
                    buttons: [true, false, false, false, false, false],
                    ..Default::default()
                },
                Default::default(),
                Default::default(),
                Default::default(),
            ],
            ..Default::default()
        };
        board.set_inputs(&inputs);

        let mut audio = Vec::new();
        board.take_audio(&mut audio);

        // 800 échantillons
        assert_eq!(audio.len(), 800);
        // Les 24 premiers doivent être +0.25
        for sample in audio.iter().take(24) {
            assert_eq!(*sample, 0.25);
        }
        // Les 24 suivants doivent être -0.25
        for sample in audio.iter().skip(24).take(24) {
            assert_eq!(*sample, -0.25);
        }

        // Relâcher le bouton
        let inputs = JammaInputs::default();
        board.set_inputs(&inputs);

        audio.clear();
        board.take_audio(&mut audio);
        for sample in &audio {
            assert_eq!(*sample, 0.0);
        }
    }

    #[test]
    fn test_layer_mask() {
        let mut board = TestBoard::new(Rotation::Rot0);
        board.set_layer_mask(0b110); // Pas de barres
        board.run_frame();

        // Pixel (0,0) doit être fond (pas de barres)
        assert_eq!(board.get_pixel(0, 0), 0x202020);
    }

    #[test]
    fn test_dip_grid() {
        let mut board = TestBoard::new(Rotation::Rot0);
        board.set_dips(&[0xFE]); // Grille désactivée (bit 0 = 0)
        board.run_frame();

        // Pixel (0,112) doit être fond (pas de grille)
        assert_eq!(board.get_pixel(0, 112), 0x202020);
    }

    #[test]
    fn test_save_load_state() {
        let mut board = TestBoard::new(Rotation::Rot0);

        // Exécuter 10 trames
        for _ in 0..10 {
            board.run_frame();
        }

        // Sauvegarder l'état
        let state = board.save_state();

        // Exécuter 5 trames de plus
        for _ in 0..5 {
            board.run_frame();
        }

        // Charger l'état sauvegardé
        board.load_state(&state).unwrap();

        // Exécuter 5 trames
        for _ in 0..5 {
            board.run_frame();
        }

        // Les pixels doivent être identiques à ceux après 15 trames
        let mut board2 = TestBoard::new(Rotation::Rot0);
        for _ in 0..15 {
            board2.run_frame();
        }

        assert_eq!(board.pixels, board2.pixels);
    }

    #[test]
    fn test_load_invalid_state() {
        let mut board = TestBoard::new(Rotation::Rot0);
        let result = board.load_state(b"XXXX");
        assert!(result.is_err());
    }
}
