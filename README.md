# 🌊 Project Alpha *(nom temporaire)*
 
> Wave survivor multijoueur from scratch en Rust — sans game engine.  
> Jusqu'à 4 joueurs, architecture client-serveur autoritaire.
 
---
 
## Stack technique
 
| Domaine | Crate |
|---|---|
| ECS | `legion` |
| Réseau | `renet` + `renetcode` |
| Rendu | `raylib` |
| Sérialisation | `bincode v2` + `serde` |
 
---
 
## Architecture
 
### Vue globale
 
```
[Home lab server] ← toujours allumé, IP fixe / DDNS
        ↑
  [Client] → connexion automatique au démarrage → choix du mode
        ↓
  Solo   : serveur spawne une instance via MemoryTransport (in-process)
  Multi  : session normale via NetcodeTransport (UDP, jusqu'à 4 joueurs)
```
 
### Structure du workspace Cargo
 
```
project-alpha/
├── shared/     # Types réseau, protocole, composants ECS communs
├── server/     # Simulation autoritaire headless (Legion ECS + renet)
└── client/     # Rendu raylib depuis snapshots + VFX locaux
```
 
### Philosophie ECS hybride
 
Le projet adopte une architecture **ECS hybride assumée** :
 
- **ECS (Legion)** → entités du monde de jeu : joueurs, ennemis, projectiles, boss
  - Composants : `Position`, `Velocity`, `Health`, ...
  - Systèmes : mouvement, combat, spawn, IA ennemis
- **Managers externes** → données par joueur, accédées par `ClientId`
  - `ShopManager` : `HashMap<ClientId, ...>`
  - `SpellManager` : `HashMap<ClientId, Vec<Sort>>`
**Règle de communication** : les systèmes ECS n'accèdent jamais directement aux managers. Ils émettent des événements (`EventSpellCast`, `EventHit`, `EventKill`...) que les managers consomment, et vice versa.
 
### Règle VFX
 
Tous les effets visuels (particules, slash, glow, camera shake) sont gérés **uniquement côté client**. Le serveur envoie des événements réseau légers, le client en déduit les VFX à spawner. Aucun VFX dans l'ECS serveur.
 
```
Exemple :
  Serveur → EventKill { position }
  Client  → spawne burst de particules à cette position
```
 
### Canaux réseau (renet)
 
| Canal | Constante | Type | Usage |
|---|---|---|---|
| 0 | `CHANNEL_STATE` | Unreliable | Snapshots d'état (positions, health — haute fréquence) |
| 1 | `CHANNEL_EVENT` | Reliable Ordered | Événements de jeu (kill, hit, mort, VFX...) |
| 2 | `CHANNEL_INPUT` | Unreliable | Inputs joueur (mouvement, dash — haute fréquence) |
| 3 | `CHANNEL_SHOP` | Reliable Ordered | Transactions shop, sorts |
 
---
 
## Lancer le projet
 
> ⚠️ Prérequis : Rust stable, `cargo`
 
```bash
# Cloner le repo
git clone https://github.com/Snowdeenn/network_project_alpha.git
cd network_project_alpha
 
# Lancer le serveur
cargo run -p server
 
# Lancer le client (dans un autre terminal)
cargo run -p client
```
 
Par défaut le client se connecte à `127.0.0.1:7777`.
 
---
 
## Gameplay
 
- **Wave survivor** : survivre à des vagues d'ennemis de plus en plus difficiles
- **Shop inter-vagues** : acheter des sorts et des améliorations passives
- **Sorts** : cartes à usage unique ou avec cooldown, hotbar en bas de l'écran
- **Boss** : un boss en fin de vague avec une FSM synchronisée serveur → client
- **Équilibrage dynamique** : difficulté adaptée au nombre de joueurs en ligne
---
 
## Style visuel
 
Style **hand-painted minimaliste** :
- Silhouettes simples, palette pastel désaturée pour le décor
- Couleurs saturées réservées aux éléments actifs (joueur, sorts, ennemis)
- Particules et VFX en code pur (aucun asset pour les effets)
- Glow simulé par blending additif, bloom en post-process GLSL
---
 
## Roadmap
 
Voir [ROADMAP.md](./ROADMAP.md) pour le détail des phases.
 
---
 
## Licence
 
Projet personnel — tous droits réservés.
 

