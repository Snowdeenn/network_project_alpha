# Roadmap rendu et VFX — Project Alpha

État au 5 octobre 2026 par lecture des sources du jeu. Les capacités internes des dépendances Git Prism et Nodus ne sont pas auditées ici. Les fonctionnalités présentes restent à vérifier en jeu.

## Architecture actuelle

- `client/src/app/mod.rs` initialise Winit, le contexte GPU Prism, les ressources, shaders et passes de post-process.
- `client/src/app/states/in_game.rs` gère snapshots, événements, scène, caméra et rendu.
- `client/src/rendering/mod.rs` génère les commandes monde/HUD.
- `client/src/graphic_data/asset_manager.rs` regroupe actuellement le gestionnaire d'animations ; les ressources GPU sont gérées avec Prism.
- `client/src/graphic_data/tile_map.rs` gère le rendu de la carte.
- `client/src/ui/` contient le HUD Nodus et le debug Egui.
- `client/src/graphic_data/shader/` contient les shaders WGSL.

Les anciennes étapes Raylib, DrawRing, GLSL et RenderTexture2D sont remplacées par l'intégration WGPU/Prism. Le module `rendering/backend/raylib.rs` conserve un nom historique.

## Fondations

- [x] Scène InGameScene extraite de la boucle principale.
- [x] Rendu via prism::Frame et commandes monde/HUD.
- [x] Chargement des animations JSON avec les ressources GPU Prism.
- [x] Registre d'identifiants et BufferManager partagé.
- [x] Initialisation des shaders et passes de post-process, avec uniforms de flash.
- [x] Caméra et interpolation depuis les snapshots.
- [ ] Clarifier ou renommer le module portant encore le nom Raylib.
- [ ] Vérifier l'ordre des passes, la composition carte/monde et le redimensionnement.
- [ ] Valider les ressources et identifiants lors des rechargements ou changements de scène.

## Effets branchés au gameplay

- [x] ParticlePool : 512 emplacements préalloués et réutilisés après expiration ; les spawns sont ignorés si le pool est plein.
- [x] Poussière de déplacement depuis les positions interpolées.
- [x] Particules d'impact et camera shake sur EntityHit.
- [x] Slash depuis position, direction et dimensions de SpawnRect.
- [x] Flash de post-process sur PlayerHit.
- [ ] Ajuster densité, durée et intensité des effets en playtest.

## Primitives présentes, intégration à compléter

VfxManager expose des flashes par entité, trails d'épée avec historique circulaire et fantômes de dash, avec des tests unitaires. La scène appelle actuellement spawn_slash, mais ne branche pas ces trois autres API.

- [ ] Déclencher le flash de l'entité touchée et appliquer son état au rendu.
- [ ] Alimenter les trails avec les positions et phases d'attaque.
- [ ] Déclencher les fantômes de dash depuis un état ou événement adapté.
- [ ] Nettoyer les historiques des entités disparues et vérifier les limites de capacité.
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
