# Roadmap rendu et VFX — Project Alpha

État du renderer au 21 octobre 2026 : migration vers Prism V2 et vérification GPU hors écran. Le comportement complet en partie et les performances restent à vérifier en jeu.

## Architecture actuelle

- `client/src/app/mod.rs` initialise Winit, le contexte GPU Prism, les ressources, shaders et passes de post-process.
- `client/src/app/states/in_game.rs` gère snapshots, événements, scène, caméra et rendu.
- `client/src/rendering/mod.rs` génère les commandes monde/HUD.
- `client/src/graphic_data/asset_manager.rs` regroupe actuellement le gestionnaire d'animations ; les ressources GPU sont gérées avec Prism.
- `client/src/graphic_data/tile_map.rs` gère le rendu de la carte.
- `client/src/ui/` contient le HUD Nodus et le debug Egui.
- `client/src/graphic_data/shader/` contient les shaders WGSL.

Le backend GPU utilise `prism-v2` : `GraphRenderer`, `ResourceManager` et `FrameV2`. L'ancien backend Raylib a été supprimé. Voir [l'intégration Prism V2](renderer-prism-v2.md) pour le flux de rendu et la frontière de compatibilité Nodus.

## Fondations

- [x] Scène InGameScene extraite de la boucle principale.
- [x] Rendu via FrameV2, géométries et matériaux V2, avec vues monde et HUD distinctes.
- [x] Chargement des animations JSON avec les ressources GPU Prism.
- [x] Registre d'identifiants et BufferManager partagé.
- [x] Initialisation des shaders et passes de post-process, avec uniforms de flash.
- [x] Caméra et interpolation depuis les snapshots.
- [x] Supprimer le backend portant encore le nom Raylib.
- [x] Vérifier hors écran l'ordre des couches, les VFX additifs, le HUD après les effets et le redimensionnement.
- [ ] Valider les ressources et identifiants lors des rechargements ou changements de scène.

## Effets branchés au gameplay

- [x] Runtime VFX unique : registre JSON, instances, budgets partagés et pool de 1024 particules.
- [x] Poussière de déplacement depuis les positions interpolées.
- [x] Particules d'impact et camera shake sur EntityHit.
- [x] Slash depuis position, direction et dimensions de SpawnRect.
- [x] Flash de post-process sur PlayerHit.
- [x] Blind et aperçus de debug pilotés par VfxApi, sans timers parallèles.
- [x] Supprimer les anciens pools, VfxManager et la couche de compatibilité.
- [ ] Ajuster densité, durée et intensité des effets en playtest.

## Primitives présentes, intégration à compléter

VfxApi expose les flashes par entité, trails manuels bornés et fantômes de dash.
Ils utilisent les familles du nouveau runtime et leur rendu est testé sur GPU.
Les déclencheurs métier exigent des événements identifiant l'entité ou la phase d'animation.

- [x] Appliquer le flash d'entité au rendu, y compris aux sprites avec masque alpha.
- [ ] Déclencher le flash depuis un événement identifiant la victime.
- [ ] Alimenter les trails avec les positions et phases d'attaque.
- [ ] Déclencher les fantômes de dash depuis un état ou événement adapté.
- [x] Nettoyer les historiques des entités disparues et vérifier les limites de capacité.
- [ ] Ajouter burst de mort ennemi et respiration à l'arrêt.

## Polish et éclairage

- [ ] Lerp de l'arme et transitions UI.
- [ ] Retour visuel sur achat refusé.
- [ ] Glow via les commandes et modes de blending Prism.
- [ ] Bloom dans une passe WGSL.
- [ ] Lumières sur les projectiles et réglage du camera shake.
- [ ] Effets sonores synchronisés avec les événements de gameplay.

## Performance et outillage

- [ ] Mesurer coût des commandes, texte HUD, particules et trails.
- [ ] Identifier les allocations restantes avant de promettre un rendu sans allocation.
- [ ] Évaluer AnimEntityManager et le nettoyage des entités supprimées.
- [ ] Étudier le hot reload des shaders avec Prism ; il n'est pas établi par l'intégration actuelle.
- [ ] Mesurer séparément sérialisation réseau et construction des snapshots.

Priorité : brancher et valider les primitives existantes, puis mesurer avant d'ajouter bloom et optimisations.
