# ADR 005 — Filtres vidéo sur carte graphique (wgpu), filtres processeur en référence
- Contexte : les filtres (faisceau, masque, halo, géométrie…) étaient calculés par le processeur à
  la taille de sortie. En x8 (3072 × 1792) ils coûtaient environ 40 ms par image : l'interface ne
  montrait plus qu'environ 18 images par seconde alors que le jeu tournait à 60.
- Décision : les mêmes calculs sont refaits en WGSL (`gpufx.wgsl`) et exécutés par la carte
  graphique dans le rendu wgpu d'eframe (rappel `egui_wgpu`, `gpufx_paint.rs`). Le processeur ne
  prépare plus que la source (overscan, agrandissement ×2) et des tables (`gpufx_data.rs`) ; les
  calculs entiers restent entiers, les tables sont celles de `postfx.rs`. Passes : faisceau →
  géométrie + tube → halo (réduction, 3 flous) → passe finale dessinée directement dans la fenêtre.
  `postfx::render` reste la référence et le repli : rendu OpenGL sans cible flottante, image plus
  grande que les textures permises ou sortant de la fenêtre.
- Conséquences : `gpufx_tests.rs` compare les deux rendus sur tous les préréglages et réglages
  extrêmes (écart maximal 2 niveaux sur 255, dû au halo). Toute évolution d'un filtre se fait des
  deux côtés. La fenêtre Filtres indique le calcul utilisé ; la barre de performance affiche
  « FILTRE GPU » ou « CPU ». Ajout d'une qualité globale (`quality.rs`) sur la même barre.
