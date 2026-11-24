//! Panneaux et fenêtres de l'interface : menus, messages, barre d'état, débogueur, overlay.

use eframe::egui;

use crate::app::App;
use crate::debugger::DebugAction;
use crate::dips_menu;
use crate::emu_thread::Cmd;
use crate::menu::{self, MenuState};
use crate::overlay::{self, RunState};
use crate::statusbar::{format_status, StatusInfo};

/// Largeur initiale (points) du panneau de débogage.
pub(crate) const DEBUG_PANEL_W: f32 = 360.0;

impl App {
    pub(crate) fn status_text(&self) -> String {
        let scale = self.scale_info();
        format_status(&StatusInfo {
            system: Some(self.driver.system_name()),
            game: self.board.as_ref().map(|b| b.title.as_str()),
            resolution: self.board.as_ref().map(|b| (b.video.width, b.video.height)),
            refresh_hz: self.board.as_ref().map(|b| b.video.refresh_hz),
            ms: self.perf.avg(30),
            fps: Some(self.perf.fps),
            scale: Some(&scale),
            audio: self
                .board
                .as_ref()
                .map(|b| (b.audio.sample_rate, u32::from(b.audio.channels))),
            paused: self.paused,
            stopped: self.board.is_none(),
        })
    }

    pub(crate) fn windows(&mut self, ctx: &egui::Context) {
        self.mire_panel(ctx);
        if let Some(g) = self.games_win.show(ctx, &self.driver.games()) {
            self.load_game(&g);
        }
        if let Some(info) = self.board.clone() {
            if self.dips_win.show(ctx, &info, &mut self.dips) {
                self.send(Cmd::Dips(self.dips.clone()));
                if let Err(e) =
                    dips_menu::save_dips(&self.driver.saves_dir(), &info.name, &self.dips)
                {
                    self.error = Some(e);
                }
            }
            self.controls
                .show(ctx, &info, &mut self.bindings, &mut self.pads);
        }
        let mut fx = self.prefs.fx;
        let env = self.fx_env();
        if self.fx_win.show(ctx, &mut fx, &env) {
            self.prefs.fx = fx;
        }
        if let Some(a) = &self.about {
            self.about_win.show(ctx, a);
        }
    }

    pub(crate) fn panels(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let fullscreen = self.prefs.display.fullscreen;
        if !fullscreen {
            let scale_info = self.scale_info();
            let st = MenuState {
                paused: self.paused,
                loaded: self.board.is_some(),
                slot: self.slot,
                display: &self.prefs.display,
                scale_info: &scale_info,
                overlay: self.prefs.overlay,
                volume: self.prefs.audio.volume,
                mute: self.prefs.audio.mute,
                has_debugger: self.board.is_some(),
                debug_open: self.debugger.open,
                status_bar: self.prefs.debug.show_status_bar,
                has_about: self.about.is_some(),
            };
            let action = egui::Panel::top("menu")
                .show(ui, |ui| menu::show(ui, &st))
                .inner;
            if let Some(a) = action {
                self.handle(ctx, a);
            }
        }
        if let Some(e) = self.error.clone().or_else(|| self.info_msg.clone()) {
            egui::Panel::top("message").show(ui, |ui| {
                ui.horizontal(|ui| {
                    if ui.small_button("×").clicked() {
                        self.error = None;
                        self.info_msg = None;
                    }
                    let color = if self.error.is_some() {
                        egui::Color32::LIGHT_RED
                    } else {
                        egui::Color32::LIGHT_GREEN
                    };
                    ui.colored_label(color, e);
                });
            });
        }
        if !fullscreen && self.prefs.debug.show_status_bar {
            let text = self.status_text();
            egui::Panel::bottom("status").show(ui, |ui| ui.monospace(text));
        }
        if self.debugger.open && self.board.is_some() {
            let action = egui::Panel::right("debugger")
                .resizable(true)
                .default_size(DEBUG_PANEL_W)
                .show(ui, |ui| self.debugger.ui(ui, self.paused))
                .inner;
            self.debug_action(action);
        }
    }

    pub(crate) fn debug_action(&mut self, action: Option<DebugAction>) {
        match action {
            Some(DebugAction::Pause(p)) => self.set_paused(p),
            Some(DebugAction::Step(cpu)) => {
                if !self.paused {
                    self.set_paused(true);
                }
                self.send(Cmd::Step(cpu));
            }
            Some(DebugAction::FrameAdvance) => {
                if !self.paused {
                    self.set_paused(true);
                }
                self.send(Cmd::FrameAdvance);
            }
            Some(DebugAction::LayerMask(m)) => self.send(Cmd::LayerMask(m)),
            Some(DebugAction::VoiceMute(m)) => self.send(Cmd::VoiceMute(m)),
            Some(DebugAction::Peek { cpu, addr, len }) => self.send(Cmd::Peek { cpu, addr, len }),
            None => {}
        }
    }

    pub(crate) fn draw_overlay(&mut self, ctx: &egui::Context) {
        let state = match (self.board.is_some(), self.paused) {
            (false, _) => RunState::Stopped,
            (true, true) => RunState::Paused,
            (true, false) => RunState::Running,
        };
        let scale = self
            .layout
            .as_ref()
            .map(|d| format!("x{} {}x{}", d.factor, d.px.0, d.px.1));
        let Some(area) = self.central_rect else {
            return;
        };
        overlay::show(
            ctx,
            &self.prefs.overlay,
            &self.perf,
            state,
            scale.as_deref(),
            area,
        );
    }
}
