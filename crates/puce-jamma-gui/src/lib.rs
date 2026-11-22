//! puce-jamma-gui — voir docs/ARCHITECTURE.md
#![forbid(unsafe_code)]

pub mod audio;
pub mod bindings;
pub mod capture;
pub mod display;
pub mod emu_loop;
pub mod emu_thread;
pub mod gamepad;
pub mod input;
pub mod overlay;
pub mod pixel2x;
pub mod postfx;
pub mod prefs;
pub mod resample;
pub mod statusbar;

#[cfg(test)]
mod smoke {
    #[test]
    fn it_builds() {}
}
