# STATE — journal de passation (garder < 150 lignes)
## Phase courante
J1 Frontal. Prochaine carte : J104.
## Fait
- (conception) Workspace 3 crates vides, contrat docs/spec/00, GUI Neo Geo copié dans ref/, données des filtres (fx.bin, hq2x_lut.bin, masques) dans crates/puce-jamma-gui/data/.
- J000 : tests triviaux ajoutés aux 3 lib.rs ; check_metrics.ps1 vérifié (baseline créée si absente, REGRESSION + code 1 si pass baisse) ; .gitignore et metrics/.gitkeep créés ; gate OK.
- J001 : contrat puce-jamma complet — types inputs/video/audio/dips/board/debug/driver, traits JammaBoard/BoardDebug/BoardDriver, utilitaires rotate/default_dips/dip_value/set_dip/active_low avec tests (8 tests). Gate OK.
- J002 : carte de test puce-jamma-testboard complète (mire colorée, grille DIP, curseur contrôlable, témoins boutons, audio carré 1kHz, savestate versionné, BoardDebug). 13 tests OK. Gate OK (fmt/clippy/test).
- J100 : placement de l'image dans puce-jamma-gui — display_size/layout/scale_label généralisés (rotation, aspect Tv via info.aspect), prefs.rs avec ScaleMode/Aspect repris de ng-gui. 9 tests OK dont cas 384x224 et vertical Rot270. Gate OK.
- J101 : scalers 2x repris de ng-gui/pixel2x.rs (HQ2X, Eagle, SuperEagle, 2xSaI, Scale2x…) — fichier copié tel quel (déjà indépendant de la taille), module déclaré dans lib.rs, test upscale_neogeo_sizes ajouté (384×224→768×448, 224×288→448×576). 8 tests pixel2x OK. Gate OK.
- J102 : filtres CRT repris de ng-gui/postfx.rs — généralisé avec params src_w/src_h (plus de FB_W/FB_H), signature render(src, src_w, src_h, fx, w, h). Tests adaptés + 2 nouveaux tests 384×224. 11 tests postfx OK. Gate OK.
- J103 : overlay et barre d'état repris — Perf paramétré par refresh_hz (Perf::new(59.637) → budget_ms=16.768), StatusInfo générique (system/game/resolution/refresh_hz, bios/model retirés). 9 tests overlay + 4 statusbar OK. Gate OK.
## Blocages
(aucun)
