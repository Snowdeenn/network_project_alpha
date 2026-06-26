# 🗺️ Project Alpha — Roadmap

> Wave survivor en Rust · Legion ECS · renet · raylib  
> Style visuel : hand-painted minimaliste avec juice (particules, lerp, glow)

---

## Statut global

| Phase | Nom | Statut |
|---|---|---|
| 1 | Fondations réseau | ✅ Terminé |
| 2 | Modes de jeu & transport | 🔄 En cours |
| 3 | Gameplay de base & Architecture ECS | 🔄 En cours |
| 4 | Contenu & progression | ⏳ À venir |
| 5 | Style visuel & VFX | ⏳ À venir |
| 6 | Boss & équilibrage | ⏳ À venir |
| 7 | Polish & infrastructure | ⏳ À venir |

---

## Architecture globale

```
[Home lab server] ← toujours allumé, IP fixe / DDNS
        ↑
  [Client] → connexion au démarrage → choix de la classe & du mode
        ↓
  Solo   : serveur spawne instance locale via MemoryTransport (in-process)
  Multi  : serveur garde la session via NetcodeTransport (UDP)
```

```
project-alpha/
├── shared/   # Types réseau, protocole, composants ECS communs (AttackStats, MeleeBrain...)
├── server/   # Simulation autoritaire headless (Systèmes de combat unifiés, Spawner découpé)
└── client/   # Rendu raylib depuis snapshots + VFX locaux (Slash, impacts, UI)
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

Le serveur home lab est le point d'entrée unique. Au lancement le client s'y connecte automatiquement puis choisit son mode et son personnage.

### 2a — Infrastructure serveur
- [ ] Adresse serveur configurable (fichier de config ou variable d'environnement, pas hardcodé)
- [ ] DDNS setup sur le home lab (ex: DuckDNS) pour IP publique stable
- [ ] Serveur en écoute permanente, gestion des reconnexions clients

### 2b — Sélection du mode de jeu & Lobby de classe
- [ ] Écran de sélection Solo / Multijoueur au lancement
- [ ] **Lobby de sélection des personnages** (Max 4 joueurs) : Choix parmi les classes (`Warrior`, `Assassin`, `Mage`, `Tank`)
- [ ] Mode **Solo** : serveur spawne une instance via `MemoryTransport` (in-process, zéro réseau)
- [ ] Mode **Multijoueur** : session normale via `NetcodeTransport` (UDP), jusqu'à 4 joueurs
- [ ] Lancement d'une partie (lobby minimal : ready → start)
- [x] Spawner de joueur Data-Driven (injection des composants spécifiques à la classe choisie au moment du `spawn`)
- [ ] Respawn des joueurs en cours de partie

---

## Phase 3 — Gameplay de base & Architecture ECS 🔄

Assainir l'architecture pour rendre le combat et les vagues 100% extensibles (Open/Closed Principle).

### 3a — Mouvement & Pipeline de combat unifié
- [ ] Bug dash corrigé (#26)
- [x] Dash avec cooldown validé côté serveur
- [x] **Système de combat générique unifié** : Utilisation du composant `AttackStats` (range, damage, dimensions) et du composant de transition `AttackIntent` partagés entre Joueurs et Ennemis.
- [x] **Boîtes d'attaques dynamiques** : `create_melee_attackbox` génère des hitbox sur-mesure (plus de constantes d'allonge globales).
- [x] **Collisions unifiées et optimisées** : `check_collide_attackbox` lit le composant `Damage` de la hitbox et utilise des buffers `thread_local!` pour éliminer les allocations par frame (`.collect()`).
- [x] Knockback sur les ennemis touchés (#31)
- [x] **Mouvement dynamique** : Remplacer la constante `ACCEL` par un composant `MovementStats` pour que l'Assassin coure plus vite que le Tank.
- [ ] Gestion des sorts (actifs / passifs — à définir)
- [x] Mort du joueur (à polish)

### 3b — Ennemis & Vagues (Architecture découplée)
- [x] **Éclatement du God System `wave_update`** en 3 sous-systèmes spécialisés à responsabilité unique :
  * `wave_death_reaper` : récolte des morts et mise à jour des compteurs.
  * `enemy_spawner` : logique physique d'apparition en cercle autour des joueurs.
  * `wave_flow_manager` : cerveau de haut niveau / machine à états des vagues (`InProgress`, `BetweenWave`).
- [x] **IA par Tags Comportementaux** : Utilisation de filtres positifs (`MeleeBrain`, `RangedBrain`) à la place des filtres d'exclusion négatifs (`!RangedIA`) pour permettre l'ajout de nouveaux monstres sans toucher au code existant.
- [x] Intégration des 3 types d'ennemis distincts (minimum) via la nouvelle architecture de Tags et de statistiques dynamiques.
- [x] Nombre de vagues configurable depuis `wave.json`
- [ ] Boss de fin de vague avec FSM (Phase 6 — Synchronisation FSM Boss)

### 3c — Debug & outillage
- [x] ImGUI intégré (#30) pour debug in-game (stats réseau, état ECS, positions)

---

## Phase 4 — Contenu & progression ⏳

### 4a — Contenu joueur & Équilibrage des classes
- [ ] Validation physique des 4 profils de jeu (Warrior équilibré, Assassin ultra-rapide/fragile, Tank lent/robuste, Mage à distance)
- [ ] 6 types d'ennemis total (configurés uniquement via `AttackStats` et composants Brain)
- [ ] 15 cartes de sort minimum
- [ ] 10 vagues avec difficulté croissante
- [ ] Map plus grande avec zones distinctes

### 4b — Multijoueur complet
- [ ] Réseau : Réplication et synchronisation des composants dynamiques (`AttackStats`, `MovementStats`) des 4 joueurs.
- [ ] Affichage de tous les joueurs dans le HUD
- [x] Mort individuelle sans bloquer la partie des autres
- [ ] Score partagé en fin de partie

### 4c — Audio
- [ ] Sons d'impact (attaque, mort ennemi)
- [ ] Musique d'ambiance (loop)
- [ ] Son de shop

---

## Phase 5 — Style visuel & VFX ⏳

Style hand-painted minimaliste : formes simples, brosses texturées, palette pastel désaturée avec accents saturés réservés aux éléments actifs.

### 5a — Assets de base
- [x] Personnage joueur (silhouette cape, œil) — dessiné dans Krita
- [ ] 3 types d'ennemis — formes minimalistes
- [ ] Tileset sol (brosses chalk, grain léger)
- [ ] Éléments de décor (buissons ronds, monolithes)

### 5b — Système de particules (client uniquement)
- [x] Pool de particules (`Vec<Particle>` avec position, vélocité, lifetime, color, size)
- [x] Particules poussière sous les pieds (run + changement de direction) (à polish)
- [ ] Particule de buée/respiration (timer quand le joueur s'arrête)
- [ ] Particules d'impact (ennemi touché) — déclenché par événement réseau `EventHit`
- [ ] Particules de mort ennemi (burst de cercles) — déclenché par `EventKill`
- [ ] Effet de shake sur la carte du joueur quand il ne peut pas acheter (#4 backlog)

### 5c — VFX de combat
- [ ] Slash effect : arc `DrawRing` adapté graphiquement aux dimensions réelles de l'`AttackBox` reçues du serveur.
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
- [ ] Équilibrage dynamique selon le nombre de joueurs (difficulté adaptative des vagues)
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

> **Objectif** : Exploiter la nouvelle architecture de combat/vagues pour finaliser le Lobby multijoueur et connecter les statistiques dynamiques.  
> Ordre recommandé : `Bug dash (#26) → 2b (Lobby + Menu de classes) → Intégration MovementStats (3a) → Intégration des variants d'ennemis (3b) → Phase 5`
