# Index des tâches (CHAÎNE LINÉAIRE : chaque carte dépend de la précédente ; profil rust-dev)

| Ordre | Id | Phase | Titre |
|---|---|---|---|
| 1 | J000 | J0 Contrat et mire | Workspace Cargo vérifié, gate, métriques |
| 2 | J001 | J0 Contrat et mire | puce-jamma : contrat « connecteur JAMMA » (types, traits, utilitaires) |
| 3 | J002 | J0 Contrat et mire | puce-jamma-testboard : carte de test (mire, entrées, bip, DIP, savestate, débogage) |
| 4 | J100 | J1 Frontal | puce-jamma-gui : placement de l'image (échelle entière/ajustée, rapport, rotation) |
| 5 | J101 | J1 Frontal | puce-jamma-gui : scalers 2x (HQ2X, Eagle, 2xSaI, Scale2x…) — reprise |
| 6 | J102 | J1 Frontal | puce-jamma-gui : filtres CRT (postfx : interpolation, scanlines, masques, presets) — reprise généralisée |
| 7 | J103 | J1 Frontal | puce-jamma-gui : overlay de performances et barre d'état — reprise |
| 8 | J104 | J1 Frontal | puce-jamma-gui : préférences (prefs.toml) — reprise |
| 9 | J105 | J1 Frontal | puce-jamma-gui : boucle d'émulation (1/2) — step_once, cadence, captures PNG |
| 10 | J105b | J1 Frontal | puce-jamma-gui : boucle d'émulation (2/2) — thread, commandes, cadence, savestates |
| 11 | J106 | J1 Frontal | puce-jamma-gui : son (cpal, rééchantillonnage, régulation de dérive) — reprise |
| 12 | J107 | J1 Frontal | puce-jamma-gui : commandes JAMMA (1/2) — actions, touches par défaut, clavier -> JammaInputs |
| 13 | J107b | J1 Frontal | puce-jamma-gui : commandes JAMMA (2/2) — manettes gilrs, préférences, fenêtre de configuration |
| 14 | J108 | J1 Frontal | puce-jamma-gui : menus (jeux du driver, DIP décrits, filtres, commandes, états, captures) |
| 15 | J109 | J1 Frontal | puce-jamma-gui : application (1/2) — boucle egui (image, rotation, filtres, overlay, son, entrées) |
| 16 | J109b | J1 Frontal | puce-jamma-gui : application (2/2) — point d'entrée run(), options, exemple sur la mire |
| 17 | J110 | J1 Frontal | puce-jamma-gui : débogueur (1/2) — lecture mémoire, fenêtres CPU et mémoire, pas à pas |
| 18 | J110b | J1 Frontal | puce-jamma-gui : débogueur (2/2) — registres matériels, images de débogage, couches |
| 19 | J200 | J2 Publication | Guide d'intégration, README, version v0.1.0 |
