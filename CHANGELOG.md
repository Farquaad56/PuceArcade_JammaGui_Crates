# Changelog

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
