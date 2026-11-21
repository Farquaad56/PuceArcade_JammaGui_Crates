# Méthode
Identique à Puce16 Neo Geo / PuceArcade CPS1 : cartes autoportantes (« Claude pense, le codeur applique »), gate unique, une session par carte,
revue séparée en fin de phase (docs/REVIEW.md). Oracle du frontal : la carte de test `puce-jamma-testboard` (images et sons déterministes) et
les tests repris du GUI Neo Geo (filtres, échelle, préférences, touches).
Phases : J0 contrat et mire -> J1 frontal (affichage, filtres, overlay, préférences, thread, son, commandes, menus, application, débogueur, savestates) -> J2 publication.
Kanban Hermes : `scripts/kanban_import.ps1` (board « jamma »).

## Granularité (petit modèle, contexte 135k) — règles de découpage des cartes
- Une carte = UNE session neuve. Budget indiqué dans l'en-tête de chaque carte (« lecture ≈ Xk ; code ≈ N lignes ; session ≈ Yk ») ; cible ≤ 60k, arrêt propre à 90k.
- Au plus ~250 lignes de Rust écrites à la main par carte (hors fichiers générés par `tools/` et fichiers copiés par commande), au plus 4 sous-tâches,
  chacune compilable et testée seule. Une carte trop grosse se découpe en `Txxx` + `Txxxb` (même crate, fichiers distincts : `impl` répartis sur plusieurs fichiers).
- Fichiers repris d'un autre dépôt (`ref/puce16-neogeo/…`) : copie par commande (`Copy-Item`), puis éditions ciblées (remplacements, suppressions) ;
  ne jamais les réécrire en entier dans la réponse. Ils peuvent dépasser 400 lignes : lecture par plages (`Select-String` pour trouver, puis lignes ciblées).
- Sorties d'outils toujours filtrées (`| Select-Object -Last 40`) ; tests ciblés pendant la sous-tâche, gate complet en fin de sous-tâche.
- Les cartes sont rédigées par Claude : l’agent ne les réécrit pas (il coche les sous-tâches et met à jour board.json). Une carte découpée garde son id pour la 1re partie ; `scripts/kanban_sync.ps1` met à jour Hermes sans doublon.
