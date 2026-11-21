# PuceArcade JammaGui Crates

Le « connecteur JAMMA » en Rust : un contrat unique entre une carte d'arcade émulée et un frontal, comme le connecteur des bornes
qui accepte n'importe quelle carte.

| Crate | Rôle |
|---|---|
| `puce-jamma` | contrat : `JammaBoard` (vidéo, son, commandes 4 joueurs × 6 boutons, pièces, service, test, tilt, analogique, DIP décrits, savestates, débogage optionnel) et `BoardDriver` (catalogue, chargement). Aucune dépendance. |
| `puce-jamma-testboard` | carte de test (mire, curseur, bip, DIP) pour développer et tester les frontaux. |
| `puce-jamma-gui` | frontal egui réutilisable, repris du GUI de Puce16 Neo Geo : scalers 2x (HQ2X, Eagle, 2xSaI, Scale2x…), filtres CRT et masques, overlay de performances, son cpal, clavier et manettes, menus DIP générés, savestates, captures, débogueur générique. |

Contrat : `docs/spec/00-contrat-jamma.md`. Développement par cartes Hermes Kanban (`tasks/`, `scripts/kanban_import.ps1`, board « jamma »), méthode de Puce16 Neo Geo.
Premier utilisateur : [PuceArcade CPS1](https://github.com/Farquaad56/PuceArcade_CPS1) (épingle une révision git).
