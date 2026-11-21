# ADR-001 — Un contrat « connecteur JAMMA » entre cartes émulées et frontal
Statut : accepté (2026-11-21, demande de l'utilisateur)
## Contexte
Plusieurs émulateurs d'arcade (Puce16 Neo Geo, PuceArcade CPS1, Cave, Galaxian…) ont chacun un GUI. Le connecteur JAMMA des bornes a résolu le même
problème côté matériel : une borne, n'importe quelle carte.
## Décision
`puce-jamma` définit le trait `JammaBoard` (vidéo, son, commandes, DIP, savestates, débogage optionnel) et `BoardDriver` (catalogue/chargement).
`puce-jamma-gui` est un frontal unique qui ne dépend que de ce contrat. Les filtres et l'overlay du GUI Neo Geo y sont repris tels quels, rendus
indépendants de la taille d'image. Puce16 Neo Geo n'est pas migré pour l'instant.
## Conséquences
Chaque émulateur n'écrit qu'un adaptateur (entrées actives hautes -> registres, métadonnées DIP, panneaux de débogage). Le contrat est versionné ;
les émulateurs épinglent une révision git.
