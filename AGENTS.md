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

## Granularité (petit modèle, contexte 135k) — règles de découpage des cartes
- Une carte = UNE session neuve. Budget indiqué dans l'en-tête de chaque carte (« lecture ≈ Xk ; code ≈ N lignes ; session ≈ Yk ») ; cible ≤ 60k, arrêt propre à 90k.
- Au plus ~250 lignes de Rust écrites à la main par carte (hors fichiers générés par `tools/` et fichiers copiés par commande), au plus 4 sous-tâches,
  chacune compilable et testée seule. Une carte trop grosse se découpe en `Txxx` + `Txxxb` (même crate, fichiers distincts : `impl` répartis sur plusieurs fichiers).
- Fichiers repris d'un autre dépôt (`ref/puce16-neogeo/…`) : copie par commande (`Copy-Item`), puis éditions ciblées (remplacements, suppressions) ;
  ne jamais les réécrire en entier dans la réponse. Ils peuvent dépasser 400 lignes : lecture par plages (`Select-String` pour trouver, puis lignes ciblées).
- Sorties d'outils toujours filtrées (`| Select-Object -Last 40`) ; tests ciblés pendant la sous-tâche, gate complet en fin de sous-tâche.
- Les cartes sont rédigées par Claude : l’agent ne les réécrit pas (il coche les sous-tâches et met à jour board.json). Une carte découpée garde son id pour la 1re partie ; `scripts/kanban_sync.ps1` met à jour Hermes sans doublon.

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
