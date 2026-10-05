# Simulations Python / Rust

## Parties réelles et workloads

`workload.py` démarre le serveur de production puis connecte de vrais clients Renet/Netcode via l'exemple Rust `network_bots`. Python décide des actions à partir des snapshots reçus. Aucun système de gameplay n'est remplacé et les configurations ne sont pas modifiées.

```sh
python scripts/workload.py --players 1 --seconds 60 --profile idle
python scripts/workload.py --players 2 --seconds 120 --profile movement
python scripts/workload.py --players 4 --seconds 180 --profile combat --hz 60 --cast-hz 20
```

- `idle` : joueurs immobiles, entrées neutres.
- `movement` : déplacements circulaires, sans combat.
- `combat` : poursuite de l'ennemi vivant le plus proche, attaques, achats entre vagues si l'or suffit, demandes de sorts selon `--cast-hz` et tentatives de respawn avec les vies partagées.
- `--hz` : cadence visée des envois d'entrées, pas fréquence du tick serveur. La cadence effective dépend aussi du coût Python/IPC.
- `--players` : 1 à 4, limite du serveur actuel.
- `--seconds` : durée totale, connexion et lobby compris.
- `--no-build` : réutiliser les binaires ; reconstruire après toute modification Rust.
- `--external-server --address 127.0.0.1:7777` : utiliser un serveur déjà lancé sans l'arrêter à la fin. Préférer une session fraîche, car le démarrage/rejoin de parties existantes n'est pas encore stabilisé.
- `--output chemin` : dossier neuf pour les résultats, afin de ne pas écraser un précédent essai.

Chaque essai produit `server.log`, `telemetry.jsonl` (actions envoyées et tous les snapshots/événements reçus) et `summary.json` (paramètres, compteurs, états finaux et éventuelle erreur). Les résultats sous `workload-results/` sont ignorés par Git.

Les bots passent prêts après que tous ont rejoint le lobby. Le profil combat est une stratégie simple de charge, pas une garantie de survie ou de victoire. Les sorts ne sont demandés qu'après acquisition réelle : une session sans achat peut donc ne produire aucun lancer. Les tentatives à fréquence élevée servent notamment à observer les refus de cooldown.

Le compteur d'octets mesure les payloads applicatifs reçus, sans en-têtes UDP ni retransmissions. L'écart entre livraisons de snapshots est mesuré côté Python et inclut le batching de l'adaptateur ; ce n'est ni un RTT réseau ni une mesure du temps CPU serveur. Les ticks des snapshots sont enregistrés tels quels, même si leur numérotation est incorrecte. Les essais ne sont pas déterministes (IA, spawn, ordonnancement réseau).

### Mesures et journalisation

La collecte CPU/mémoire fonctionne sous Windows (API natives via ctypes) et Linux (/proc), sans package Python externe. Elle échantillonne séparément le serveur, l'adaptateur Rust des bots et le pilote Python toutes les 0,5 seconde (`--sample-interval`).

- `resources.jsonl` : CPU cumulé, CPU sur l'intervalle, mémoire résidente (RSS), mémoire privée sous Windows et compteurs I/O du processus. CPU 100 % signifie un cœur logique complet ; un processus multithread peut dépasser 100 %. Sous Linux, la mémoire privée n'est pas mesurée et reste null. Les compteurs I/O suivent les conventions de chaque OS.
- `server-ticks.jsonl` : temps réel de traitement de chaque tick, intervalle entre ticks, réception réseau, commandes, simulation ECS, événements, construction/sérialisation des snapshots, envoi et nettoyage. Inclut nombre de joueurs, entités actives/allouées, événements en file, messages/octets applicatifs et erreurs de décodage. Un dépassement signifie que le traitement dépasse le budget de 50 ms, hors sommeil et écriture du fichier de métriques.
- `events.jsonl` : événements de gameplay, démarrage/arrêt des processus et erreurs, horodatés pour les corréler aux mesures serveur.
- `telemetry.jsonl` : commandes des bots et réponses brutes, avec temps des décisions Python, échanges avec l'adaptateur et cycles complets. Le temps d'échange inclut l'attente réseau volontaire et n'est pas un RTT.
- `summary.json` : paramètres, distributions moyenne/p50/p95/p99/max, dépassements du budget, ressources par processus, compteurs réseau et états finaux. Zéro échantillon signifie une mesure indisponible, pas une utilisation nulle.

Exemple de collecte pour comparer des charges :

```sh
python scripts/workload.py --players 4 --seconds 120 --profile combat --hz 60 --cast-hz 20 --sample-interval 0.5 --release
```

Utiliser le même profil de compilation, les mêmes configurations et la même machine pour comparer les essais. Sans `--release`, les binaires sont en debug. La collecte et les logs ont un coût : le profiler mesure séparément le temps d'écriture de la ligne précédente (`observer_ms_previous_tick`), et l'intervalle réel du tick inclut cette instrumentation. La durée `simulation_ms` couvre le schedule ECS complet, pas chaque système individuellement. L'échantillonnage CPU/RAM commence après le lancement des bots et peut manquer les pics d'initialisation.

Le fichier de métriques est activé uniquement si `NETWORK_ALPHA_METRICS` est défini ; le pilote le définit pour le serveur qu'il lance. Les compteurs et chronomètres ajoutent un petit coût même sans fichier. Pour un serveur externe local, `--server-pid PID` permet de mesurer son processus ; ses timings internes nécessitent qu'il ait été lancé avec `NETWORK_ALPHA_METRICS` pointant vers `server-ticks.jsonl` dans le dossier de résultats. Un serveur distant ne peut pas être échantillonné localement.

`--log-level debug` configure le filtre Tracing du serveur lancé. Les logs serveur existants sont conservés, sans inventer de traces internes qui ne sont pas émises par le code.

Vérification de la collecte native et des statistiques :

```sh
python -m unittest discover -s scripts -p "test_process_metrics.py" -v
```

Le serveur utilise actuellement l'authentification locale Unsecure. Le pilote n'utilise pas la fenêtre du jeu et ne lance pas d'effet visuel.

## Scénarios ciblés sans réseau

Depuis la racine du dépôt :

```sh
python scripts/simulation.py
```

Python 3 et Cargo suffisent, sans package Python externe. Le script compile l'exemple Rust `simulation_bridge`, puis le pilote avec des commandes JSON sur stdin/stdout. Après compilation, `--no-build` réutilise le binaire.

Pour lancer tous les scénarios après compilation :

```sh
python -m unittest discover -s scripts -p "test_simulation.py" -v
```

La suite vérifie également le déplacement d'un joueur sans déplacer l'autre et le refus d'un lancement sans or. Si `python` n'est pas dans le PATH, utiliser le chemin complet de l'interpréteur Python 3.

Le scénario crée deux joueurs et un ennemi, équipe la boule de feu, la lance, tente un second lancement pendant le cooldown et avance 102 ticks (5,1 secondes simulées). Il vérifie le coût, le rejet pendant le cooldown, les dégâts, la santé du second joueur, l'expiration du cooldown et la disparition du projectile. Une assertion échouée donne un code de sortie non nul.

## Créer un scénario

Importer `Simulation` depuis `simulation.py`, utiliser `with Simulation() as game`, puis `game.command(op, **arguments)`.

- `player` : id, class (Warrior/Assassin/Mage/Tank), position [x,y].
- `enemy` : id, position [x,y], hp ; cible immobile sans IA autonome.
- `gold` : id du joueur, amount à ajouter.
- `equip` : id, spell (identifiant JSON), slot de 0 à 3.
- `input` : id, movement [x,y] facultatif, aim [x,y] non nul, slot facultatif.
- `step` : ticks, de 0 à 10000 ; chaque tick vaut 50 ms.
- `snapshot` : état des entités, or, cooldowns, nombre de projectiles et événements depuis la dernière lecture.

Les entrées de déplacement persistent ; envoyer movement [0,0] pour arrêter. Les demandes de sort sont consommées par le système Rust.

## Portée

Les systèmes Rust de mouvement, projectile, collision, sorts, statuts et dégâts sont exécutés sans rendu. La simulation utilise des barrières explicites entre création de composants et consommation pour rendre les scénarios reproductibles.

Cette première version teste le gameplay des sorts. Elle injecte directement l'or et l'équipement et ne valide pas encore le transport UDP, le lobby, les achats, les vagues, l'IA autonome ou le parcours mort/respawn. Elle ne modifie pas les configurations du jeu et n'ouvre aucun port. Le binaire est un exemple Cargo séparé du serveur normal.
