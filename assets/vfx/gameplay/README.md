# Effets gameplay

Ces JSON remplacent les paramètres auparavant codés dans les anciens pools.
`VfxRegistry::register_gameplay_effects()` les embarque et les enregistre dans
l'ordre des dépendances, avant les premières demandes gameplay.

Les noms `gameplay/` évitent les collisions avec les exemples et les assets de sorts.
Les composants de `entity_hit.json` doivent être chargés avant ce composite.
Les fichiers peuvent aussi être rechargés via les méthodes ordinaires du registre.

Les nouvelles options de description gardent les valeurs par défaut des anciens JSON :

- Particules : position `box`, vitesse polaire ou cartésienne (`x`/`y`),
  `evolution.growth`, `appearance.scale` et `appearance.fade_lifetime`.
- Meshes : représentations `quad` et `ring`, `appearance.fade_lifetime`.
  Pour un anneau, `appearance.size` et l'override `VfxSpawnParams.size`
  contiennent `[rayon intérieur, rayon extérieur]`.
- Trails : `emission.manual` utilise les points fournis par VfxApi plutôt
  qu'une émission à taux fixe ; `appearance.fade_points` réduit largeur et alpha.
- Post-process : `appearance.envelope` contrôle le fondu d'entrée/sortie du
  poids. Une sortie `null` couvre toute la durée de vie de l'occurrence.
- Feedback : `camera_shake` émet une impulsion, `entity_tint` expose une teinte
  pour le propriétaire pendant la durée demandée, sans créer de ressource GPU.

Voir `doc/client-vfx-system.md` pour les API, limites et tests.
