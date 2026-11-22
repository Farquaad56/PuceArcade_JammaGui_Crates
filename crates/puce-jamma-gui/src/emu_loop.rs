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
use crate::emu_thread::{frame_period, step_once, Cmd, DebugSnapshot, Event};

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
}

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
        let dbg = with_debug.then(|| DebugSnapshot::of(board.as_mut()));
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
            }
            Cmd::Inputs(i) => self.inputs = i,
            Cmd::Dips(d) => {
                if let Some(b) = self.board.as_mut() {
                    b.set_dips(&d);
                }
            }
            Cmd::SaveState(slot) => self.save_state(slot),
            Cmd::LoadState(slot) => self.load_state(slot),
            Cmd::Step => self.step(),
            Cmd::FrameAdvance => self.frame(true),
            Cmd::LayerMask(m) => {
                if let Some(d) = self.board.as_mut().and_then(|b| b.debug()) {
                    d.set_layer_mask(m);
                }
            }
            Cmd::Screenshot => self.screenshot(),
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
                self.send(Event::Loaded(info));
            }
            Err(e) => {
                self.game.clear();
                self.send(Event::LoadError(e));
            }
        }
    }

    /// Une instruction du CPU 0 si la carte a un débogueur, sinon une trame ; puis image et instantané.
    fn step(&mut self) {
        let Some(board) = self.board.as_mut() else {
            return;
        };
        match board.debug() {
            Some(d) => d.step_instruction(0),
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
        let path = capture::screenshot_path(&self.driver.saves_dir(), &self.game, board.frame_count());
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
        tx.send(Cmd::Step).unwrap();
        match wait_for(&rx, |e| matches!(e, Event::Frame { .. })) {
            Event::Frame { frame, pixels, .. } => {
                assert_eq!(frame, 1);
                assert_eq!(pixels.len(), 288 * 224);
            }
            _ => unreachable!(),
        }
        assert!(matches!(wait_for(&rx, |e| matches!(e, Event::Debug(_))), Event::Debug(_)));
        // En pause, aucune autre trame n'arrive.
        assert!(rx.recv_timeout(Duration::from_millis(100)).is_err());

        tx.send(Cmd::SaveState(1)).unwrap();
        assert_eq!(wait_for(&rx, |e| matches!(e, Event::Saved(_))), Event::Saved(1));
        assert!(dir.join("mire.s1").exists());
        tx.send(Cmd::LoadState(1)).unwrap();
        tx.send(Cmd::LoadState(7)).unwrap();
        assert!(matches!(wait_for(&rx, |e| matches!(e, Event::StateError(_))), Event::StateError(_)));

        tx.send(Cmd::Screenshot).unwrap();
        match wait_for(&rx, |e| matches!(e, Event::ScreenshotSaved(_))) {
            Event::ScreenshotSaved(p) => assert!(p.ends_with("mire_1.png") && p.exists()),
            _ => unreachable!(),
        }

        tx.send(Cmd::Load("zz".into())).unwrap();
        assert!(matches!(wait_for(&rx, |e| matches!(e, Event::LoadError(_))), Event::LoadError(_)));
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
        let frames = rx.try_iter().filter(|e| matches!(e, Event::Frame { .. })).count();
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
}
