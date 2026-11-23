# Brancher une carte émulée sur le frontal JAMMA

Le frontal (`puce-jamma-gui`) ne connaît qu'un trait : `JammaBoard`. Un émulateur l'implémente pour sa carte,
fournit un `BoardDriver` (catalogue et chargement des jeux), puis appelle `run`. Contrat complet : `docs/spec/00-contrat-jamma.md`.

## 1. Dépendances (révision git épinglée)

```toml
[dependencies]
puce-jamma     = { git = "https://github.com/Farquaad56/PuceArcade_JammaGui_Crates", package = "puce-jamma",     rev = "<SHA>" }
puce-jamma-gui = { git = "https://github.com/Farquaad56/PuceArcade_JammaGui_Crates", package = "puce-jamma-gui", rev = "<SHA>" }
```

Toujours une révision (`rev`), jamais une branche : le contrat ne change qu'avec un ADR et une nouvelle version mineure.
Le SHA de chaque version est noté dans `docs/STATE.md` et dans `CHANGELOG.md`. `puce-jamma` n'a aucune dépendance ; seul le
binaire de l'émulateur dépend de `puce-jamma-gui` (le cœur de l'émulateur peut dépendre du seul contrat).

## 2. La carte : `JammaBoard`

```rust
use puce_jamma::*;

pub struct MaCarte { info: BoardInfo, machine: Machine /* … */ }

impl JammaBoard for MaCarte {
    fn info(&self) -> &BoardInfo { &self.info }
    fn reset(&mut self) { self.machine.reset() }
    fn run_frame(&mut self) { self.machine.run_frame() }            // exactement une trame
    fn frame_count(&self) -> u64 { self.machine.frame_count() }
    fn video(&self) -> VideoOut<'_> {                               // 0x00RRGGBB, width × height, AVANT rotation
        VideoOut { info: self.info.video, pixels: &self.machine.framebuffer().pixels }
    }
    fn take_audio(&mut self, out: &mut Vec<f32>) {                  // entrelacé selon `channels`, dans [-1, 1]
        out.extend(self.machine.take_audio());
    }
    fn set_inputs(&mut self, i: &JammaInputs) { self.machine.set_inputs(to_registres(i)) }
    fn dips(&self) -> Vec<u8> { self.machine.dips() }
    fn set_dips(&mut self, banks: &[u8]) { self.machine.set_dips(banks) }
    fn save_state(&mut self) -> Vec<u8> { self.machine.save_state() }
    fn load_state(&mut self, d: &[u8]) -> Result<(), String> { self.machine.load_state(d) }
    // fn debug(&mut self) -> Option<&mut dyn BoardDebug> : facultatif (débogueur générique du frontal)
}
```

- **Entrées** : `JammaInputs` est en logique ACTIVE HAUTE (`true` = appuyé). Les cartes réelles lisent souvent des bits actifs
  bas : `util::active_low(appuyé, bit)` renvoie 0 si appuyé, `bit` sinon. Exemple pour un port IN1 (joueur 1 dans l'octet bas) :
  ```rust
  let p = &i.players[0];
  let in1 = 0xFF
      & !(u8::from(p.right) << 0) & !(u8::from(p.left) << 1) & !(u8::from(p.down) << 2) & !(u8::from(p.up) << 3)
      & !(u8::from(p.buttons[0]) << 4) & !(u8::from(p.buttons[1]) << 5) & !(u8::from(p.buttons[2]) << 6);
  ```
- **`BoardInfo`** : `players` (1 à 4) et `buttons` (0 à 6) limitent ce que la fenêtre « Commandes » affiche ;
  `video.aspect` est le rapport du moniteur APRÈS rotation (4:3, ou 3:4 pour un jeu vertical) ; `video.rotation` suit les ROTxx de MAME.
- **DIP** : une `DipBank` par octet, une `DipSwitch` par réglage (masque, libellé, valeur par défaut, réglages possibles).
  Le frontal construit le menu, enregistre `<saves_dir>/<jeu>.dips` et appelle `set_dips`.
- La carte n'est pas `Send` : elle est créée et utilisée dans le thread d'émulation du frontal.

## 3. Le système : `BoardDriver`

```rust
pub struct MonDriver { roms: PathBuf, saves: PathBuf }

impl BoardDriver for MonDriver {
    fn system_name(&self) -> &str { "Mon système" }
    fn games(&self) -> Vec<GameEntry> { /* nom, titre, parent, available (ROM présentes), supported */ }
    fn default_game(&self) -> Option<String> { Some("monjeu".into()) }
    fn load(&self, name: &str) -> Result<Box<dyn JammaBoard>, String> { /* appelé DANS le thread d'émulation */ }
    fn saves_dir(&self) -> PathBuf { self.saves.clone() }   // savestates <jeu>.s<n>, captures <jeu>_<trame>.png, DIP <jeu>.dips
}
```

## 4. `main.rs`

```rust
use puce_jamma_gui::{run, AppOptions};

fn main() {
    let driver = MonDriver { roms: "roms".into(), saves: "saves".into() };
    // bezels_dir : dossier de bezels RetroArch (<jeu>.cfg + PNG), affichés en plein écran (Ctrl+B)
    let opts = AppOptions { window_title: "Mon émulateur".into(), config_dir: ".".into(), bezels_dir: Some("bezels".into()) };
    if let Err(e) = run(Box::new(driver), opts) {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
```

Au démarrage, le frontal relance le dernier jeu des préférences (`<config_dir>/prefs.toml`) s'il existe encore, sinon
`default_game()`. Un échec de chargement s'affiche dans la fenêtre, sans panique.

## 5. Exemple complet

La carte de test `puce-jamma-testboard` (mire, curseur, bip, DIP, savestate, débogueur) et son exemple :
`crates/puce-jamma-testboard/src/` et `crates/puce-jamma-gui/examples/testboard.rs`.

```
cargo run -p puce-jamma-gui --example testboard --release
```
