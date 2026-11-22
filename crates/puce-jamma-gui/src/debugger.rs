//! Débogueur générique (ergonomie reprise de ref/puce16-neogeo/ng-gui/debugger.rs), construit
//! uniquement à partir de `DebugSnapshot` : aucune connaissance d'un CPU ou d'une puce précis.
//! Onglets CPU (registres en hexadécimal), Mémoire (lecture seule par `Cmd::Peek`),
//! Registres et Vidéo (`video_debug.rs`).

use eframe::egui;

use crate::emu_thread::{DebugSnapshot, MAX_PEEK};
use crate::video_debug::VideoDebug;

/// Ce que le débogueur demande à l'application.
#[derive(Clone, Debug, PartialEq)]
pub enum DebugAction {
    Pause(bool),
    /// Une instruction du CPU sélectionné.
    Step(usize),
    FrameAdvance,
    LayerMask(u32),
    Peek {
        cpu: usize,
        addr: u32,
        len: usize,
    },
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Cpu,
    Memory,
    Registers,
    Video,
}

impl Tab {
    pub const ALL: [Tab; 4] = [Tab::Cpu, Tab::Memory, Tab::Registers, Tab::Video];

    /// Nom stable écrit dans prefs.toml.
    pub fn name(self) -> &'static str {
        match self {
            Tab::Cpu => "cpu",
            Tab::Memory => "memory",
            Tab::Registers => "registers",
            Tab::Video => "video",
        }
    }

    pub fn from_name(name: &str) -> Option<Tab> {
        Tab::ALL.into_iter().find(|t| t.name() == name)
    }

    pub fn label(self) -> &'static str {
        match self {
            Tab::Cpu => "CPU",
            Tab::Memory => "Mémoire",
            Tab::Registers => "Registres",
            Tab::Video => "Vidéo",
        }
    }
}

/// Valeur en hexadécimal sur le nombre de chiffres de `bits` (arrondi au chiffre supérieur).
pub fn format_reg(value: u32, bits: u8) -> String {
    let digits = usize::from(bits.clamp(4, 32)).div_ceil(4);
    let masked = if bits >= 32 {
        value
    } else {
        value & ((1u32 << bits) - 1)
    };
    format!("{masked:0digits$X}")
}

/// Colonne ASCII d'une ligne d'éditeur hexadécimal (caractères non imprimables -> `.`).
pub fn ascii_col(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|&b| {
            if (0x20..0x7F).contains(&b) {
                b as char
            } else {
                '.'
            }
        })
        .collect()
}

/// Adresse saisie en hexadécimal (préfixes `$` ou `0x` acceptés).
pub fn parse_addr(text: &str) -> Option<u32> {
    let t = text.trim();
    let t = t
        .strip_prefix('$')
        .or_else(|| t.strip_prefix("0x"))
        .unwrap_or(t);
    u32::from_str_radix(t, 16).ok()
}

/// Octets affichés par l'éditeur hexadécimal (16 lignes de 16).
const MEM_LEN: usize = 256;

pub struct Debugger {
    pub open: bool,
    pub tab: Tab,
    snap: DebugSnapshot,
    /// Génération de l'instantané (les textures de l'onglet Vidéo sont refaites quand elle change).
    pub(crate) generation: u64,
    cpu: usize,
    addr: u32,
    addr_text: String,
    memory: Vec<u8>,
    pub(crate) video: VideoDebug,
}

impl Debugger {
    /// `tab` : nom de l'onglet enregistré dans les préférences (CPU s'il est inconnu).
    pub fn new(open: bool, tab: &str) -> Self {
        Debugger {
            open,
            tab: Tab::from_name(tab).unwrap_or(Tab::Cpu),
            snap: DebugSnapshot::default(),
            generation: 0,
            cpu: 0,
            addr: 0,
            addr_text: "0".into(),
            memory: Vec::new(),
            video: VideoDebug::default(),
        }
    }

    pub fn set_snapshot(&mut self, s: DebugSnapshot) {
        self.snap = s;
        self.generation += 1;
    }

    pub fn set_memory(&mut self, m: Vec<u8>) {
        self.memory = m;
    }

    /// Commande de lecture pour l'onglet Mémoire.
    pub fn peek_cmd(&self) -> crate::emu_thread::Cmd {
        crate::emu_thread::Cmd::Peek {
            cpu: self.cpu,
            addr: self.addr,
            len: MEM_LEN.min(MAX_PEEK),
        }
    }

    /// Lecture à demander pour l'onglet Mémoire.
    pub fn peek_request(&self) -> DebugAction {
        DebugAction::Peek {
            cpu: self.cpu,
            addr: self.addr,
            len: MEM_LEN.min(MAX_PEEK),
        }
    }

    /// Dessine le panneau ; renvoie l'action demandée.
    pub fn ui(&mut self, ui: &mut egui::Ui, paused: bool) -> Option<DebugAction> {
        let mut action = None;
        ui.horizontal(|ui| {
            let label = if paused { "Continuer" } else { "Pause" };
            if ui.button(label).clicked() {
                action = Some(DebugAction::Pause(!paused));
            }
            if ui
                .button("Pas")
                .on_hover_text("Une instruction du CPU sélectionné")
                .clicked()
            {
                action = Some(DebugAction::Step(self.cpu));
            }
            if ui.button("Image").on_hover_text("Une trame").clicked() {
                action = Some(DebugAction::FrameAdvance);
            }
        });
        ui.horizontal(|ui| {
            for t in Tab::ALL {
                ui.selectable_value(&mut self.tab, t, t.label());
            }
        });
        ui.separator();
        if self.snap.cpus.is_empty() && self.snap.registers.is_empty() {
            ui.weak("En attente de l'état de la carte…");
        }
        let a = match self.tab {
            Tab::Cpu => {
                self.cpu_tab(ui);
                None
            }
            Tab::Memory => self.memory_tab(ui),
            Tab::Registers => {
                self.video.registers_ui(ui, &self.snap);
                None
            }
            Tab::Video => self.video.video_ui(ui, &self.snap, self.generation),
        };
        action.or(a)
    }

    fn cpu_tab(&mut self, ui: &mut egui::Ui) {
        if self.snap.cpus.len() > 1 {
            ui.horizontal(|ui| {
                ui.label("Pas sur :");
                for (i, c) in self.snap.cpus.iter().enumerate() {
                    ui.radio_value(&mut self.cpu, i, &c.name);
                }
            });
            ui.separator();
        }
        for (i, c) in self.snap.cpus.iter().enumerate() {
            let head = format!("{}  PC={}", c.name, format_reg(c.pc, c.address_bits));
            if i == self.cpu {
                ui.strong(egui::RichText::new(head).color(egui::Color32::LIGHT_GREEN));
            } else {
                ui.strong(head);
            }
            // largeur d'affichage des registres : 16 bits pour un CPU 8 bits (bus 16 bits)
            let bits = if c.address_bits <= 16 { 16 } else { 32 };
            egui::Grid::new(("cpu", i))
                .num_columns(4)
                .striped(true)
                .show(ui, |ui| {
                    for (n, (name, v)) in c.registers.iter().enumerate() {
                        ui.monospace(name);
                        ui.monospace(format_reg(*v, bits));
                        if n % 2 == 1 {
                            ui.end_row();
                        }
                    }
                });
            ui.separator();
        }
    }

    fn memory_tab(&mut self, ui: &mut egui::Ui) -> Option<DebugAction> {
        let mut refresh = false;
        ui.horizontal(|ui| {
            for (i, c) in self.snap.cpus.iter().enumerate() {
                if ui.selectable_label(self.cpu == i, &c.name).clicked() {
                    self.cpu = i;
                    refresh = true;
                }
            }
            ui.label("Adresse");
            if ui.text_edit_singleline(&mut self.addr_text).lost_focus() {
                if let Some(a) = parse_addr(&self.addr_text) {
                    self.addr = a & !0xF;
                    refresh = true;
                }
            }
            if ui.button("Lire").clicked() {
                refresh = true;
            }
        });
        let bits = self.snap.cpus.get(self.cpu).map_or(24, |c| c.address_bits);
        for (l, line) in self.memory.chunks(16).enumerate() {
            let addr = self.addr.wrapping_add(16 * l as u32);
            let hex: Vec<String> = line.iter().map(|b| format!("{b:02X}")).collect();
            ui.monospace(format!(
                "{}  {}  {}",
                format_reg(addr, bits),
                hex.join(" "),
                ascii_col(line)
            ));
        }
        refresh.then(|| self.peek_request())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_format() {
        assert_eq!(format_reg(0x1234, 16), "1234");
        assert_eq!(format_reg(0x12, 24), "000012");
        assert_eq!(format_reg(0xFFFF_FFFF, 32), "FFFFFFFF");
        assert_eq!(format_reg(0x1_2345, 16), "2345");
    }

    #[test]
    fn memory_helpers() {
        assert_eq!(ascii_col(b"AB\x00z\x7f"), "AB.z.");
        assert_eq!(parse_addr("$FF8000"), Some(0xFF8000));
        assert_eq!(parse_addr("0x10"), Some(0x10));
        assert_eq!(parse_addr("zz"), None);
    }

    #[test]
    fn tab_names_round_trip() {
        for t in Tab::ALL {
            assert_eq!(Tab::from_name(t.name()), Some(t));
        }
    }
}
