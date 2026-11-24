//! Thread d'émulation : propriétaire de la carte (`driver.load` y est appelé, la carte n'est pas `Send`),
//! traite les commandes, cadence les trames à `frame_period(refresh_hz)` (rattrapage limité à 3 trames),
//! savestates `<saves_dir>/<jeu>.s<slot>`, `Event::Debug` après `Step` / `FrameAdvance`.

use std::path::PathBuf;
use std::sync::Arc;
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use crossbeam_channel::{unbounded, Receiver, RecvTimeoutError, Sender};
use puce_jamma::board::JammaBoard;
use puce_jamma::driver::BoardDriver;
use puce_jamma::inputs::JammaInputs;

use crate::capture;
use crate::emu_thread::{frame_period, step_once, Cmd, DebugSnapshot, Event, MAX_PEEK};

/// Nombre maximal de trames rattrapées d'un coup après un retard.
const MAX_CATCH_UP: u32 = 3;

/// Démarre le thread d'émulation.
pub fn spawn(driver: Arc<dyn BoardDriver>) -> (Sender<Cmd>, Receiver<Event>, JoinHandle<()>) {
    let (cmd_tx, cmd_rx) = unbounded::<Cmd>();
    let (ev_tx, ev_rx) = unbounded::<Event>();
    let join = thread::spawn(move || Emu::new(driver, ev_tx).run(cmd_rx));
    (cmd_tx, ev_rx, join)
}

/// Chemin du savestate d'un jeu.
pub fn state_path(saves_dir: &std::path::Path, game: &str, slot: u8) -> PathBuf {
    saves_dir.join(format!("{game}.s{slot}"))
}

struct Emu {
    driver: Arc<dyn BoardDriver>,
    tx: Sender<Event>,
    board: Option<Box<dyn JammaBoard>>,
    game: String,
    paused: bool,
    inputs: JammaInputs,
    period: Duration,
    next: Instant,
    /// Débogueur ouvert : instantanés réguliers.
    debug_live: bool,
    /// Onglet Son ouvert : voies envoyées régulièrement.
    voices_live: bool,
    /// Voies coupées, réappliquées à chaque chargement.
    voice_mute: u64,
}

/// En marche, débogueur ouvert : un instantané toutes les `LIVE_EVERY` trames.
const LIVE_EVERY: u64 = 10;

/// En marche, onglet Son ouvert : les voies toutes les `VOICES_EVERY` trames (≈ 30 par seconde).
const VOICES_EVERY: u64 = 2;

impl Emu {
    fn new(driver: Arc<dyn BoardDriver>, tx: Sender<Event>) -> Self {
        Emu {
            driver,
            tx,
            board: None,
            game: String::new(),
            paused: false,
            inputs: JammaInputs::default(),
            period: frame_period(60.0),
            next: Instant::now(),
            debug_live: false,
            voices_live: false,
            voice_mute: 0,
        }
    }

    fn send(&self, ev: Event) {
        // L'interface a pu se fermer : les événements sont alors simplement perdus.
        let _ = self.tx.send(ev);
    }

    fn run(mut self, rx: Receiver<Cmd>) {
        loop {
            let running = self.board.is_some() && !self.paused;
            let cmd = if running {
                let wait = self.next.saturating_duration_since(Instant::now());
                match rx.recv_timeout(wait) {
                    Ok(c) => Some(c),
                    Err(RecvTimeoutError::Timeout) => None,
                    Err(RecvTimeoutError::Disconnected) => return,
                }
            } else {
                match rx.recv() {
                    Ok(c) => Some(c),
                    Err(_) => return,
                }
            };
            match cmd {
                Some(Cmd::Quit) => return,
                Some(c) => self.handle(c),
                None => self.tick(),
            }
        }
    }

    /// Trames dues (au plus `MAX_CATCH_UP`), puis prochaine échéance.
    fn tick(&mut self) {
        let now = Instant::now();
        let mut n = 0;
        while self.next <= now && n < MAX_CATCH_UP {
            self.frame(false);
            self.next += self.period;
            n += 1;
        }
        if self.next <= now {
            // Trop de retard : on repart de maintenant au lieu d'accélérer.
            self.next = now + self.period;
        }
    }

    /// Une trame : image, son, statistiques (et instantané de débogage si demandé).
    fn frame(&mut self, with_debug: bool) {
        let Some(board) = self.board.as_mut() else {
            return;
        };
        let t0 = Instant::now();
        let (pixels, audio) = step_once(board.as_mut(), &self.inputs);
        let emu_ms = t0.elapsed().as_secs_f32() * 1000.0;
        let info = board.info().video;
        let frame = board.frame_count();
        let live = self.debug_live && frame % LIVE_EVERY == 0;
        let dbg = (with_debug || live).then(|| DebugSnapshot::of(board.as_mut()));
        self.send(Event::Frame {
            pixels,
            info,
            frame,
        });
        if !audio.is_empty() {
            self.send(Event::Audio(audio));
        }
        self.send(Event::Stats { emu_ms });
        if let Some(d) = dbg {
            self.send(Event::Debug(d));
        }
        if self.voices_live && (with_debug || frame % VOICES_EVERY == 0) {
            self.send_voices();
        }
    }

    /// Voies de la carte courante (rien si elle n'a pas de débogueur).
    fn send_voices(&mut self) {
        if let Some(d) = self.board.as_mut().and_then(|b| b.debug()) {
            let v = d.voices();
            self.send(Event::Voices(v));
        }
    }

    /// Applique l'état des voies (enregistrement, muets) à la carte courante.
    fn apply_voice_state(&mut self) {
        let (live, mute) = (self.voices_live, self.voice_mute);
        if let Some(d) = self.board.as_mut().and_then(|b| b.debug()) {
            d.set_voice_scope(live);
            d.set_voice_mute(mute);
        }
    }

    fn handle(&mut self, cmd: Cmd) {
        match cmd {
            Cmd::Load(name) => self.load(name),
            Cmd::Reset => {
                if let Some(b) = self.board.as_mut() {
                    b.reset();
                }
            }
            Cmd::Pause(p) => {
                self.paused = p;
                self.next = Instant::now();
                if p && self.debug_live {
                    self.send_snapshot();
                }
                if p && self.voices_live {
                    self.send_voices();
                }
            }
            Cmd::DebugLive(on) => {
                self.debug_live = on;
                if on {
                    self.send_snapshot();
                }
            }
            Cmd::VoicesLive(on) => {
                self.voices_live = on;
                self.apply_voice_state();
                if on {
                    self.send_voices();
                }
            }
            Cmd::VoiceMute(m) => {
                self.voice_mute = m;
                self.apply_voice_state();
            }
            Cmd::Inputs(i) => self.inputs = i,
            Cmd::Dips(d) => {
                if let Some(b) = self.board.as_mut() {
                    b.set_dips(&d);
                }
            }
            Cmd::SaveState(slot) => self.save_state(slot),
            Cmd::LoadState(slot) => self.load_state(slot),
            Cmd::Step(cpu) => self.step(cpu),
            Cmd::FrameAdvance => self.frame(true),
            Cmd::LayerMask(m) => {
                if let Some(d) = self.board.as_mut().and_then(|b| b.debug()) {
                    d.set_layer_mask(m);
                }
            }
            Cmd::Screenshot => self.screenshot(),
            Cmd::Peek { cpu, addr, len } => {
                let mem = self
                    .board
                    .as_mut()
                    .and_then(|b| b.debug())
                    .map_or_else(Vec::new, |d| {
                        (0..len.min(MAX_PEEK) as u32)
                            .map(|i| d.peek(cpu, addr.wrapping_add(i)))
                            .collect()
                    });
                self.send(Event::Memory(mem));
            }
            Cmd::Quit => {}
        }
    }

    fn load(&mut self, name: String) {
        self.board = None;
        match self.driver.load(&name) {
            Ok(b) => {
                let info = b.info().clone();
                self.period = frame_period(info.video.refresh_hz);
                self.next = Instant::now();
                self.game = name;
                self.board = Some(b);
                self.apply_voice_state();
                self.send(Event::Loaded(info));
                if self.debug_live {
                    self.send_snapshot();
                }
            }
            Err(e) => {
                self.game.clear();
                self.send(Event::LoadError(e));
            }
        }
    }

    /// Instantané de débogage de la carte courante.
    fn send_snapshot(&mut self) {
        if let Some(board) = self.board.as_mut() {
            let snap = DebugSnapshot::of(board.as_mut());
            self.send(Event::Debug(snap));
        }
    }

    /// Une instruction du CPU `cpu` si la carte a un débogueur, sinon une trame ; puis image et instantané.
    fn step(&mut self, cpu: usize) {
        let Some(board) = self.board.as_mut() else {
            return;
        };
        match board.debug() {
            Some(d) => d.step_instruction(cpu),
            None => {
                self.frame(true);
                return;
            }
        }
        let v = board.video();
        let (pixels, info) = (v.pixels.to_vec(), v.info);
        let frame = board.frame_count();
        let snap = DebugSnapshot::of(board.as_mut());
        self.send(Event::Frame {
            pixels,
            info,
            frame,
        });
        self.send(Event::Debug(snap));
        if self.voices_live {
            self.send_voices();
        }
    }

    fn save_state(&mut self, slot: u8) {
        let Some(board) = self.board.as_mut() else {
            return self.send(Event::StateError("aucun jeu chargé".into()));
        };
        let data = board.save_state();
        let dir = self.driver.saves_dir();
        let path = state_path(&dir, &self.game, slot);
        let res = std::fs::create_dir_all(&dir).and_then(|_| std::fs::write(&path, data));
        match res {
            Ok(()) => self.send(Event::Saved(slot)),
            Err(e) => self.send(Event::StateError(format!("{} : {e}", path.display()))),
        }
    }

    fn load_state(&mut self, slot: u8) {
        let path = state_path(&self.driver.saves_dir(), &self.game, slot);
        let Some(board) = self.board.as_mut() else {
            return self.send(Event::StateError("aucun jeu chargé".into()));
        };
        let res = std::fs::read(&path)
            .map_err(|e| format!("{} : {e}", path.display()))
            .and_then(|d| board.load_state(&d));
        if let Err(e) = res {
            self.send(Event::StateError(e));
        }
    }

    fn screenshot(&mut self) {
        let Some(board) = self.board.as_ref() else {
            return;
        };
        let v = board.video();
        let path =
            capture::screenshot_path(&self.driver.saves_dir(), &self.game, board.frame_count());
        match capture::write_png(&path, v.pixels, &v.info) {
            Ok(()) => self.send(Event::ScreenshotSaved(path)),
            Err(e) => self.send(Event::StateError(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use puce_jamma_testboard::driver::TestDriver;

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("jamma_emu_{tag}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    /// Attend le premier événement qui satisfait `f` (2 s au plus).
    fn wait_for(rx: &Receiver<Event>, f: impl Fn(&Event) -> bool) -> Event {
        let end = Instant::now() + Duration::from_secs(2);
        loop {
            let left = end.saturating_duration_since(Instant::now());
            let ev = rx.recv_timeout(left).expect("événement attendu non reçu");
            if f(&ev) {
                return ev;
            }
        }
    }

    #[test]
    fn load_step_save_quit() {
        let dir = temp_dir("main");
        let (tx, rx, join) = spawn(Arc::new(TestDriver::new(&dir)));
        tx.send(Cmd::Pause(true)).unwrap();
        tx.send(Cmd::Load("mire".into())).unwrap();
        match wait_for(&rx, |e| matches!(e, Event::Loaded(_))) {
            Event::Loaded(info) => assert_eq!(info.name, "mire"),
            _ => unreachable!(),
        }
        tx.send(Cmd::Step(0)).unwrap();
        match wait_for(&rx, |e| matches!(e, Event::Frame { .. })) {
            Event::Frame { frame, pixels, .. } => {
                assert_eq!(frame, 1);
                assert_eq!(pixels.len(), 288 * 224);
            }
            _ => unreachable!(),
        }
        assert!(matches!(
            wait_for(&rx, |e| matches!(e, Event::Debug(_))),
            Event::Debug(_)
        ));
        // En pause, aucune autre trame n'arrive.
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());

        tx.send(Cmd::SaveState(1)).unwrap();
        assert_eq!(
            wait_for(&rx, |e| matches!(e, Event::Saved(_))),
            Event::Saved(1)
        );
        assert!(dir.join("mire.s1").exists());
        tx.send(Cmd::LoadState(1)).unwrap();
        tx.send(Cmd::LoadState(7)).unwrap();
        assert!(matches!(
            wait_for(&rx, |e| matches!(e, Event::StateError(_))),
            Event::StateError(_)
        ));

        tx.send(Cmd::Screenshot).unwrap();
        match wait_for(&rx, |e| matches!(e, Event::ScreenshotSaved(_))) {
            Event::ScreenshotSaved(p) => assert!(p.ends_with("mire_1.png") && p.exists()),
            _ => unreachable!(),
        }

        tx.send(Cmd::Load("zz".into())).unwrap();
        assert!(matches!(
            wait_for(&rx, |e| matches!(e, Event::LoadError(_))),
            Event::LoadError(_)
        ));
        tx.send(Cmd::Quit).unwrap();
        join.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn runs_at_refresh_rate() {
        let dir = temp_dir("rate");
        let (tx, rx, join) = spawn(Arc::new(TestDriver::new(&dir)));
        tx.send(Cmd::Load("mire".into())).unwrap();
        wait_for(&rx, |e| matches!(e, Event::Loaded(_)));
        std::thread::sleep(Duration::from_millis(500));
        tx.send(Cmd::Quit).unwrap();
        join.join().unwrap();
        let frames = rx
            .try_iter()
            .filter(|e| matches!(e, Event::Frame { .. }))
            .count();
        // 60 Hz pendant ~0,5 s : environ 30 trames (marge pour une machine chargée).
        assert!((15..=40).contains(&frames), "{frames} trames");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn quit_when_interface_is_gone() {
        let dir = temp_dir("drop");
        let (tx, rx, join) = spawn(Arc::new(TestDriver::new(&dir)));
        drop(rx);
        tx.send(Cmd::Load("mire".into())).unwrap();
        drop(tx);
        join.join().unwrap();
    }

    #[test]
    fn debug_step_and_peek() {
        let dir = temp_dir("dbg");
        let (tx, rx, join) = spawn(Arc::new(TestDriver::new(&dir)));
        tx.send(Cmd::Pause(true)).unwrap();
        tx.send(Cmd::Load("mire".into())).unwrap();
        wait_for(&rx, |e| matches!(e, Event::Loaded(_)));
        tx.send(Cmd::Step(0)).unwrap();
        match wait_for(&rx, |e| matches!(e, Event::Debug(_))) {
            Event::Debug(d) => assert_eq!(d.cpus[0].pc, 1),
            _ => unreachable!(),
        }
        tx.send(Cmd::Peek {
            cpu: 0,
            addr: 0,
            len: 8,
        })
        .unwrap();
        match wait_for(&rx, |e| matches!(e, Event::Memory(_))) {
            Event::Memory(m) => assert_eq!(m, 1u64.to_le_bytes().to_vec()),
            _ => unreachable!(),
        }
        tx.send(Cmd::Quit).unwrap();
        join.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn layer_mask_hides_color_bars() {
        let dir = temp_dir("layers");
        let (tx, rx, join) = spawn(Arc::new(TestDriver::new(&dir)));
        tx.send(Cmd::Pause(true)).unwrap();
        tx.send(Cmd::Load("mire".into())).unwrap();
        wait_for(&rx, |e| matches!(e, Event::Loaded(_)));
        tx.send(Cmd::LayerMask(0b110)).unwrap();
        tx.send(Cmd::FrameAdvance).unwrap();
        match wait_for(&rx, |e| matches!(e, Event::Frame { .. })) {
            Event::Frame { pixels, .. } => assert_eq!(pixels[0], 0x202020),
            _ => unreachable!(),
        }
        tx.send(Cmd::Quit).unwrap();
        join.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn debug_live_sends_snapshots() {
        let dir = temp_dir("live");
        let (tx, rx, join) = spawn(Arc::new(TestDriver::new(&dir)));
        tx.send(Cmd::Load("mire".into())).unwrap();
        tx.send(Cmd::DebugLive(true)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut snaps = 0;
        while Instant::now() < deadline && snaps < 3 {
            if let Ok(Event::Debug(d)) = rx.recv_timeout(Duration::from_millis(100)) {
                assert!(!d.cpus.is_empty());
                snaps += 1;
            }
        }
        assert!(snaps >= 3, "instantanés réguliers en marche : {snaps}");
        tx.send(Cmd::Quit).unwrap();
        join.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn voices_live_sends_voices_and_keeps_mute() {
        let dir = temp_dir("voices");
        let (tx, rx, join) = spawn(Arc::new(TestDriver::new(&dir)));
        tx.send(Cmd::VoiceMute(1)).unwrap();
        tx.send(Cmd::VoicesLive(true)).unwrap();
        tx.send(Cmd::Load("mire".into())).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut n = 0;
        while Instant::now() < deadline && n < 5 {
            if let Ok(Event::Voices(v)) = rx.recv_timeout(Duration::from_millis(100)) {
                assert_eq!(v.len(), 3);
                assert_eq!(v[1].name, "Sinus");
                n += 1;
            }
        }
        assert!(n >= 5, "voies régulières en marche : {n}");
        // muet appliqué à la carte chargée après la commande : bouton 1 sans son
        let mut inputs = JammaInputs::default();
        inputs.players[0].buttons[0] = true;
        tx.send(Cmd::Inputs(inputs)).unwrap();
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut silent = 0;
        while Instant::now() < deadline && silent < 5 {
            if let Ok(Event::Audio(a)) = rx.recv_timeout(Duration::from_millis(100)) {
                assert!(a.iter().all(|s| *s == 0.0));
                silent += 1;
            }
        }
        assert!(silent >= 5);
        tx.send(Cmd::Quit).unwrap();
        join.join().unwrap();
        let _ = std::fs::remove_dir_all(&dir);
    }
}
