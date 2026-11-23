# ADR 003 — Système d'un jeu du catalogue (`GameEntry::system`)
- Contexte : un même driver peut servir plusieurs systèmes (le driver Capcom sert le CPS1 et le CPS2). La fenêtre
  « Ouvrir un jeu » doit dire à quel système appartient chaque jeu et permettre de filtrer par système.
- Décision : contrat v0.6 — `GameEntry` gagne `system: String`, libre (« CPS1 », « CPS2 »…), vide quand le driver
  ne sert qu'un système. Le frontal n'affiche l'étiquette `[système]` et les cases « Système » que si le catalogue
  contient au moins deux systèmes non vides ; le filtre masque les systèmes décochés (`GameFilter::hidden_systems`).
- Conséquences : changement cassant pour les drivers qui construisent `GameEntry` littéralement (ajout du champ,
  `String::new()` pour un driver mono-système) ; la carte de test et le driver Capcom sont mis à jour.
- Cartes impactées : aucune carte ouverte (évolution demandée par l'utilisateur, hors cartes).
