# STATE — journal de passation (garder < 150 lignes)
## Phase courante
J2 Publication terminée : v0.1.0. Prochaine étape : intégration dans PuceArcade CPS1 (carte T500 du CPS1).
## Fait
- (conception) Workspace 3 crates vides, contrat docs/spec/00, GUI Neo Geo copié dans ref/, données des filtres (fx.bin, hq2x_lut.bin, masques) dans crates/puce-jamma-gui/data/.
- J000 : tests triviaux ajoutés aux 3 lib.rs ; check_metrics.ps1 vérifié (baseline créée si absente, REGRESSION + code 1 si pass baisse) ; .gitignore et metrics/.gitkeep créés ; gate OK.
- J001 : contrat puce-jamma complet — types inputs/video/audio/dips/board/debug/driver, traits JammaBoard/BoardDebug/BoardDriver, utilitaires rotate/default_dips/dip_value/set_dip/active_low avec tests (8 tests). Gate OK.
- J002 : carte de test puce-jamma-testboard complète (mire colorée, grille DIP, curseur contrôlable, témoins boutons, audio carré 1kHz, savestate versionné, BoardDebug). 13 tests OK. Gate OK (fmt/clippy/test).
- J100 : placement de l'image dans puce-jamma-gui — display_size/layout/scale_label généralisés (rotation, aspect Tv via info.aspect), prefs.rs avec ScaleMode/Aspect repris de ng-gui. 9 tests OK dont cas 384x224 et vertical Rot270. Gate OK.
- J101 : scalers 2x repris de ng-gui/pixel2x.rs (HQ2X, Eagle, SuperEagle, 2xSaI, Scale2x…) — fichier copié tel quel (déjà indépendant de la taille), module déclaré dans lib.rs, test upscale_neogeo_sizes ajouté (384×224→768×448, 224×288→448×576). 8 tests pixel2x OK. Gate OK.
- J102 : filtres CRT repris de ng-gui/postfx.rs — généralisé avec params src_w/src_h (plus de FB_W/FB_H), signature render(src, src_w, src_h, fx, w, h). Tests adaptés + 2 nouveaux tests 384×224. 11 tests postfx OK. Gate OK.
- J103 : overlay et barre d'état repris — Perf paramétré par refresh_hz (Perf::new(59.637) → budget_ms=16.768), StatusInfo générique (system/game/resolution/refresh_hz, bios/model retirés). 9 tests overlay + 4 statusbar OK. Gate OK.
- J104-J200 (Claude, 2026-11-22, agent arrêté à la demande de l'utilisateur pour aller plus vite) :
  - J104 prefs.rs repris (prefs.toml, rotation forcée « auto|0|90|180|270 », fréquence audio souhaitée ; DIP MVS retirés) ; puis `last_game` ([session]) pour J109b.
  - J105/J105b emu_thread.rs (Cmd, Event, DebugSnapshot, step_once, frame_period), capture.rs (PNG après rotation), emu_loop.rs (thread propriétaire de la carte, cadence Instant + rattrapage 3 trames, savestates <jeu>.s<n>, captures, Debug après Step/FrameAdvance).
  - J106 resample.rs (AudioResampler repris + push_interleaved mono/stéréo) et audio.rs (format natif f32/i16/u16, sortie mono). Piège : 55 930 échantillons à 55 930,4 Hz -> 47 998 sorties (délai d'amorçage d'un échantillon) ; tolérance du test élargie à 47 998 avec justification.
  - J107/J107b input.rs (51 actions calculées, touches et boutons par défaut, to_jamma), bindings.rs (échange de touches comme l'original, prefs), gamepad.rs (manette n -> joueur n, stick), controls_menu.rs (joueurs 1..=players, boutons 1..=buttons). Bug attrapé par le test : la colonne Système affichait le joueur 3 sur une carte à 2 joueurs.
  - J108 games.rs (filtre), dips_menu.rs (générique, persistance <jeu>.dips), fx_menu.rs (copié tel quel), menu.rs + raccourcis purs. Écart : pause sur Ctrl+P (P = Start du joueur 2 en J107), voir QUESTIONS.
  - J109/J109b app.rs + app_actions.rs + app_panels.rs (trames, rotation, filtres, son régulé par drc_adjust, entrées, menus, fenêtres, overlay, barre d'état, taille initiale 3 × display_size Tv), lib.rs run()/AppOptions/startup_game, exemples/testboard.rs.
  - J110/J110b debugger.rs (onglets CPU, Mémoire par Cmd::Peek -> Event::Memory, Pas/Image/Continuer) et video_debug.rs (registres, images zoom 1-4, couches -> LayerMask).
  - Vérification visuelle (Xvfb + llvmpipe) : mire à x2 et 60 fps, mire_v tournée correctement, overlay, débogueur (la fenêtre s'élargit du panneau). Captures : docs/images/.
  - J200 docs/INTEGRATION.md, README (captures, commandes, raccourcis), CHANGELOG ; tag annoté local v0.1.0 (push du tag par l'utilisateur).
- **v0.1.0 = commit `311b8eccc5e6b1204704ecf985ef785b8ecec33c`** (tag annoté local `v0.1.0`) : révision à épingler par PuceArcade CPS1 (carte T500), `rev = "311b8eccc5e6b1204704ecf985ef785b8ecec33c"`.
## Blocages
(aucun)
