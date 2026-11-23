# ADR 002 — Catégorie des jeux du catalogue (`GameKind`)
- Contexte : la fenêtre « Ouvrir un jeu » doit distinguer les versions commerciales (originaux et clones
  officiels) des bootlegs, hacks et prototypes, avec un filtre et un code couleur. Le titre MAME ne suffit pas :
  trois bootlegs CPS1 (`strideruc`, `sf2mkot`, `cps1mult`) n'ont pas le mot « bootleg » dans leur titre ; seul le
  champ constructeur de la ligne `GAME` de MAME le dit. Le frontal ne connaît pas ce champ.
- Décision : contrat v0.5 — `GameEntry` gagne `kind: GameKind` (`Original`, `Clone`, `Bootleg`, `Hack`,
  `Prototype`, `Original` par défaut). `GameKind::from_mame(parent, constructeur, titre)` donne la règle commune :
  hack si le constructeur ou le titre contient « hack », puis bootleg (constructeur ou titre), puis prototype
  (titre), sinon clone si le jeu a un parent, original sinon. Une conversion officielle (« conversion ») reste un
  clone. `GameKind::commercial()` vaut vrai pour les originaux et les clones officiels.
- Conséquences : changement cassant pour les drivers qui construisent `GameEntry` littéralement (ajout du
  champ) ; le CPS1 et la carte de test sont mis à jour. Le frontal filtre et colore la liste selon `kind`.
- Cartes impactées : aucune carte ouverte (évolution demandée par l'utilisateur, hors cartes).
