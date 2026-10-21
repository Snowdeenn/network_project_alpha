# Boule de feu

`../fireball.json` compose les quatre effets du vol :

- `core.json` : un cœur de 20 unités, avec une référence de mesh sphérique.
- `flames.json` : 24 particules par seconde, émises autour du cœur.
- `trail.json` : un ruban de 12 unités de large, limité à 32 points.
- `light.json` : une lumière orange de rayon 80.

`../fireball_impact.json` compose l'impact : `impact_particles.json` émet une
seule fois 40 particules dans toutes les directions.

## Chargement

Charger et publier les composants avant leurs composites :

1. `assets/config/vfx/fireball/core.json`
2. `assets/config/vfx/fireball/flames.json`
3. `assets/config/vfx/fireball/trail.json`
4. `assets/config/vfx/fireball/light.json`
5. `assets/config/vfx/fireball/impact_particles.json`
6. `assets/config/vfx/fireball.json`
7. `assets/config/vfx/fireball_impact.json`

Les noms utilisables pour le spawn sont `fireball` et `fireball_impact`.
Les cinq composants élémentaires peuvent utiliser le loader async et
`registry.register_loaded()`. Publier ensuite les deux composites avec
`registry.load()` pendant la préparation de la scène, ou `register_json()`
avec leur contenu déjà lu.

## Déclenchement

Pour le vol, fournir `VfxSpawnParams.lifetime: 10.0` et un attachement au
projectile avec `MissingOwner::Immediate`. Les durées de sécurité du cœur, de la
lumière et du trail sont aussi de dix secondes. Ne pas activer `looping`.
L'attachement fournit la position résolue ; `projectile_center` nomme seulement
la source attendue et ne recherche pas automatiquement une entité.

Lors de l'impact, le gameplay arrête le handle de vol avec `StopMode::Immediate`,
puis demande `fireball_impact` à la position de collision, sans attachement.
Les particules d'impact disparaissent seules en moins de 0,6 seconde.
Il n'y a pas de champ `on_impact` dans le schéma actuel.

L'arrêt immédiat retire le cœur, la lumière, le trail et les flammes de vol.
`Drain` conserverait notamment le cœur et la lumière jusqu'à leur durée de vie
de dix secondes ; il n'est donc pas adapté à l'arrêt de ce composite à l'impact.

## Ressources visuelles

Les chemins de mesh, matériau et textures sont des références prévues pour
le rendu final ; ces ressources ne sont pas fournies. L'adaptateur actuel affiche
les surfaces en quads et le trail en segments. La lumière est simulée et extraite,
mais ne possède pas encore de passe GPU. Ces fichiers ne créent ni collision,
ni déplacement, ni dégâts.
