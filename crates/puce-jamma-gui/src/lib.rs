//! puce-jamma-gui — voir docs/ARCHITECTURE.md
#![forbid(unsafe_code)]

pub mod display;
pub mod pixel2x;
pub mod postfx;
pub mod prefs;

#[cfg(test)]
mod smoke {
    #[test]
    fn it_builds() {}
}
