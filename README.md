# Project Alpha

Jeu de survie par vagues multijoueur en Rust, sans moteur de jeu intégré. Le serveur autoritaire simule le monde ; le client affiche les snapshots et produit les effets visuels. Une session accueille jusqu'à quatre joueurs.

Documentation mise à jour le 5 octobre 2026 par lecture des sources. Les fonctionnalités présentes ne sont pas nécessairement validées par un playtest.

## Stack actuelle

- Rust, édition 2024, workspace Cargo à trois crates.
- Serveur : ECS Legion, Renet/Netcode UDP, bincode v2.
- Client : fenêtre Winit, GPU WGPU, rendu Prism, interface Nodus et debug Egui.
- Dépendances Git Snowdeenn : math, prism, nodus et weave.
- Configuration JSON via Serde ; shaders WGSL.

Raylib n'est plus une dépendance Cargo ; un module portant ce nom subsiste dans les sources.

## Organisation

```text
client/src/
  app/             Boucle Winit, ressources et scènes menu/lobby/partie
  core/            Client réseau, événements et états de jeu/UI/boutique
  graphic_data/    Assets, animations, tile map et shaders WGSL
  rendering/       Monde, HUD, caméra et VFX
  ui/              HUD Nodus et debug Egui
server/src/
  app/             Initialisation et boucle serveur
  net/             Transport UDP et réception des messages
  session/         Lobby, classes et registre des joueurs
  simulation/      Composants, ressources et systèmes Legion
  navigation/      Grille spatiale et gestion des flow fields
  replication/     Snapshots et événements réseau
  utils/           Pools d'entités et files d'événements
utils/src/         Protocole, configuration, sorts, cartes, arènes et buffers
assets/            Configurations JSON, classes, textures et polices
doc/               Roadmaps et cahier des charges UI
```

La crate commune est `utils`, auparavant décrite comme `shared` dans la documentation.

## Fonctionnement

1. Le menu crée le client réseau lors du choix Solo ou Multijoueur.
2. Le lobby synchronise la sélection Warrior/Assassin/Mage/Tank et l'état prêt.
3. Le serveur crée les joueurs depuis les configurations de classe et démarre les vagues.
4. Le client envoie ses entrées à 20 Hz. Le serveur vise un tick toutes les 50 ms et fournit ce pas fixe à la simulation.
5. Les snapshots transmettent entités, état des vagues et informations propres au joueur destinataire. Le client interpole les positions.
6. Les événements synchronisent notamment boutique, sorts, morts et respawn, et déclenchent les VFX locaux.

Legion gère les entités côté serveur. Des registres de joueurs, boutiques, sorts, pools et files d'événements complètent l'ECS. Les VFX restent côté client.

### Transport actuel

**Le serveur doit être lancé séparément, y compris pour Solo.** Les deux choix de menu utilisent le même client UDP Netcode. Le serveur embarqué et le transport mémoire restent à implémenter.

Client et serveur utilisent l'adresse codée en dur `127.0.0.1:7777`, le protocole `1337` et l'authentification Netcode `Unsecure`. Le serveur écoute sur l'interface locale ; les connexions entre machines nécessitent une évolution de cette configuration.

Les cinq canaux sont définis dans `utils/src/net/mod.rs` :

- `0 / CHANNEL_STATE` : snapshots, non fiable.
- `1 / CHANNEL_EVENT` : événements, fiable et ordonné.
- `2 / CHANNEL_INPUT` : entrées, non fiable.
- `3 / CHANNEL_SHOP` : boutique, fiable et ordonné.
- `4 / CHANNEL_LOBBY` : lobby, fiable et ordonné.

## Lancement

Prérequis : une chaîne Rust compatible avec l'édition 2024 et les dépendances, les outils de compilation natifs de la plateforme, et un accès aux dépendances Cargo/Git au premier build. Le client nécessite un GPU pris en charge par WGPU.

Lancer depuis la racine du dépôt : les chemins des assets et shaders sont relatifs à ce répertoire.

```sh
cargo run -p server
```

Dans un autre terminal :

```sh
cargo run -p client
```

Au menu, Entrée sélectionne Solo et M sélectionne Multijoueur. Choisir ensuite une classe et passer prêt dans le lobby.

Commandes telles que codées avec les codes physiques Winit :

- W/A/S/D : déplacement ; souris : visée.
- Espace : dash ; clic gauche : attaque.
- E/Q/V/C : emplacements de sorts, avec clic gauche lors de l'appui sur la touche.
- P : changement du mode de debug.

## Configuration et vérification

`assets/config/` contient vagues, ennemis, sorts, paramètres de jeu, physique, session et animations. `assets/classes/` contient les quatre classes. `server_config.json` configure la session, pas l'adresse réseau.

Les sources contiennent notamment des tests d'arènes, buffers, cartes, flow fields, pools et VFX. Pour vérifier le workspace :

```sh
cargo check --workspace
cargo test --workspace
```

Ces commandes sont des instructions, pas un compte rendu de tests exécutés lors de cette mise à jour documentaire.

## Documentation

- [Roadmap du jeu](doc/ROADMAP.md)
- [Roadmap du rendu et des VFX](doc/roadmap-renderer-vfx.md)
- [Cahier des charges UI](doc/Cahier_des_charge_framework_ui_embarquer.md) : objectifs de conception et intégration actuelle, sans garantie de performance mesurée.

Projet personnel — tous droits réservés.
