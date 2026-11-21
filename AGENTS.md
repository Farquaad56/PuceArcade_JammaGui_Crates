# AGENTS.md — règles pour l'agent de développement (modèle local, contexte 135k)

Projet : **PuceArcade JammaGui Crates** — contrat `puce-jamma` + frontal egui réutilisable `puce-jamma-gui` (repris du GUI de Puce16 Neo Geo).
Hôte : Windows / PowerShell. Profil Hermes : `rust-dev`.

## Boucle de travail
1. Lire `docs/STATE.md` puis **une seule** carte `tasks/Jxxx.md` ; ne lire que sa section « À lire ».
2. Faire la 1re sous-tâche non cochée ; test d'abord. 3. Acceptation ciblée puis `powershell scripts/gate.ps1`.
4. Cocher, commit `Jxxx.n: message`. 5. Fin de carte : `docs/STATE.md` + `tasks/board.json`.

## Budget de contexte (135k)
≤ 3 fichiers source ouverts en entier ; jamais un fichier > 400 lignes en entier (plage de lignes de la carte ou grep) ; sorties de tests filtrées ;
à 90k tokens : finir la sous-tâche, écrire STATE.md, s'arrêter.

## Règles de code
- Un crate par carte. `puce-jamma` est GELÉ (docs/spec/00) : tout changement = ADR.
- Fichiers ≤ 400 lignes, fonctions ≤ 60 lignes. `#![forbid(unsafe_code)]`. Pas de `unwrap()` hors tests.
- Aucune taille d'image codée en dur dans le frontal : tout vient de `VideoInfo`. Aucune référence à un système précis (Neo Geo, CPS1…) dans le frontal.
- Logique testable (calculs d'échelle, filtres, correspondance des touches, préférences) dans des fonctions pures testées sans fenêtre ;
  le code egui ne fait qu'afficher.
- Cartes « reprise » : partir du fichier de `ref/puce16-neogeo/ng-gui/` indiqué, retirer ce qui est propre au Neo Geo (BIOS, mahjong, carte mémoire,
  LSPC…), remplacer les types `ng_*` par ceux de `puce-jamma`. Ne pas changer le comportement des filtres (tests repris).

## Anti-boucle
3 tentatives max sur un même échec, puis bissection, « Blocage » dans STATE.md et `blocked`. Ne jamais affaiblir un test.
Pas de web : `docs/` et `ref/` seulement ; une info manquante va dans `docs/QUESTIONS.md`.
