# Audit mémoire du serveur — 5 octobre 2026

## Cause principale identifiée

Validation après modification du filtre par l'utilisateur : recompilation puis quatre joueurs en combat, 60 Hz, 180 secondes, sans erreur (`workload-results/flowfield-confirmation-180s/`). La requête construit désormais son filtre explicitement via `<(Entity, &Position)>::query().filter(component::<Player>())`. RSS à t≈3/30/60/120/180 : 13,59 / 14,04 / 14,16 / 14,45 / 14,52 Mio. Mémoire privée : 4,13 → 4,86 Mio ; tas vivant : 1,452 → 1,612 Mio. Les champs de navigation et slots BufferManager restent à quatre pendant tout le run. Le combat reste actif (27 052 événements SpawnRect reçus, 22 SpellUsed ; diffusions comptées par destinataire), maximum 162 entités allouées et 35 actives. La hausse principale précédente de centaines de Mio a disparu sur cette fenêtre ; une faible hausse résiduelle de ~0,16 Mio du tas vivant demeure, non attribuée par ce seul run. Le nettoyage à la déconnexion et les autres problèmes secondaires restent distincts.

`server/src/simulation/systems/flow_field.rs:19` reçoit une requête explicite `Query<(Entity, &Position)>`. L'attribut `#[filter(component::<Player>())]` n'est pas appliqué à cette requête dans Legion codegen 0.4 : ces filtres d'attribut concernent la requête automatique des systèmes for_each/par_for_each. Le système parcourt donc toutes les entités avec Position et crée des champs pour les ennemis, pièces et attaques/projectiles temporaires. `FlowFieldManager.fields` les conserve après disparition des entités.

Preuves obtenues :

- Combat 60 secondes : solde attribué à `update_flow_fields_system` +127 869 368 octets (~121,95 Mio), contre +8 138 452 pour `check_collide_attackbox_system`. Ces soldes par contexte incluent des transferts entre systèmes. Résultats : `workload-results/allocation-systems-60s/`.
- Combat 20 secondes : 322 FlowFields pour quatre joueurs, contre exactement quatre slots BufferManager. Chaque champ contient 15 000 coûts u32 et 15 000 directions Vec2 de huit octets, soit 180 000 octets. Les 322 champs représentent 57 960 000 octets de tableaux, correspondant au solde +57 995 960 octets attribué à la navigation. Résultats : `workload-results/allocation-flowfields-20s/`.
- Reproduction avec le système de production : un joueur et 100 entités temporaires avec Position, créées puis détruites. Le World revient à une seule entité vivante, mais la map garde 101 champs. Hausse du tas vivant : +18 048 840 octets (~17,21 Mio). Résultats : `workload-results/flowfield-system-reproduction/`. Relance : `python scripts/legion_memory_probe.py --modes flowfield_system --output workload-results/flowfield-system-new`.
- Calcul FlowField seul, sans ce cache : 1 000 recalculs, aucune croissance du tas vivant, mémoire privée +20 Kio. Les allocations temporaires du calcul sont libérées. Résultats : `workload-results/flowfield-isolation/`.

Sur le test de 60 secondes, BufferManager effectue 25 476 acquisitions : quatre créations de slots, 21 998 remplacements de type et 3 474 réutilisations. Les remplacements provoquent des allocations mais détruisent l'ancienne valeur, et le nombre de slots reste quatre. Les collect de la simulation produisent des vecteurs locaux détruits en fin de scope ; ranged_ia_movement collecte dans une ArrayVec de taille quatre.

Correction à réaliser : imposer Player dans la requête effective, puis retirer les champs des entités qui ne sont plus des joueurs présents. Le recyclage des buffers et les listes de voisins restent des pistes de réduction des allocations, distinctes de cette accumulation. Aucun correctif de gameplay appliqué pendant cette recherche : les changements portent sur le diagnostic et les sondes. Les anciens constats ci-dessous sont dépassés lorsqu'ils déclarent la cause principale inconnue ou supposent un champ par joueur.

## Périmètre et conclusion

Audit du code serveur, des conteneurs partagés et des chemins utilisés dans les workloads. Aucun correctif appliqué. Les comparaisons utilisent les exécutables debug existants avec `--no-build` ; elles ne constituent pas une mesure en release ni une validation d'une recompilation des sources.

La croissance mémoire en combat est reproductible. Plusieurs rétentions sont présentes dans le code, mais la cause principale des dizaines/centaines de Mio supplémentaires n'est pas encore attribuée. Une allocation fréquente ne prouve pas une fuite : il faut distinguer les objets encore vivants de la mémoire que l'allocateur conserve après libération.

## Mesures

Test utilisateur de dix minutes (`workload-results/20261005-181752/`) : quatre joueurs, combat, inputs 60 Hz, 600,56 secondes, aucune erreur. RSS à t≈3/60/120/180/300/420/600 : 41,01 / 135,61 / 251,72 / 339,38 / 511,67 / 684,75 / 943,52 Mio. Mémoire privée : 31,72 → 936,71 Mio. Aucun plateau sur dix minutes. Après la troisième minute, la croissance devient très régulière, environ 86,6 Mio de mémoire privée par minute (~1,44 Mio/s).

Le workload conserve de l'activité : de la troisième à la dixième minute, environ 7 570 événements SpawnRect sont reçus par minute (diffusions comptées par destinataire), tandis que la moyenne d'entités actives reste ~15,6 et le traitement moyen ~9,7–10 ms/tick. Aucun SpellUsed n'est reçu à partir de la troisième minute ; la mémoire continue pourtant de croître. La partie ne progresse plus vers de nouvelles vagues dans cette période, donc ce n'est pas un scénario de progression normale pendant dix minutes, mais les attaques normales persistent. Le dernier snapshot montre trois joueurs à zéro HP et un à 10 HP.

Maximum global : 162 entités allouées et 32 actives. Les bots restent sous ~9 Mio RSS et Python sous ~38 Mio, contre ~944 Mio pour le serveur. Les allocations de nouveaux sorts ne sont pas nécessaires pour entretenir la croissance tardive ; une ressource créée auparavant ou un autre chemin récurrent restent possibles. Le diagnostic doit se concentrer sur les phases qui continuent d'être exécutées, en particulier attaques normales/événements, simulation et réplication, sans attribuer la hausse à un site sans mesure du tas.

Retest après recompilation offline du serveur et de l'adaptateur : quatre joueurs, combat, 60 Hz, 180 secondes, sans erreur. RSS à t≈3/30/60/90/120/150/180 : 40,4 / 81,9 / 135,6 / 190,7 / 251,9 / 296,6 / 339,6 Mio. Mémoire privée : 31,1 → 331,0 Mio, soit environ 1,70 Mio/s. Maximum : 166 entités allouées et 37 actives. Aucun plateau visible sur cette durée. Résultats : `workload-results/audit-memory-retest-180s/`. Ce retest confirme la reproduction avec les sources recompilées, sans attribuer la croissance à un des défauts listés ci-dessous.

- Test témoin : quatre joueurs, profil idle, 20 Hz d'inputs, 30 secondes. Entre t≈3 et t≈30, RSS 39,2 → 39,5 Mio ; mémoire privée 30,0 → 30,2 Mio. Maximum : 154 entités allouées, 9 actives. Résultats : `workload-results/audit-memory-idle/`.
- Comparaison : quatre joueurs, profil combat, même fréquence et durée. Entre t≈3 et t≈30,5, RSS 36,8 → 81,2 Mio ; mémoire privée 28,1 → 71,8 Mio, soit environ 1,59 Mio/s. Maximum : 159 entités allouées, 14 actives. Résultats : `workload-results/audit-memory-combat/`.
- Run antérieur : quatre joueurs, combat, 60 Hz, 180 secondes. Entre t≈3 et t≈180, RSS 38,3 → 329,0 Mio ; mémoire privée 29,2 → 320,3 Mio, environ 1,64 Mio/s. Maximum : 163 entités allouées, 26 actives. Résultats : `workload-results/20261005-154253/`.

Les deux nouveaux runs se terminent sans erreur. Leur durée courte ne permet pas d'exclure un plateau ultérieur. Les profils diffèrent par plusieurs actions (déplacement, attaques, sorts, shop) : ils isolent l'activité de jeu, pas un système individuel. Le nombre d'entités et les compteurs réseau ne suffisent pas à mesurer toutes les allocations internes.

## Rétentions confirmées par lecture du code

### 1. Les ralentissements expirés restent dans `ActiveSlows`

`server/src/simulation/systems/spells.rs:570` ajoute chaque application à un `Vec`. `update_active_slows`, ligne 628, parcourt et décrémente les entrées sans jamais supprimer celles qui expirent. Les entrées visant une entité disparue restent également présentes.

Conséquence : la mémoire dépend du nombre cumulé d'applications de Slow pendant la vie du serveur, et non du nombre d'effets actifs. Corriger implique de purger les effets expirés et les cibles disparues.

Ce défaut ne peut pas expliquer le run de combat existant : `assets/config/spell.json` configure uniquement Fireball avec Damage, Knockback et Burn, sans Slow.

### 2. Les données de navigation et de shop survivent à la déconnexion

`server/src/simulation/systems/flow_field.rs:25` crée un FlowField par entité joueur dans `FlowFieldManager.fields`. Le traitement de déconnexion (`server/src/net/mod.rs:47`) retire le joueur du registre et du monde, sans retirer son champ de navigation. Chaque nouvelle entité joueur peut donc laisser les deux tableaux de son champ dans la map. Leur charge utile représente environ 12 octets par case de grille, hors overhead.

`server/src/simulation/resources/shop.rs:28` conserve aussi les inventaires par client. Acheter retire un item ; cela ne retire pas l'entrée client à la déconnexion. La rétention est faible par client, mais croît avec les identifiants distincts.

Aucun de ces mécanismes ne justifie une pente continue avec les mêmes quatre joueurs sans reconnexions. La réapparition réutilise l'entité existante.

### 3. Legion conserve les blocs de localisation des entités détruites

Dans la dépendance locale Legion 0.4.0, `src/internals/entity.rs`, `LocationMap::insert` crée des blocs d'identifiants ; `LocationMap::remove` vide la localisation sans supprimer le bloc devenu vide. Les créations/destructions d'attaques et de projectiles peuvent donc faire augmenter ces métadonnées malgré un `world.len()` stable.

Cette rétention est confirmée dans l'implémentation consultée, mais son volume dans ces workloads n'est pas mesuré. Elle ne doit pas être présentée comme l'explication des 300 Mio sans comptage des blocs/allocations. Piste : mesurer ce coût, puis évaluer le recyclage des entités temporaires ou une évolution de la dépendance.

## Allocations récurrentes à investiguer, sans fuite démontrée

### Buffers vidés avec `mem::take` puis non restitués

- `server/src/replication/snapshot.rs:141` transfère le vecteur au snapshot. Sa capacité n'est pas rendue au BufferManager après l'envoi : elle est libérée avec le snapshot. À quatre clients et vingt ticks/seconde, ce chemin tourne jusqu'à quatre-vingts fois par seconde.
- `server/src/simulation/systems/attack.rs:257` et `coin.rs:133` prennent le vecteur de candidats, puis libèrent l'emplacement sans y restituer ce vecteur.
- `server/src/simulation/resources/shop.rs:52` utilise le même schéma pour les actions.

Le recyclage annoncé est donc perdu sur ces chemins. Cela augmente les allocations/libérations, mais les vecteurs locaux sont détruits : aucune fuite directe n'est démontrée. La stabilité du test idle, qui envoie également des snapshots, indique que ce chemin seul n'explique pas la croissance de combat.

### Vérification isolée de mem::take

Test avec le vrai BufferManager : 100 032 passages, buffer de 4 Kio, processus distincts pour destruction et restitution du vecteur pris. Sans restitution : 409 733 962 octets cumulés alloués (~391 Mio), mais seulement +24 octets de tas vivant et +76 Kio de mémoire privée depuis la baseline. Avec restitution : 6 998 octets cumulés alloués, +4 120 octets de tas vivant (buffer conservé), aucune hausse de mémoire privée. Résultats : `workload-results/mem-take-isolation/measurements.json`. Relance : `python scripts/legion_memory_probe.py --modes take recycle --output workload-results/mem-take-isolation-new`.

Ce test confirme une forte pression d'allocation sans accumulation dans ce cas isolé. Il ne reproduit pas les tailles variables, la concurrence ou les allocations imbriquées du serveur. Deux autres vecteurs non recyclés existent dans `server/src/replication/event.rs:36` et `:46` : les événements ciblés et généraux sont consommés par valeur après take, donc leurs allocations sont libérées. Le chemin physics restitue correctement ses deux vecteurs au manager. Le take des compteurs réseau porte sur une structure de compteurs et ne constitue pas une allocation de buffer. Aucun correctif de ces chemins appliqué pendant cette vérification.

### Autres allocations temporaires et capacités conservées

`SpatialGrid::build` clone les offsets (`server/src/navigation/spatial_grid.rs:75`). Les calculs de FlowField recréent une liste de visites et une file temporaire (`utils/src/map/flow_field.rs:42`). Les collisions et AOE créent également des vecteurs temporaires. Le BufferManager peut remplacer un buffer par un autre type selon l'ordre d'acquisition ; il conserve les capacités via `clear` lorsqu'un buffer est réutilisé.

Ces mécanismes peuvent contribuer à la pression d'allocation et au maintien d'un pic de mémoire. Leur fréquence ne suffit pas à expliquer une hausse continue : une mesure du tas est nécessaire.

## Ce qui n'est pas établi comme fuite

Les queues usuelles d'événements/dégâts sont vidées ou drainées. Les burns expirés sont retirés. Les pools ennemis/pièces sont bornés dans cette configuration. Les canaux Renet ont des limites mémoire ; les messages de debug augmentent les allocations et le trafic, mais leur nombre ne prouve pas un backlog illimité. Les fichiers JSONL volumineux sont écrits sur disque par les outils, pas conservés comme historique dans la RAM serveur.

## Prochaine vérification pour attribuer la hausse principale

### Mesure des allocations du serveur réel

Instrumentation optionnelle via la feature Cargo `allocation-metrics`. Le binaire serveur utilise un GlobalAlloc qui délègue à System sans allouer pour ses compteurs. Il compte les octets alloués/libérés, le tas encore vivant et le nombre d'appels réussis (realloc compris). Les relevés par phase et cumuls sont enregistrés dans `server-ticks.jsonl` et agrégés dans `summary.json`. Les exemples/bots ne changent pas d'allocateur. Sans cette feature, les compteurs sont signalés désactivés.

Test de quatre joueurs en combat, inputs 60 Hz, 90 secondes, sans erreur : `workload-results/allocation-combat-90s/`. En excluant les trois premières secondes, le tas vivant passe de 31 958 955 à 187 742 043 octets (30,48 → 179,04 Mio, soit +148,57 Mio). La mémoire privée du processus passe approximativement de 32,4 à 183,0 Mio dans la plage mesurée par Python. La hausse correspond donc majoritairement à des allocations encore vivantes, plutôt qu'à la seule conservation par Windows de mémoire déjà libérée.

Sur cette fenêtre, la simulation effectue 66 179 606 appels d'allocation/reallocation et alloue 4 646 075 328 octets cumulés (4,33 Gio), dont 4 488 659 726 libérés (4,18 Gio). Son solde de tas est +157 415 602 octets (+150,12 Mio). Autres soldes par phase : réception réseau -10,95 Mio, commandes -0,38 Mio, événements +9,26 Mio, snapshots +1,65 Mio, envoi réseau -0,96 Mio, nettoyage 0. Les sommes incluent le tick de début de fenêtre, tandis que la différence des cumuls part de la fin de ce tick ; elles ne s'alignent donc pas exactement. Les allocations de l'observateur hors phases figurent aussi dans les cumuls globaux.

Ces soldes indiquent quand les allocations apparaissent/disparaissent ; ils n'attribuent pas la propriété d'une allocation à une phase. Par exemple, le réseau peut libérer au tick suivant un message créé pendant les événements. Les compteurs sont globaux, comprennent les threads parallèles et leurs relevés ne sont pas des instantanés atomiques de l'ensemble des champs. La simulation est néanmoins très nettement la phase qui explique le solde positif persistant. Il faut désormais distinguer ses systèmes et les objets conservés pour obtenir le site exact. Les compteurs n'incluent pas directement les piles de threads, les allocations natives qui contournent cet allocateur, ni l'overhead du tas Windows.

Relance avec recompilation : `python scripts/workload.py --players 4 --seconds 180 --hz 60 --profile combat --allocation-metrics`. La feature ajoute des compteurs atomiques à chaque allocation : les temps de traitement ne doivent pas être comparés directement à un run sans instrumentation. Validation : 36 tests Rust et 3 tests Python passent, dont un test dédié allocation/zeroing/reallocation/alignement/libération. Aucun correctif de gameplay appliqué.

### Résultat du test isolé Legion

Un exemple autonome, `server/examples/legion_memory_probe.rs`, teste la version Legion du projet sans importer la logique serveur, ses ressources, le réseau ou ses métriques. Un allocateur instrumenté délègue à `std::alloc::System` et compte les octets demandés encore vivants ainsi que le volume cumulé alloué. Le pilote `scripts/legion_memory_probe.py` mesure également RSS/mémoire privée pendant que le processus attend à chaque checkpoint. Chaque mode utilise un processus neuf. Les créations/destructions sont accélérées, sans cadence de jeu : on compare le coût par opération, pas une pente en secondes.

Le probe maintient 150 entités persistantes. Les modes de destruction ajoutent puis retirent Position, Velocity, Active et Lifetime, avec des transitions de composants. Le mode pool réutilise 64 entités supplémentaires avec leurs composants fixes ; il teste le contournement par recyclage, pas les transitions de composants du mode destruction.

Résultats du premier passage (`workload-results/legion-isolation/measurements.json`), variation du tas vivant depuis la baseline :

- `World::push` direct : +2 979 982 octets à 7 040 opérations ; +40 638 606 octets à 100 032 opérations (38,76 Mio). Mémoire privée : +42,03 Mio à 100 032.
- `CommandBuffer` persistant : +196 750 octets à 7 040 opérations ; +2 550 414 octets à 100 032 opérations (2,43 Mio). Mémoire privée : +2,93 Mio à 100 032.
- Système exécuté via `Schedule` : +260 490 octets à 7 040 opérations ; +2 614 154 octets à 100 032 opérations (2,49 Mio). Mémoire privée : +3,50 Mio à 100 032.
- Pool de 64 entités : aucune croissance du tas vivant ni de la mémoire privée après 100 032 réutilisations.

Le nombre d'entités vivantes reste 150 dans les trois premiers modes et 214 dans le pool. Après destruction du World et des commandes/schedule/ressources, le compteur revient près du niveau avant leur création : environ 13 Kio résiduels dans les modes directs/commandes/pool et 73 Kio dans le mode schedule, qui initialise aussi le runtime parallèle. Les compteurs englobent le processus, notamment les buffers d'I/O ; ces résidus ne sont pas attribués précisément à Legion. La mémoire privée peut conserver davantage que le tas vivant après destruction.

L'écart entre `World::push` et CommandBuffer concorde avec l'allocation d'identifiants par blocs de 16 : `World::push` renouvelle son Allocate via extend, alors que CommandBuffer conserve son Allocate. Les blocs de localisation restent dans le World jusqu'à sa destruction. Les attaques/projectiles de production utilisent `command.push`, pas `World::push` à chaque attaque.

Conclusion : le test confirme et quantifie la rétention des métadonnées, ainsi que l'efficacité du recyclage dans ce cas. Il ne reproduit pas les centaines de Mio du serveur sur le chemin CommandBuffer/Schedule, même après 100 000 opérations. Legion n'est donc pas identifié comme cause principale. Ce probe ne couvre pas tous les composants, requêtes et accès parallèles de la production ; une interaction spécifique reste possible.

Pour relancer : `python scripts/legion_memory_probe.py --output workload-results/legion-isolation-new`. Le pilote compile offline avec le lockfile, puis exécute les quatre modes.

Mesurer les octets d'allocations encore vivants par site/phase pendant le combat, ainsi que la taille/capacité des ressources et les blocs de localisation Legion. Puis comparer des workloads séparant déplacement, attaque normale et sorts. Cette étape permettra de distinguer une vraie accumulation d'objets de la rétention/fragmentation du tas Windows avant de choisir un correctif.
