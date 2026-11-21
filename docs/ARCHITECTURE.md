# Architecture

```
PuceArcade_JammaGui_Crates/
  crates/
    puce-jamma            contrat « connecteur JAMMA » (docs/spec/00) — aucune dépendance (INTERFACE GELÉE)
    puce-jamma-testboard  carte de test (mire, réaction aux entrées, bip, DIP factices) : sert d'oracle au frontal
    puce-jamma-gui        frontal egui réutilisable : thread d'émulation, affichage + filtres (scalers 2x, CRT, masques), overlay,
                          son cpal, clavier/manettes -> JammaInputs, menus (jeux, DIP, filtres, commandes), savestates, captures, débogueur
  ref/puce16-neogeo/      GUI de Puce16 Neo Geo (même auteur) en lecture seule : modèle de TOUT le frontal
  docs/ spec/ adr/   tasks/ (cartes J*.md + board.json)   metrics/
```
Graphe : `puce-jamma` <- `puce-jamma-testboard` ; `puce-jamma` <- `puce-jamma-gui` (testboard en dev-dependency et dans l'exemple).
Utilisation par un émulateur (ex. PuceArcade CPS1) : dépendances git épinglées
`puce-jamma = { git = "https://github.com/Farquaad56/PuceArcade_JammaGui_Crates", package = "puce-jamma", rev = "<sha>" }` (idem `puce-jamma-gui`),
puis `puce_jamma_gui::run(driver, AppOptions { .. })` dans son `main.rs`.

## Frontal (puce-jamma-gui)
- API publique : `pub struct AppOptions { pub window_title: String, pub config_dir: PathBuf }` ; `pub fn run(driver: Box<dyn BoardDriver>, opts: AppOptions) -> Result<(), String>`.
- Thread d'émulation : seul propriétaire de la carte (`driver.load` y est appelé). Canal de commandes (crossbeam) : Load, Reset, Pause, Inputs, Dips,
  SaveState(slot), LoadState(slot), Step, LayerMask, Quit. Sortie : trames (pixels + VideoInfo), audio, statistiques, état de débogage à la demande.
- Affichage : image native -> rotation (util::rotate) -> mise à l'échelle (entière / ajustée, rapport `aspect`) -> filtres (pixel2x, postfx) -> texture.
- Préférences : `<config_dir>/prefs.toml` (affichage, filtres, overlay, son, commandes) ; savestates et captures : `driver.saves_dir()`.
- Les tailles d'image ne sont JAMAIS des constantes : tout vient de `VideoInfo` (Neo Geo : 320×224 ; CPS1 : 384×224 ; jeux verticaux tournés).

## Dépendances autorisées
puce-jamma : aucune. puce-jamma-gui : eframe/egui, cpal, gilrs, rfd, crossbeam-channel, png, toml. Tout ajout = ADR.
