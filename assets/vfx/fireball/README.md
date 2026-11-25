# Boule de feu

`../fireball.json` compose les quatre effets du vol :

- `core.json` : un cœur de 20 unités, avec une référence de mesh sphérique.
- `flames.json` : 120 particules par seconde, émises autour du cœur.
- `trail.json` : un ruban de 12 unités de large, limité à 32 points.
- `light.json` : une lumière orange de rayon 80.

`../fireball_impact.json` compose l'impact : `impact_particles.json` émet une
seule fois 40 particules dans toutes les directions.

## Chargement

Charger et publier les composants avant leurs composites :

1. `assets/vfx/fireball/core.json`
2. `assets/vfx/fireball/flames.json`
3. `assets/vfx/fireball/trail.json`
4. `assets/vfx/fireball/light.json`
5. `assets/vfx/fireball/impact_particles.json`
6. `assets/vfx/fireball.json`
7. `assets/vfx/fireball_impact.json`

Les noms utilisables pour le spawn sont `fireball` et `fireball_impact`.
`VisualAssets` retrouve les fichiers dans `assets/config/vfx_catalog.json`,
charge aussi les composites de manière asynchrone et publie leurs dépendances
avant eux. `ensure_loaded("fireball")` suffit pour demander le vol complet.

## Déclenchement

Le serveur réplique `EntityKind::Projectile` avec `visual_profile: 1`, configuré
sur le sort. Le profil `projectile_feu` de `assets/config/visual_profiles.json`
associe `spawn` au vol attaché et `impact` à l'arrêt du vol puis à l'explosion.
Le client détecte l'apparition une fois par entité et suit les tickets puis handles.
`SpellUsed` ne déclenche pas le vol sur le joueur.

Pour le vol, fournir `VfxSpawnParams.lifetime: 10.0` et un attachement au
projectile avec `MissingOwner::Immediate`. Les durées de sécurité du cœur, de la
lumière et du trail sont aussi de dix secondes. Ne pas activer `looping`.
L'attachement fournit la position résolue ; `projectile_center` nomme seulement
la source attendue et ne recherche pas automatiquement une entité.
Le budget des flammes réserve le maximum de particules vivantes simultanément,
disparition comprise, au lieu du total émis pendant ces dix secondes.

Lors de l'impact, le serveur émet un événement visuel identifié avec le profil,
l'entité source, sa position et sa direction. Les actions du profil arrêtent
`flight` avec `StopMode::Immediate`, puis demandent `fireball_impact` à cette
position, sans attachement. Une expiration ne déclenche pas d'explosion.
Les particules d'impact disparaissent seules en moins de 0,6 seconde.
Il n'y a pas de champ `on_impact` dans le schéma actuel.

Les triggers sont dans le profil, indépendamment du schéma des effets.
Voir `doc/client-visual-triggers.md` pour les paramètres et options complets.

L'arrêt immédiat retire le cœur, la lumière, le trail et les flammes de vol.
`Drain` conserverait notamment le cœur et la lumière jusqu'à leur durée de vie
de dix secondes ; il n'est donc pas adapté à l'arrêt de ce composite à l'impact.

## Ressources visuelles

Les chemins de mesh, matériau et textures sont des références prévues pour
le rendu final ; ces ressources ne sont pas fournies. L'adaptateur actuel affiche
les surfaces en quads et le trail en segments. La lumière est simulée et extraite,
mais ne possède pas encore de passe GPU. Ces fichiers ne créent ni collision,
ni déplacement, ni dégâts.
