# Méthode
Identique à Puce16 Neo Geo / PuceArcade CPS1 : cartes autoportantes (« Claude pense, le codeur applique »), gate unique, une session par carte,
revue séparée en fin de phase (docs/REVIEW.md). Oracle du frontal : la carte de test `puce-jamma-testboard` (images et sons déterministes) et
les tests repris du GUI Neo Geo (filtres, échelle, préférences, touches).
Phases : J0 contrat et mire -> J1 frontal (affichage, filtres, overlay, préférences, thread, son, commandes, menus, application, débogueur, savestates) -> J2 publication.
Kanban Hermes : `scripts/kanban_import.ps1` (board « jamma »).
