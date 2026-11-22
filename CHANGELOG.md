# Changelog

## v0.2.0

- `puce-jamma-gui`, fenêtre « Filtres vidéo », nouvelle section **Tube CRT** (réglages indépendants des préréglages
  d'écran, sauvegardés dans `prefs.toml`, section `[fx]`) :
  - canaux RVB : gain du rouge, du vert et du bleu (0 à 200 %) ;
  - convergence : décalage du rouge et du bleu par rapport au vert (−8 à 8 px, X et Y), boutons « Convergence
    parfaite » et « Tube fatigué » ;
  - barre de balayage : bande claire qui descend l'écran avec une traînée sombre, intensité, durée d'un passage
    (0,5 à 10 s) et hauteur réglables.
  - faisceau et phosphore : faisceau au ralenti (0,1 à 10 s par image) qui redessine l'image ligne par ligne ;
    chaque ligne garde l'image de son dernier passage et s'éteint selon la persistance du phosphore, jusqu'à une
    luminosité minimale réglable ; le jeu continue en temps réel ;
  - rémanence entre images (0 à 90 %) : traînées derrière les objets qui bougent.
- API : `Fx` gagne `red`, `green`, `blue`, `conv_rx`, `conv_ry`, `conv_bx`, `conv_by`, `sweep`, `sweep_speed`,
  `sweep_width`, `beam`, `phosphor`, `beam_floor`, `ghost` ; `postfx::Phosphor` (état entre deux images,
  `apply` après `render`) ; `postfx::render` prend le numéro de trame (position de la barre) ; `postfx::sweep_gains`,
  `Fx::tube_active`, `Fx::without_tube`, `Fx::with_tube_of`.
- Exemple `fx_demo` : aperçu des filtres sur une image PNG.

## v0.1.0

Commit `311b8eccc5e6b1204704ecf985ef785b8ecec33c` (tag `v0.1.0`).

Première version du contrat et du frontal.

- `puce-jamma` (contrat gelé v0.1) : `JammaBoard`, `BoardDebug`, `BoardDriver`, `JammaInputs` (4 joueurs × 6 boutons, 4 pièces,
  service, test, tilt, analogique), `VideoInfo` (rotation, rapport du moniteur), `AudioInfo`, DIP décrits ;
  utilitaires `rotate`, `default_dips`, `dip_value`, `set_dip`, `active_low`.
- `puce-jamma-testboard` : mire horizontale et verticale, curseur, bip, DIP, savestate, débogueur.
- `puce-jamma-gui` : repris du GUI de Puce16 Neo Geo et généralisé à toute taille d'image et toute rotation :
  placement entier ou ajusté, scalers 2x, filtres CRT et masques, overlay de performances, barre d'état, préférences
  (`prefs.toml`), thread d'émulation cadencé à `refresh_hz` (rattrapage limité à 3 trames), son cpal au format natif
  du périphérique avec rééchantillonnage et régulation de dérive, 51 actions JAMMA (clavier et 4 manettes), fenêtre
  « Commandes » filtrée par la carte, menus DIP générés, savestates (10 emplacements), captures PNG, débogueur générique
  (CPU, mémoire, registres matériels, images, couches). API : `run(driver, AppOptions)`.
