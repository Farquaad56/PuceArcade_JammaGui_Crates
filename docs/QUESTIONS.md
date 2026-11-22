# Questions ouvertes
## Décisions (2026-11-21, utilisateur)
- Dépôt dédié (ce dépôt), épinglé par révision git par les émulateurs.
- Mêmes filtres et overlay que Puce16 Neo Geo (scalers HQ2X/Eagle/2xSaI/Scale2x…, presets CRT, masques Trinitron / Faded tube, overlay de performances).
- Puce16 Neo Geo n'est PAS migré vers ce frontal pour l'instant.
## Points ouverts
- [OUVERT] Q-STEREO : le mixage mono -> stéréo du frontal duplique la voie (channels = 1). Panoramique non prévu.
- [OUVERT] Q-4P : correspondance clavier par défaut des joueurs 3-4 (aucune en v0.1 : manettes 3-4 seulement).
- [RÉSOLU] Q-PAUSE-P (J108) : la carte J108 met la pause sur P, mais J107 donne P au Start du joueur 2. Retenu : pause sur Ctrl+P ; les autres raccourcis restent sur F3/F5-F8/F11/F12 sans modificateur.
- [RÉSOLU] Q-RESAMPLE-TOL (J106) : « 55 930,4 -> 48 000 ± 1 » oubliait l'échantillon d'amorçage de l'interpolation : 55 929 × 48 000 / 55 930,4 = 47 998,8 -> 47 998 sorties ; borne basse du test à 47 998.
