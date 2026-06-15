# 🗺️ Project Alpha — Roadmap

> Wave survivor en Rust · Legion ECS · renet · raylib  
> Style visuel : hand-painted minimaliste avec juice (particules, lerp, glow)

---

## Statut global

| Phase | Nom | Statut |
|---|---|---|
| 1 | Fondations réseau | ✅ Terminé |
| 2 | Modes de jeu & transport | 🔄 En cours |
| 3 | Gameplay de base | 🔄 En cours |
| 4 | Contenu & progression | ⏳ À venir |
| 5 | Style visuel & VFX | ⏳ À venir |
| 6 | Boss & équilibrage | ⏳ À venir |
| 7 | Polish & infrastructure | ⏳ À venir |

---

## Architecture globale

```
[Home lab server] ← toujours allumé, IP fixe / DDNS
        ↑
  [Client] → connexion au démarrage → choix du mode
        ↓
  Solo   : serveur spawne instance locale via MemoryTransport (in-process)
  Multi  : serveur garde la session via NetcodeTransport (UDP)
```

```
project-alpha/
├── shared/   # Types réseau, protocole, composants ECS communs
├── server/   # Simulation autoritaire headless (Legion ECS + renet)
└── client/   # Rendu raylib depuis snapshots + VFX locaux
```

**Règle VFX** : tout effet visuel (particules, slash, glow) est géré **uniquement côté client**. Le serveur envoie des événements (`EventKill`, `EventHit`...), le client en déduit les VFX à spawner. Jamais de VFX dans l'ECS serveur.

---

## Phase 1 — Fondations réseau ✅

Architecture client-serveur autoritaire opérationnelle.

- [x] Mise en place de `renet` (4 canaux : reliable ordered, unreliable, etc.)
- [x] Crate `shared` : types réseau communs, protocole sérialisé avec `bincode v2`
- [x] Serveur headless : simulation ECS (Legion) sans rendu
- [x] Client léger : rendu depuis snapshots serveur
- [x] Interpolation client-side des entités
- [x] Gestion des IDs d'entités (collision corrigée)
- [x] Delta timing correct côté client et serveur
- [x] Synchronisation du shop entre serveur et clients
- [x] Animation "sold" côté client (3 phases, timer-driven)
- [x] HUD resolution-independent (ScreenScale constants)

---

## Phase 2 — Modes de jeu & transport ⏳

Le serveur home lab est le point d'entrée unique. Au lancement le client s'y connecte automatiquement puis choisit son mode.

### 2a — Infrastructure serveur
- [ ] Adresse serveur configurable (fichier de config ou variable d'environnement, pas hardcodé)
- [ ] DDNS setup sur le home lab (ex: DuckDNS) pour IP publique stable
- [ ] Serveur en écoute permanente, gestion des reconnexions clients

### 2b — Sélection du mode de jeu
- [ ] Écran de sélection Solo / Multijoueur au lancement
- [ ] Mode **Solo** : serveur spawne une instance via `MemoryTransport` (in-process, zéro réseau)
- [ ] Mode **Multijoueur** : session normale via `NetcodeTransport` (UDP), jusqu'à 4 joueurs
- [ ] Lancement d'une partie (lobby minimal : ready → start)
- [ ] Respawn des joueurs en cours de partie

---

## Phase 3 — Gameplay de base 🔄

Rendre le jeu jouable et fun dans sa forme la plus simple.

### 3a — Mouvement & combat
- [ ] Bug dash corrigé (#26)
- [ ] Dash avec cooldown validé côté serveur
- [ ] Attaque de mêlée (arc hitbox côté serveur)
- [ ] Knockback sur les ennemis touchés (#31)
- [ ] Gestion des sorts (actifs / passifs — à définir)
- [ ] Mort du joueur

### 3b — Ennemis & vagues
- [ ] 3 types d'ennemis distincts (comportements différents)
- [ ] Nombre de vagues configurable
- [ ] Spawner côté serveur avec timing précis
- [ ] Boss de fin de vague avec FSM (Phase 6 — Synchronisation FSM Boss)

### 3c — Progression in-run
- [ ] XP et level-up en cours de partie
- [ ] Shop fonctionnel avec au moins 6 items
- [ ] Items passifs qui modifient les stats (vitesse, dégâts, regen)

### 3d — Debug & outillage
- [ ] ImGUI intégré (#30) pour debug in-game (stats réseau, état ECS, positions)

---

## Phase 4 — Contenu & progression ⏳

### 4a — Contenu joueur
- [ ] 6 types d'ennemis total
- [ ] 15 items de shop minimum
- [ ] 10 vagues avec difficulté croissante
- [ ] Map plus grande avec zones distinctes

### 4b — Multijoueur complet
- [ ] Affichage de tous les joueurs dans le HUD
- [ ] Mort individuelle sans bloquer la partie des autres
- [ ] Score partagé en fin de partie

### 4c — Audio
- [ ] Sons d'impact (attaque, mort ennemi)
- [ ] Musique d'ambiance (loop)
- [ ] Son de shop

---

## Phase 5 — Style visuel & VFX ⏳

Style hand-painted minimaliste : formes simples, brosses texturées, palette pastel désaturée avec accents saturés réservés aux éléments actifs.

### 5a — Assets de base
- [ ] Personnage joueur (silhouette cape, œil) — dessiné dans Krita
- [ ] 3 types d'ennemis — formes minimalistes
- [ ] Tileset sol (brosses chalk, grain léger)
- [ ] Éléments de décor (buissons ronds, monolithes)

### 5b — Système de particules (client uniquement)
- [ ] Pool de particules (`Vec<Particle>` avec position, vélocité, lifetime, color, size)
- [ ] Particules poussière sous les pieds (run + changement de direction)
- [ ] Particule de buée/respiration (timer quand le joueur s'arrête)
- [ ] Particules d'impact (ennemi touché) — déclenché par événement réseau `EventHit`
- [ ] Particules de mort ennemi (burst de cercles) — déclenché par `EventKill`
- [ ] Effet de shake sur la carte du joueur quand il ne peut pas acheter (#4 backlog)

### 5c — VFX de combat
- [ ] Slash effect : arc `DrawRing` blanc, 2-3 frames, pur raylib
- [ ] Flash d'impact sur l'ennemi (inversion couleur 1 frame)
- [ ] Trail de l'épée (lerp de position, positions historisées)
- [ ] VFX de dash (traînée semi-transparente)
- [ ] Événements réseau légers pour le juice SFX & particules

### 5d — Lerp & feel
- [ ] Lerp de l'arme derrière le joueur (`pos = lerp(pos, target, dt * k)`)
- [ ] Tween sur l'apparition des éléments UI (scale 0→1 avec rebond)
- [ ] Camera shake sur les impacts forts

### 5e — Éclairage & glow
- [ ] Simulation de glow : sprite dupliqué en plus grand + alpha faible (additive blending)
- [ ] Shader post-process bloom (GLSL, optionnel)
- [ ] Lumières douces sur les projectiles ennemis

---

## Phase 6 — Boss & équilibrage ⏳

### 6a — Boss
- [ ] 3 boss différents avec FSM synchronisée serveur → client
- [ ] Synchronisation des machines à états (FSM) des boss via réseau
- [ ] Phases de boss (transitions d'état selon HP)

### 6b — Équilibrage
- [ ] Équilibrage dynamique selon le nombre de joueurs (difficulté adaptative)
- [ ] Tuning des stats ennemis par vague

---

## Phase 7 — Polish & infrastructure ⏳

- [ ] Écran titre
- [ ] Menus (settings, contrôles)
- [ ] Build release optimisé (strip symbols, LTO)
- [ ] README complet avec instructions de lancement serveur/client
- [ ] Packaging (binaires Windows + Linux)
- [ ] Playtests externes (au moins 2 sessions)
- [ ] Fix des bugs remontés

---

## Priorité immédiate

> **Objectif** : rendre le jeu jouable de bout en bout (solo + multi) avant de polisher.  
> Ordre recommandé : `Bug dash (#26) → 2b → 3a → 3b → 3c → 5b → 5c → reste`
