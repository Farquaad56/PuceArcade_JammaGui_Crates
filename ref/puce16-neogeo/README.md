# Modèle : GUI de Puce16 Neo Geo (même auteur, MIT), lecture seule, NE PAS COMPILER ICI
Copie de https://github.com/Farquaad56/Puce16_NeoGeo `crates/ng-gui/src/` (2026-11-21) et des générateurs de données des filtres (`tools/`).
Les données binaires (fx.bin, hq2x_lut.bin, masques) sont déjà copiées dans `crates/puce-jamma-gui/data/`.
Spécifique Neo Geo à retirer lors de la reprise : bios_menu.rs, mahjong / carte mémoire / AES-MVS dans input.rs, menu.rs, settings.rs ; LSPC dans video_debug.rs ;
`FB_W`/`FB_H` (320×224) partout -> taille issue de `VideoInfo`.
