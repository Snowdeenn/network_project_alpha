# Project Alpha — Roadmap

État au 5 octobre 2026, établi par lecture des sources. Une case cochée indique une implémentation présente, sans garantie de validation en jeu. Les objectifs de contenu restent des cibles.

## 1. Réseau et réplication

- [x] Serveur autoritaire Legion, client consommant et interpolant les snapshots.
- [x] Renet/Netcode UDP, bincode v2 et cinq canaux : état, événements, entrées, boutique, lobby.
- [x] Simulation à pas fixe de 50 ms et entrées client à 20 Hz.
- [x] Informations propres au destinataire dans les snapshots.
- [ ] Rendre configurables l'adresse serveur et les interfaces d'écoute.
- [ ] Valider les connexions entre machines, puis l'accès au serveur home lab.
- [ ] Consolider déconnexions, reconnexions et redémarrages de session.
- [ ] Vérifier la numérotation des ticks serveur et la réplication des statistiques dynamiques : le snapshot actuel transmet surtout positions, santé et types d'entités, avec une santé maximale fixée à 100.

## 2. Menu, modes et lobby

- [x] Menu avec choix Solo et Multijoueur.
- [x] Lobby à quatre emplacements, choix de classe et état prêt.
- [x] Classes Warrior, Assassin, Mage et Tank configurées en JSON et utilisées au spawn.
- [ ] Implémenter un serveur solo embarqué avec transport mémoire. Les deux modes utilisent actuellement le même serveur UDP lancé séparément.
- [ ] Finaliser la saisie et la validation d'un code de session.
- [ ] Faire respecter le compte à rebours : le serveur annonce trois secondes mais appelle actuellement `start_game` immédiatement.
- [ ] Vérifier la capacité et les indices des emplacements, notamment après un départ.

## 3. Simulation et gameplay

- [x] Mouvement paramétré, collisions, dash et cooldown côté serveur.
- [x] Combat commun via AttackStats, AttackIntent et hitboxes dynamiques.
- [x] Dégâts, santé, knockback et projectiles.
- [x] IA mêlée, distance et kamikaze.
- [x] Grille spatiale, carte procédurale et navigation par flow fields.
- [x] Vagues séparées en récolte des morts, spawn et gestion du déroulement.
- [x] Registre de sorts JSON, résolution des lancers, zones d'effet, effets et cooldowns.
- [x] Boutique, achats et emplacements de sorts.
- [x] Mort, demandes de respawn via vies partagées ou or, et système serveur de respawn.
- [x] Valider en jeu dash, classes, sorts, achats et respawn. Statut marqué terminé à la demande de l'utilisateur le 6 octobre 2026.
- [ ] Compléter HUD multijoueur, mode spectateur et score de fin de partie.

## 4. Rendu et effets

Le client utilise Winit/WGPU, Prism, Nodus, des shaders WGSL et Egui pour le debug. Voir la [roadmap dédiée](roadmap-renderer-vfx.md).

- [x] Animations, tile map, caméra et HUD.
- [x] Pool fixe de particules, poussière de déplacement et impacts.
- [x] Slash sur SpawnRect, camera shake sur EntityHit et post-process de flash sur PlayerHit.
- [x] Primitives de flash d'entité, trail d'épée et fantômes de dash dans VfxManager.
- [ ] Brancher les primitives VFX restantes au gameplay.
- [ ] Ajouter effets de mort, respiration, glow et bloom.
- [ ] Compléter les assets et polir les transitions UI.

## 5. Contenu, boss et équilibrage

- [ ] Atteindre six types d'ennemis, quinze cartes de sort et dix vagues équilibrées.
- [ ] Développer les boss avec FSM synchronisée et phases selon la santé. Les types et événements du protocole ne constituent pas une implémentation complète.
- [ ] Adapter la difficulté au nombre de joueurs.
- [ ] Ajouter sons d'impact, boutique et musique.
- [ ] Valider carte procédurale, zones distinctes et navigation en jeu.

## 6. Qualité et distribution

- [x] Tests unitaires présents dans plusieurs modules.
- [x] Logs Tracing et interface de debug Egui.
- [ ] Exécuter et stabiliser les vérifications Cargo et playtests solo/multi.
- [ ] Mesurer allocations, sérialisation, réplication et coût du rendu avant optimisation.
- [ ] Ajouter paramètres et configuration des commandes.
- [ ] Préparer builds release et packaging Windows/Linux.
- [ ] Organiser des playtests externes et traiter les retours.

## Priorités proposées

1. Valider menu → lobby → partie → mort/respawn, ainsi que le dash.
2. Corriger les écarts de session et de réplication, puis configurer le réseau entre machines.
3. Implémenter le vrai mode solo et brancher les VFX disponibles.
4. Étendre contenu, boss et audio après stabilisation du socle.
