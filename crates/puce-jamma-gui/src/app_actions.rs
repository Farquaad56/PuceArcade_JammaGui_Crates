//! Actions des menus et des raccourcis.

use eframe::egui;

use crate::app::{App, SLOTS};
use crate::app_panels::DEBUG_PANEL_W;
use crate::emu_thread::Cmd;
use crate::menu::MenuAction;

impl App {
    pub(crate) fn handle(&mut self, ctx: &egui::Context, a: MenuAction) {
        let loaded = self.board.is_some();
        let d = &mut self.prefs.display;
        match a {
            MenuAction::OpenGames => self.games_win.open = true,
            MenuAction::Reset if loaded => self.send(Cmd::Reset),
            MenuAction::Pause(p) => self.set_paused(p),
            MenuAction::TogglePause => self.set_paused(!self.paused),
            MenuAction::FrameAdvance if loaded => {
                if !self.paused {
                    self.set_paused(true);
                }
                self.send(Cmd::FrameAdvance);
            }
            MenuAction::SaveState if loaded => self.send(Cmd::SaveState(self.slot)),
            MenuAction::LoadState if loaded => self.send(Cmd::LoadState(self.slot)),
            MenuAction::NextSlot => {
                self.slot = (self.slot + 1) % SLOTS;
                self.info_msg = Some(format!("emplacement d'état : {}", self.slot));
            }
            MenuAction::Screenshot if loaded => self.send(Cmd::Screenshot),
            MenuAction::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            MenuAction::Scale(s) => d.scale = s,
            MenuAction::Aspect(x) => d.aspect = x,
            MenuAction::Rotation(r) => d.rotation_override = r,
            MenuAction::Fullscreen(f) => d.fullscreen = f,
            MenuAction::ToggleFullscreen => d.fullscreen = !d.fullscreen,
            MenuAction::Smooth(s) => d.smooth = s,
            MenuAction::Bezels(b) => d.bezels = b,
            MenuAction::ToggleBezels => d.bezels = !d.bezels,
            MenuAction::BezelGlass(b) => d.bezel_glass = b,
            MenuAction::Overlay(o) => self.prefs.overlay.enabled = o,
            MenuAction::OverlayFull(f) => self.prefs.overlay.full = f,
            MenuAction::OverlayCorner(c) => self.prefs.overlay.corner = c,
            MenuAction::Filters => self.fx_win.open = true,
            MenuAction::ToggleMires => self.toggle_mires(),
            MenuAction::MireStep(d) if self.mire.state.active => self.mire.state.step(d),
            MenuAction::MireVariant(d) if self.mire.state.active => self.mire.state.step_variant(d),
            MenuAction::MirePanel if self.mire.state.active => {
                self.mire.state.panel = !self.mire.state.panel
            }
            MenuAction::MireStep(_) | MenuAction::MireVariant(_) | MenuAction::MirePanel => {}
            MenuAction::Dips => self.dips_win.open = true,
            MenuAction::Controls => self.controls.open = true,
            MenuAction::Volume(v) => self.prefs.audio.volume = v.min(100),
            MenuAction::Mute(m) => self.prefs.audio.mute = m,
            MenuAction::Debugger => {
                self.debugger.open = !self.debugger.open;
                // la fenêtre s'élargit (ou se rétrécit) de la largeur du panneau : l'image garde sa taille
                if !self.prefs.display.fullscreen {
                    if let Some(r) = ctx.input(|i| i.viewport().inner_rect) {
                        let dw = if self.debugger.open {
                            DEBUG_PANEL_W
                        } else {
                            -DEBUG_PANEL_W
                        };
                        let size = egui::vec2((r.width() + dw).max(320.0), r.height());
                        ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(size));
                    }
                }
                if self.debugger.open && loaded {
                    self.send(self.debugger.peek_cmd());
                }
            }
            MenuAction::StatusBar(b) => self.prefs.debug.show_status_bar = b,
            // actions sans effet tant qu'aucun jeu n'est chargé
            MenuAction::Reset
            | MenuAction::FrameAdvance
            | MenuAction::SaveState
            | MenuAction::LoadState
            | MenuAction::Screenshot => {}
        }
    }
}
