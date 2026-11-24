# ADR 004 — Voies sonores pour l'oscilloscope du débogueur (`BoardDebug::voices`)
- Contexte : le débogueur doit montrer le son voie par voie (canaux FM, voix PCM/ADPCM…), comme les
  oscilloscopes par canal des débogueurs de consoles, pour toutes les plateformes. Le découpage en voies
  dépend de chaque puce son : le frontal ne peut pas le connaître.
- Décision : contrat v0.7 — `puce_jamma::debug::AudioVoice { name, chip, sample_rate, samples, active,
  detail }` et trois méthodes de `BoardDebug`, toutes avec une implémentation par défaut : `voices()`
  (vide), `set_voice_scope(on)` (enregistrement des voies seulement onglet Son ouvert, pour ne rien coûter
  sinon) et `set_voice_mute(mask)` (bit n = voie n coupée dans le mixage, 0 = son normal). Le frontal
  ajoute l'onglet « Son » (`audio_debug.rs`) : groupes par `chip`, grille d'oscilloscopes, déclenchement
  sur front montant, échelle auto, clic = couper, clic droit = solo. Commandes `Cmd::VoicesLive`,
  `Cmd::VoiceMute`, événement `Event::Voices` (≈ 30 par seconde en marche).
- Conséquences : non cassant (méthodes par défaut) : une carte sans voies affiche « Cette carte ne
  fournit pas de voies sonores ». Chaque plateforme choisit ses voies et leur détail ; le muet ne doit pas
  changer le son quand le masque vaut 0. La carte de test fournit trois voies (bip réel, sinus, scie).
- Cartes impactées : aucune carte ouverte (évolution demandée par l'utilisateur, hors cartes).
