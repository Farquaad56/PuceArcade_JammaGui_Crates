//! Interface egui (reprise de ref/puce16-neogeo/ng-gui/app.rs, sur le contrat JAMMA) : envoie les
//! commandes à chaque image, reçoit les trames du thread d'émulation, applique rotation, filtres et
//! mise en page, joue le son, dessine menus, fenêtres, overlay et barre d'état.

use std::sync::Arc;
use std::time::Instant;

use crossbeam_channel::{Receiver, Sender};
use eframe::egui;
use puce_jamma::board::BoardInfo;
use puce_jamma::driver::BoardDriver;
use puce_jamma::util::rotate;
use puce_jamma::video::VideoInfo;

use crate::audio::{self, AudioOut};
use crate::bezel;
use crate::bindings::Bindings;
use crate::controls_menu::ControlsWindow;
use crate::debugger::Debugger;
use crate::dips_menu::{self, DipsWindow};
use crate::display::{self, DisplayRect};
use crate::emu_loop;
use crate::emu_thread::{Cmd, Event};
use crate::fx_menu::FxWindow;
use crate::gamepad::Gamepads;
use crate::games::GamesWindow;
use crate::input::InputState;
use crate::menu::{self, MenuAction};
use crate::overlay::Perf;
use crate::postfx::{self, Fx};
use crate::prefs::{Aspect, Prefs};
use crate::resample::AudioResampler;

/// Taille de la fenêtre avant le premier chargement (points) ; ajustée ensuite par `initial_window_size`.
pub const DEFAULT_WINDOW: [f32; 2] = [960.0, 720.0];
/// Nombre d'emplacements de savestate (F8 fait le tour).
pub const SLOTS: u8 = 10;

/// `VideoInfo` affichée : rotation forcée par les préférences, sinon celle de la carte.
pub fn effective_info(
    info: &VideoInfo,
    rotation_override: Option<puce_jamma::video::Rotation>,
) -> VideoInfo {
    VideoInfo {
        rotation: rotation_override.unwrap_or(info.rotation),
        ..*info
    }
}

/// Taille initiale de la fenêtre (points) : 3 × la taille au facteur 1 avec le rapport du moniteur.
pub fn initial_window_size(info: &VideoInfo) -> (f32, f32) {
    let (w, h) = display::display_size(info, Aspect::Tv);
    (3.0 * w as f32, 3.0 * h as f32)
}

/// Convertit une image 0x00RRGGBB en image egui.
pub fn to_color_image(pixels: &[u32], w: usize, h: usize) -> egui::ColorImage {
    let px = pixels
        .iter()
        .map(|p| egui::Color32::from_rgb((p >> 16) as u8, (p >> 8) as u8, *p as u8))
        .collect();
    egui::ColorImage {
        size: [w, h],
        source_size: egui::vec2(w as f32, h as f32),
        pixels: px,
    }
}

/// Dernière trame, déjà tournée.
struct Shown {
    pixels: Vec<u32>,
    w: usize,
    h: usize,
    frame: u64,
}

/// Ce qui détermine l'image filtrée en cache.
#[derive(Clone, Copy, PartialEq)]
struct FxKey {
    frame: u64,
    px: (u32, u32),
    fx: Fx,
}

/// Son : sortie du périphérique et rééchantillonneur (absent sans périphérique).
struct Sound {
    out: AudioOut,
    resampler: AudioResampler,
    base_hz: f64,
    channels: u8,
}

pub struct App {
    pub(crate) driver: Arc<dyn BoardDriver>,
    tx: Sender<Cmd>,
    rx: Receiver<Event>,
    pub(crate) prefs: Prefs,
    saved: Prefs,
    prefs_path: std::path::PathBuf,
    pub(crate) bindings: Bindings,
    input: InputState,
    pub(crate) pads: Gamepads,
    pub(crate) board: Option<BoardInfo>,
    pub(crate) dips: Vec<u8>,
    raw: Option<(Vec<u32>, VideoInfo, u64)>,
    shown: Option<Shown>,
    texture: Option<egui::TextureHandle>,
    fx_tex: Option<(FxKey, egui::TextureHandle)>,
    pub(crate) layout: Option<DisplayRect>,
    /// Zone centrale (image) de la dernière trame affichée : l'overlay s'y place.
    pub(crate) central_rect: Option<egui::Rect>,
    /// Bezels RetroArch (plein écran).
    bezels_dir: Option<std::path::PathBuf>,
    bezel: BezelState,
    /// Dernier état « débogueur ouvert » transmis au thread d'émulation.
    debug_live_sent: bool,
    sound: Option<Sound>,
    pub(crate) paused: bool,
    pub(crate) slot: u8,
    pub(crate) error: Option<String>,
    pub(crate) info_msg: Option<String>,
    pub(crate) perf: Perf,
    frames_seen: Vec<Instant>,
    fullscreen_applied: Option<bool>,
    /// Taille de fenêtre à appliquer après le premier chargement.
    resize_to: Option<(f32, f32)>,
    sized: bool,
    pub(crate) games_win: GamesWindow,
    pub(crate) dips_win: DipsWindow,
    pub(crate) fx_win: FxWindow,
    pub(crate) controls: ControlsWindow,
    pub(crate) debugger: Debugger,
}

impl App {
    /// Démarre le thread d'émulation et charge `game` s'il est donné.
    pub fn new(
        driver: Arc<dyn BoardDriver>,
        config_dir: &std::path::Path,
        game: Option<String>,
    ) -> Self {
        let prefs_path = crate::prefs::prefs_path(config_dir);
        let prefs = Prefs::load(&prefs_path);
        let (tx, rx, _join) = emu_loop::spawn(Arc::clone(&driver));
        let (sound, error) = match AudioOut::new() {
            Ok(out) => (Some(out), None),
            Err(e) => (None, Some(format!("son désactivé : {e}"))),
        };
        let mut app = App {
            driver,
            tx,
            rx,
            bindings: Bindings::from_prefs(&prefs.input),
            saved: prefs.clone(),
            debugger: Debugger::new(prefs.debug.open, &prefs.debug.tab),
            prefs,
            prefs_path,
            input: InputState::default(),
            pads: Gamepads::default(),
            board: None,
            dips: Vec::new(),
            raw: None,
            shown: None,
            texture: None,
            fx_tex: None,
            layout: None,
            central_rect: None,
            bezels_dir: None,
            bezel: BezelState::default(),
            debug_live_sent: false,
            sound: sound.map(|out| Sound {
                resampler: AudioResampler::new(48_000.0, out.rate),
                out,
                base_hz: 48_000.0,
                channels: 1,
            }),
            paused: false,
            slot: 0,
            error,
            info_msg: None,
            perf: Perf::new(60.0),
            frames_seen: Vec::new(),
            fullscreen_applied: None,
            resize_to: None,
            sized: false,
            games_win: GamesWindow::default(),
            dips_win: DipsWindow::default(),
            fx_win: FxWindow::default(),
            controls: ControlsWindow::default(),
        };
        if let Some(g) = game {
            app.load_game(&g);
        }
        app
    }

    pub(crate) fn send(&self, cmd: Cmd) {
        // le thread d'émulation ne s'arrête que sur Quit : une erreur ici est sans conséquence
        let _ = self.tx.send(cmd);
    }

    pub(crate) fn load_game(&mut self, name: &str) {
        self.board = None;
        self.raw = None;
        self.shown = None;
        self.send(Cmd::Load(name.to_string()));
    }

    pub(crate) fn set_paused(&mut self, p: bool) {
        self.paused = p;
        self.send(Cmd::Pause(p));
        if let Some(s) = &self.sound {
            s.out.set_active(!p && self.board.is_some());
        }
    }

    fn on_loaded(&mut self, info: BoardInfo) {
        self.dips = dips_menu::load_dips(&self.driver.saves_dir(), &info);
        self.send(Cmd::Dips(self.dips.clone()));
        self.perf = Perf::new(info.video.refresh_hz);
        if let Some(s) = self.sound.as_mut() {
            s.base_hz = info.audio.sample_rate;
            s.channels = info.audio.channels;
            s.resampler = AudioResampler::new(info.audio.sample_rate, s.out.rate);
            s.out.set_active(!self.paused);
        }
        self.prefs.last_game = Some(info.name.clone());
        if !self.sized {
            let v = effective_info(&info.video, self.prefs.display.rotation_override);
            let (w, h) = initial_window_size(&v);
            let extra = if self.debugger.open {
                crate::app_panels::DEBUG_PANEL_W
            } else {
                0.0
            };
            self.resize_to = Some((w + extra, h));
            self.sized = true;
        }
        self.error = None;
        self.board = Some(info);
        if self.paused {
            self.send(Cmd::Pause(true));
        }
    }

    /// Lit les événements du thread d'émulation.
    fn poll(&mut self) {
        while let Ok(ev) = self.rx.try_recv() {
            match ev {
                Event::Loaded(info) => self.on_loaded(info),
                Event::LoadError(e) => self.error = Some(format!("chargement impossible : {e}")),
                Event::Frame {
                    pixels,
                    info,
                    frame,
                } => {
                    self.frames_seen.push(Instant::now());
                    self.raw = Some((pixels, info, frame));
                }
                Event::Audio(a) => self.play(&a),
                Event::Stats { emu_ms } => self.perf.push_emu(emu_ms),
                Event::Saved(s) => self.info_msg = Some(format!("état {s} sauvegardé")),
                Event::StateError(e) => self.error = Some(e),
                Event::ScreenshotSaved(p) => {
                    self.info_msg = Some(format!("capture : {}", p.display()))
                }
                Event::Debug(d) => {
                    self.debugger.set_snapshot(d);
                    // onglet Mémoire : relu à chaque nouvel instantané
                    if self.debugger.open && self.debugger.tab == crate::debugger::Tab::Memory {
                        let cmd = self.debugger.peek_cmd();
                        self.send(cmd);
                    }
                }
                Event::Memory(m) => self.debugger.set_memory(m),
            }
        }
        let now = Instant::now();
        self.frames_seen
            .retain(|t| now.duration_since(*t).as_secs_f32() < 1.0);
        self.perf.fps = self.frames_seen.len() as f32;
    }

    /// Rééchantillonne et joue le son reçu ; régule la dérive en ajustant la fréquence source.
    fn play(&mut self, samples: &[f32]) {
        let Some(s) = self.sound.as_mut() else { return };
        let fill = s.out.fill();
        s.resampler
            .set_src_hz(s.base_hz * (1.0 + audio::drc_adjust(fill)));
        s.resampler.push_interleaved(samples, s.channels);
        let mut out = Vec::new();
        s.resampler.pop_into(&mut out);
        let gain = audio::volume_gain(self.prefs.audio.volume, self.prefs.audio.mute);
        out.iter_mut().for_each(|v| *v *= gain);
        s.out.push(&out);
        self.perf.set_audio(fill, s.out.underruns());
    }

    /// Tourne la dernière trame reçue et met à jour la texture.
    fn update_texture(&mut self, ctx: &egui::Context) {
        let Some((pixels, info, frame)) = self.raw.take() else {
            return;
        };
        let info = effective_info(&info, self.prefs.display.rotation_override);
        let (img, w, h) = rotate(&pixels, info.width, info.height, info.rotation);
        let (w, h) = (w as usize, h as usize);
        let opts = if self.prefs.display.smooth {
            egui::TextureOptions::LINEAR
        } else {
            egui::TextureOptions::NEAREST
        };
        let ci = to_color_image(&img, w, h);
        match &mut self.texture {
            Some(t) => t.set(ci, opts),
            None => self.texture = Some(ctx.load_texture("jamma_fb", ci, opts)),
        }
        self.shown = Some(Shown {
            pixels: img,
            w,
            h,
            frame,
        });
    }

    /// Image filtrée à la taille finale (recalculée si la trame, la taille ou le réglage changent).
    fn filtered_texture(
        &mut self,
        ctx: &egui::Context,
        l: &DisplayRect,
    ) -> Option<egui::TextureId> {
        let fx = self.prefs.fx;
        let s = self.shown.as_ref()?;
        if !fx.is_active() {
            self.perf.fx_ms = None;
            return None;
        }
        let key = FxKey {
            frame: s.frame,
            px: l.px,
            fx,
        };
        if self.fx_tex.as_ref().map(|(k, _)| *k) != Some(key) {
            let (w, h) = (l.px.0 as usize, l.px.1 as usize);
            let t0 = Instant::now();
            let (frame, sw, sh) = (s.frame, s.w, s.h);
            let pixels = postfx::render(&s.pixels, sw, sh, &fx, w, h, frame);
            let img = egui::ColorImage {
                size: [w, h],
                source_size: egui::vec2(w as f32, h as f32),
                pixels,
            };
            self.perf.smooth_fx(t0.elapsed().as_secs_f32() * 1000.0);
            match &mut self.fx_tex {
                Some((k, t)) => {
                    t.set(img, egui::TextureOptions::NEAREST);
                    *k = key;
                }
                None => {
                    let t = ctx.load_texture("jamma_fx", img, egui::TextureOptions::NEAREST);
                    self.fx_tex = Some((key, t));
                }
            }
        }
        self.fx_tex.as_ref().map(|(_, t)| t.id())
    }

    /// Dessine la trame dans la zone centrale.
    /// Dossier des bezels RetroArch (None : pas de bezels).
    pub fn with_bezels(mut self, dir: Option<std::path::PathBuf>) -> Self {
        self.bezels_dir = dir;
        self
    }

    /// Bezel du jeu courant si le plein écran et l'option sont actifs : chargé en arrière-plan
    /// (PNG 4K), puis gardé en texture. Retourne sa texture et sa géométrie quand il est prêt.
    fn current_bezel(&mut self, ctx: &egui::Context) -> Option<(egui::TextureId, bezel::Geometry)> {
        let d = &self.prefs.display;
        let (Some(dir), Some(b)) = (self.bezels_dir.clone(), self.board.as_ref()) else {
            return None;
        };
        if !(d.fullscreen && d.bezels) {
            return None;
        }
        let name = b.name.clone();
        if self.bezel.game.as_deref() != Some(name.as_str()) {
            let parent = self
                .driver
                .games()
                .into_iter()
                .find(|g| g.name == name)
                .map(|g| g.parent)
                .unwrap_or_default();
            self.bezel = BezelState {
                game: Some(name.clone()),
                ..BezelState::default()
            };
            if let Some(cfg) = bezel::find_cfg(&dir, &name, &parent) {
                let (tx, rx) = crossbeam_channel::bounded(1);
                let c = ctx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(bezel::load(&cfg));
                    c.request_repaint();
                });
                self.bezel.rx = Some(rx);
            }
        }
        if let Some(rx) = &self.bezel.rx {
            if let Ok(r) = rx.try_recv() {
                self.bezel.rx = None;
                match r {
                    Ok(bz) => {
                        let img = egui::ColorImage::from_rgba_unmultiplied([bz.w, bz.h], &bz.rgba);
                        let tex = ctx.load_texture("bezel", img, egui::TextureOptions::LINEAR);
                        self.bezel.ready = Some((tex, bz.geometry()));
                    }
                    Err(e) => self.error = Some(format!("Bezel : {e}")),
                }
            }
        }
        self.bezel.ready.as_ref().map(|(t, g)| (t.id(), *g))
    }

    fn draw_frame(&mut self, ui: &mut egui::Ui) {
        let avail = ui.max_rect();
        self.central_rect = Some(avail);
        if self.board.is_none() || self.texture.is_none() {
            self.layout = None;
            ui.centered_and_justified(|ui| ui.label("Fichier > Ouvrir un jeu…"));
            return;
        }
        let bezel = self.current_bezel(ui.ctx());
        let (Some(t), Some(b)) = (self.texture.as_ref().map(|t| t.id()), self.board.as_ref())
        else {
            return;
        };
        let d = &self.prefs.display;
        let info = effective_info(&b.video, d.rotation_override);
        let ppp = ui.pixels_per_point();
        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
        let l = match bezel {
            // plein écran avec bezel : l'image remplit le trou, aux proportions de l'écran du jeu
            Some((_, g)) => {
                let (_, hole) = bezel::layout(avail, &g);
                let rect = bezel::fit(hole, info.aspect);
                let px = (
                    (rect.width() * ppp).round() as u32,
                    (rect.height() * ppp).round() as u32,
                );
                DisplayRect {
                    rect,
                    factor: (px.1 / info.height.max(1)).max(1),
                    px,
                }
            }
            None => display::layout(avail, ppp, d.scale, d.aspect, &info),
        };
        let tex = self.filtered_texture(ui.ctx(), &l).unwrap_or(t);
        let painter = ui.painter_at(avail);
        painter.image(tex, l.rect, uv, egui::Color32::WHITE);
        if let Some((bt, g)) = bezel {
            let (r, _) = bezel::layout(avail, &g);
            painter.image(bt, r, uv, egui::Color32::WHITE);
        }
        self.layout = Some(l);
    }

    pub(crate) fn scale_info(&self) -> String {
        self.layout
            .as_ref()
            .map_or_else(|| "–".into(), display::scale_label)
    }

    /// Commandes du clavier et des manettes vers le jeu (rien pendant une capture de touche).
    fn send_inputs(&mut self, ctx: &egui::Context) {
        self.input.pad_pressed = self.pads.poll(&self.bindings);
        if self.controls.capturing() || ctx.egui_wants_keyboard_input() {
            self.send(Cmd::Inputs(Default::default()));
            return;
        }
        self.input.set_keys(ctx.input(|i| i.keys_down.clone()));
        self.send(Cmd::Inputs(self.input.to_jamma(&self.bindings)));
    }

    fn shortcuts(&mut self, ctx: &egui::Context) {
        if self.controls.capturing() {
            return;
        }
        let actions: Vec<MenuAction> = ctx.input(|i| {
            i.events
                .iter()
                .filter_map(|e| match e {
                    egui::Event::Key {
                        key,
                        pressed: true,
                        repeat: false,
                        modifiers,
                        ..
                    } => menu::shortcut(*key, *modifiers),
                    _ => None,
                })
                .collect()
        });
        for a in actions {
            self.handle(ctx, a);
        }
        if self.prefs.display.fullscreen && ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            self.prefs.display.fullscreen = false;
        }
    }

    fn apply_fullscreen(&mut self, ctx: &egui::Context) {
        if let Some((w, h)) = self.resize_to.take() {
            if !self.prefs.display.fullscreen {
                ctx.send_viewport_cmd(egui::ViewportCommand::InnerSize(egui::vec2(w, h)));
            }
        }
        let want = self.prefs.display.fullscreen;
        if self.fullscreen_applied != Some(want) {
            if want || self.fullscreen_applied.is_some() {
                ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(want));
            }
            self.fullscreen_applied = Some(want);
        }
    }

    /// Sauvegarde les préférences quand elles ont changé (une seule tentative par changement).
    fn save_prefs_if_changed(&mut self) {
        self.prefs.input = self.bindings.to_prefs(self.prefs.input.mouse_sens);
        self.prefs.debug.open = self.debugger.open;
        self.prefs.debug.tab = self.debugger.tab.name().to_string();
        if self.prefs == self.saved {
            return;
        }
        if let Err(e) = self.prefs.save(&self.prefs_path) {
            self.error = Some(format!("préférences non sauvegardées : {e}"));
        }
        self.saved = self.prefs.clone();
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll();
        self.update_texture(&ctx);
        self.shortcuts(&ctx);
        self.apply_fullscreen(&ctx);
        self.send_inputs(&ctx);
        self.perf.smooth_ui(ctx.input(|i| i.unstable_dt) * 1000.0);
        self.windows(&ctx);
        self.panels(ui, &ctx);
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(egui::Color32::BLACK))
            .show(ui, |ui| self.draw_frame(ui));
        self.draw_overlay(&ctx);
        let live = self.debugger.open && self.board.is_some();
        if live != self.debug_live_sent {
            self.send(Cmd::DebugLive(live));
            self.debug_live_sent = live;
        }
        self.save_prefs_if_changed();
        ctx.request_repaint();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        self.save_prefs_if_changed();
        self.send(Cmd::Quit);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma::video::Rotation;

    fn vi(w: u32, h: u32, aspect: (u32, u32), rotation: Rotation) -> VideoInfo {
        VideoInfo {
            width: w,
            height: h,
            refresh_hz: 60.0,
            aspect,
            rotation,
        }
    }

    #[test]
    fn initial_window_sizes() {
        assert_eq!(
            initial_window_size(&vi(288, 224, (4, 3), Rotation::Rot0)),
            (864.0, 648.0)
        );
        assert_eq!(
            initial_window_size(&vi(384, 224, (4, 3), Rotation::Rot0)),
            (1152.0, 864.0)
        );
        assert_eq!(
            initial_window_size(&vi(288, 224, (3, 4), Rotation::Rot270)),
            (672.0, 897.0)
        );
    }

    #[test]
    fn rotation_override() {
        let i = vi(288, 224, (4, 3), Rotation::Rot0);
        assert_eq!(effective_info(&i, None).rotation, Rotation::Rot0);
        assert_eq!(
            effective_info(&i, Some(Rotation::Rot90)).rotation,
            Rotation::Rot90
        );
        assert_eq!(effective_info(&i, Some(Rotation::Rot90)).width, 288);
    }

    #[test]
    fn pixel_conversion() {
        let mut px = vec![0u32; 4 * 2];
        px[0] = 0x00FF_8040;
        let img = to_color_image(&px, 4, 2);
        assert_eq!(img.size, [4, 2]);
        assert_eq!(img.pixels[0], egui::Color32::from_rgb(255, 128, 64));
    }
}

/// Bezel du jeu courant : chargement en cours (`rx`) ou texture prête.
#[derive(Default)]
struct BezelState {
    /// Jeu pour lequel le bezel a été demandé.
    game: Option<String>,
    rx: Option<Receiver<Result<bezel::Bezel, String>>>,
    ready: Option<(egui::TextureHandle, bezel::Geometry)>,
}
