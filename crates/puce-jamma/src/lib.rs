//! puce-jamma — contrat « connecteur JAMMA » entre cartes émulées et frontal.
//!
//! Ce crate définit les types et traits du connecteur JAMMA logiciel :
//! une carte émulée implémente `JammaBoard`, le frontal ne connaît que ce trait.
//! Le contrat est GELÉ (ADR-001) — toute modification passe par un ADR.

#![forbid(unsafe_code)]

pub mod audio;
pub mod dips;
pub mod inputs;
pub mod video;

pub use audio::*;
pub use dips::*;
pub use inputs::*;
pub use video::*;

#[cfg(test)]
mod smoke {
    #[test]
    fn it_builds() {}
}
