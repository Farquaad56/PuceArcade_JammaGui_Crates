# Changelog

## v0.6.0
- `GameEntry::system` (ADR 003) : système du jeu quand un driver en sert plusieurs. « Ouvrir un jeu » affiche
  l'étiquette `[CPS1]` / `[CPS2]` devant le titre et une case par système pour filtrer la liste.
- Changement cassant : les drivers qui construisent `GameEntry` littéralement ajoutent `system` (vide si un seul
  système).

## v0.5.0

- Contrat (ADR 002) : `GameEntry` gagne `kind: GameKind` (`Original`, `Clone`, `Bootleg`, `Hack`, `Prototype`) ;
  `GameKind::from_mame(parent, constructeur, titre)` donne la règle de classement commune aux drivers,
  `GameKind::commercial()` vaut vrai pour les originaux et les clones officiels.
- « Ouvrir un jeu » : code couleur par catégorie (blanc : originaux, bleu : clones officiels, orange : bootlegs,
  magenta : hacks, jaune : prototypes), cases à cocher par catégorie avec le nombre de jeux (légende et
  filtre), boutons « Commerciaux seulement » et « Tout afficher », nombre de jeux affichés.

## v0.4.0

- Fenêtre « Filtres vidéo » réorganisée : préréglage et « Tout désactiver » en tête, puis cinq onglets
  (Image, Lignes et masque, Couleur, Tube, Effets) ; un point marque les onglets actifs, chaque onglet se
  réinitialise seul.
- Nouveaux effets CRT (`crt.rs`, `crt_light.rs`) :
  - scanlines « Faisceau gaussien » : épaisseur de la ligne selon la valeur de chaque canal (ligne sombre /
    claire réglables) ;
  - masques calculés « Fentes » (slot mask des moniteurs d'arcade) et « Points » (shadow mask), taille des
    luminophores réglable (auto ou 1 à 4 px) ;
  - température du blanc (5 000 à 9 300 K), niveau du noir ;
  - halo (lumière diffusée dans le verre), intensité et taille ;
  - overscan (0 à 10 %) ;
  - courbure, vignettage, coins arrondis (désactivés automatiquement quand un bezel est affiché) ;
  - instabilité de l'image, taches de pureté ;
  - images noires intercalées (netteté du mouvement), actives seulement si l'écran va au moins à 1,8 fois la
    fréquence du jeu.
- Préréglage « Moniteur d'arcade 15 kHz ». Les préréglages ne touchent plus à l'overscan, à la géométrie, aux
  défauts ni aux images noires.
- Ordre du rendu : faisceau, géométrie, tube, halo, puis couleur, masque et vignettage en une passe (le masque
  est fixe sur la face du tube et n'est plus déformé).
- Bezels : option « Reflets de la vitre » (menu Affichage, préférence `[display] bezel_glass`, activée par
  défaut) ; désactivée, l'écran du bezel devient entièrement transparent (`Bezel::pixels(glass)`).
- API : `FxWindow::show` prend un `FxEnv` (bezel affiché, fréquences de l'écran et du jeu) ; `Fx` gagne
  `beam_dk`, `beam_br`, `mask_pitch`, `temp`, `black`, `bloom`, `bloom_size`, `overscan`, `curve`,
  `vignette`, `corner`, `jitter`, `purity`, `bfi` ; `ScanMode::Gaussian`, `Mask::Slot`, `Mask::Shadow`.
- Mires de réglage (`mire.rs`, `mire_menu.rs`) : bouton « Mires » et Ctrl+M ; géométrie, convergence,
  overscan, barres de couleur, pureté, contraste, luminosité, dégradés, gamma, netteté, mouvement, avec
  variantes ; à la définition du jeu, à travers les filtres et le bezel ; jeu en pause pendant les mires ;
  Page préc. / suiv. (mire), Ctrl + Page (variante), Ctrl+H (panneau).
- Barre de balayage : lueur additive du faisceau en plus de l'éclaircissement, visible aussi sur le noir
  (mires sur fond noir).
- Barre de balayage et instabilité animées aussi sur les mires et après de longues parties (numéro de
  trame réduit en entier avant le calcul en f32).
- Halo : blocs blancs parasites dans les zones noires (racine d'un négatif de la somme glissante du flou)
  corrigés.
- Exemples `crt_demo` (aperçu du préréglage arcade), `fx_bench` (coût par trame) et `mires` (mires en PNG).

## v0.3.0

- Bezels RetroArch en plein écran (`puce_jamma_gui::bezel`) : `<jeu>.cfg` puis parent puis `_generic.cfg`
  dans `AppOptions::bezels_dir`, PNG chargé dans un fil, écran détecté par l'alpha (< 128 autour du centre,
  vitre semi-transparente dessinée par-dessus le jeu), image du jeu ajustée à l'écran du bezel.
- Menu Affichage : case « Bezels en plein écran », raccourci Ctrl+B, préférence `[display] bezels` (activée par
  défaut).
- API : `AppOptions` gagne `bezels_dir: Option<PathBuf>` ; `StatusInfo` gagne `audio` (fréquence et canaux).
- Barre d'état : fréquence et mode du son (« son 24038 Hz stéréo », « son 55930 Hz mono »).
- Exemple `bezel_info` : taille du bezel et écran détecté.

## v0.2.0

- `puce-jamma-gui`, fenêtre « Filtres vidéo », nouvelle section **Tube CRT** (réglages indépendants des préréglages
  d'écran, sauvegardés dans `prefs.toml`, section `[fx]`) :
  - canaux RVB : gain du rouge, du vert et du bleu (0 à 200 %) ;
  - convergence : décalage du rouge et du bleu par rapport au vert (−8 à 8 px, X et Y), boutons « Convergence
    parfaite » et « Tube fatigué » ;
  - barre de balayage : bande claire qui descend l'écran avec une traînée sombre, intensité, durée d'un passage
    (0,5 à 10 s) et hauteur réglables.
- API : `Fx` gagne `red`, `green`, `blue`, `conv_rx`, `conv_ry`, `conv_bx`, `conv_by`, `sweep`, `sweep_speed`,
  `sweep_width` ; `postfx::render` prend le numéro de trame (position de la barre) ; `postfx::sweep_gains`,
  `Fx::tube_active`, `Fx::without_tube`, `Fx::with_tube_of`.
- Débogueur : état de la carte rafraîchi en continu quand le panneau est ouvert (≈ 6 fois par seconde, à la
  mise en pause et au chargement ; `Cmd::DebugLive`), « Pas » sur le CPU choisi (`Cmd::Step(cpu)`), mémoire relue
  à chaque instantané, registres des CPU 8 bits sur 16 bits.
- Overlay de performances placé dans la zone de l'image et sous les fenêtres (il ne recouvre plus les fenêtres,
  le panneau du débogueur ni la barre d'état).
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
