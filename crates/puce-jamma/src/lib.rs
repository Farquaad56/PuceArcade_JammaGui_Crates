//! puce-jamma — contrat « connecteur JAMMA » entre cartes émulées et frontal.
//!
//! Ce crate définit les types et traits du connecteur JAMMA logiciel :
//! une carte émulée implémente `JammaBoard`, le frontal ne connaît que ce trait.
//! Le contrat est GELÉ (ADR-001) — toute modification passe par un ADR.

#![forbid(unsafe_code)]

pub mod audio;
pub mod board;
pub mod debug;
pub mod dips;
pub mod driver;
pub mod inputs;
pub mod util;
pub mod video;

pub use audio::*;
pub use board::*;
pub use debug::*;
pub use dips::*;
pub use driver::*;
pub use inputs::*;
pub use util::*;
pub use video::*;

#[cfg(test)]
mod smoke {
    #[test]
    fn it_builds() {}
}
