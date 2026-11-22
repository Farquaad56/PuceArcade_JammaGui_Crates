//! puce-jamma-gui — voir docs/ARCHITECTURE.md
#![forbid(unsafe_code)]

pub mod capture;
pub mod display;
pub mod emu_thread;
pub mod overlay;
pub mod pixel2x;
pub mod postfx;
pub mod prefs;
pub mod statusbar;

#[cfg(test)]
mod smoke {
    #[test]
    fn it_builds() {}
}
