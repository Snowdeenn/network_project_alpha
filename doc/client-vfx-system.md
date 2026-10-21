# Système VFX client

Implémentation du 21 octobre 2026, à partir de `CLIENT_VFX_ARCHITECTURE.md`
et d'une lecture des sources avant modification. Le document décrit une cible ;
ses exemples de noms et son format RON ne remplacent pas les équivalents existants.

## Bases conservées

- `vfx/parser/` décrit déjà les particules, meshes, decals, trails, lumières et
  post-process en JSON. Le format `version/name/family/logical/visual` est conservé.
- `vfx/loader.rs` possède déjà le chargement asynchrone, les erreurs, un délai
  maximal et l'annulation. Ce chargeur est conservé.
- `utils::Arena` et `utils::Id` fournissent déjà les emplacements réutilisables
  et les générations. Les handles publics réutilisent ces types.
- `VfxInstanceManager`, `ParticlePool` et `VfxSimulation` possèdent déjà les
  horloges d'émission, les durées de vie, les transitions et les disparitions.
- `VfxGraph` sait déjà séquencer les instances et sous-graphes. Seul un délai
  initial par nœud a été ajouté pour les composites décrits par des marqueurs.
- Les effets gameplay sont décrits dans `assets/vfx/gameplay/` et enregistrés
  au démarrage par `register_gameplay_effects()`. `rendering/vfx/`, son pool de
  512 particules, `VfxManager` et `compatibility.rs` ont été supprimés.

Poussière, impacts, slashes, fantômes, trails, flashes d'entités, flash d'écran,
blind et demandes de secousse utilisent maintenant les mêmes instances,
révisions, budgets et règles de nettoyage que les autres effets. Le rendu est
extrait une seule fois et soumis à Prism V2. Les anciens timers de flash et
de blind sont remplacés par les simulations post-process du runtime.

## Responsabilités

`VfxApi` partage une file bornée sur le thread client, par `Rc<RefCell<_>>`,
cohérente avec `Resources`. Elle ne donne accès ni aux instances ni au GPU.
Le réseau, les entrées et les animations peuvent conserver chacun son clone.

`VfxRegistry` résout un nom d'asset vers une révision immuable partagée en `Arc`.
`AssetRegister` conserve les descriptions existantes. Une révision composite
retient celles de ses enfants ; une occurrence retient sa définition jusqu'à
extinction. Recharger un enfant ne modifie donc pas un composite déjà publié :
recharger aussi ce composite pour utiliser la nouvelle référence.

`VfxSystem` possède les occurrences complètes et réutilise le manager existant
pour leurs composants. Il traite les demandes, résout les attachements, avance
les graphes et la simulation, puis recycle les effets terminés. Un handle désigne
toute l'occurrence composite ; un arrêt s'applique aussi aux composants différés.

`extract()` produit un snapshot possédé. `render_bridge::append_to_frame()`
le convertit en primitives client dans `rendering::SceneFrame`. Le backend produit
les objets et uploads Prism V2 ; le runtime ne crée aucun pipeline ni binding.

## Chargement et utilisation

Un registre créé seul démarre vide ; le jeu y enregistre ses presets gameplay.
Charger les autres assets nécessaires à la préparation d'une
scène, avant de demander leur spawn. `registry.load()` est une commodité synchrone,
à ne pas appeler dans la boucle de rendu. `AssetLoader` charge et valide les
descriptions élémentaires en arrière-plan ; `registry.register_loaded()` publie
leur résultat sans relire ni parser à nouveau le fichier.

```rust
use client::vfx::{VfxSpawnParams, VfxSystem};
use client::vfx::registry::Priority;

let mut system = VfxSystem::default();
system.registry.load(
    "assets/vfx/exemples_vfx/particle.json",
    Priority::Normal,
)?;
let api = system.api();
let ticket = api.spawn("impact_embers", VfxSpawnParams::default())?;
```

Pour le chargement asynchrone, conserver le runtime Tokio, le chargeur et
l'identifiant de la requête entre les frames :

```rust
use client::vfx::{AssetLoadContext, AssetLoader};

let runtime = tokio::runtime::Builder::new_current_thread().build()?;
let mut loader = AssetLoader::new();
let mut pending_load = Some({
    let _guard = runtime.enter();
    loader.request_load(AssetLoadContext::new(
        "assets/vfx/exemples_vfx/particle.json",
    ))
});
```

Puis une fois par frame, avant les requêtes de spawn :

```rust
runtime.block_on(loader.run_request());

if let Some(request) = pending_load {
    match system.registry.register_loaded(&mut loader, request, Priority::Normal) {
        Ok(false) => {} // Encore en cours : conserver la requête.
        Ok(true) => {
            pending_load = None;
            // L'effet est maintenant disponible dans system.registry.
        }
        Err(error) => {
            pending_load = None;
            tracing::error!("Chargement VFX échoué : {error}");
        }
    }
}
```

`run_request()` collecte seulement les tâches terminées ou expirées. La validation
est commune aux chemins synchrone et asynchrone. Une erreur de lecture, de parsing
ou de validation est remontée par `register_loaded()` avec le chemin du fichier,
sans remplacer la dernière révision valide. Les résultats sont consommés une seule
fois : une requête annulée ou déjà consommée renvoie `InvalidInput`.
`LoadRequestId` et `LoadingState` sont aussi exportés pour les contextes qui
stockent plusieurs requêtes ou inspectent leur progression. Les composites
continuent à être publiés par `register_json()` après leurs enfants.

Dans le jeu, `Resources` contient déjà `VfxApi` et `VfxSystem`.
L'appelant fournit position, direction, échelle et durée d'émission continue
via `VfxSpawnParams`. Les champs optionnels `size`, `duration`, `tint`, `velocity`
et `entity` adaptent les surfaces, durées mesh/post-process/feedback, teintes,
vitesses initiales et propriétaire. Les durées de vie des particules viennent
des assets. `SpawnOptions` ajoute attachement, répétition, clés d'événement et
de prédiction, ainsi que `refresh` pour remplacer un effet nommé du même
propriétaire ou alimenter un trail manuel existant.

```rust
match api.take_result(ticket) {
    Some(client::vfx::SpawnResult::Spawned(handle)) => {
        let command = api.stop(handle, client::vfx::StopMode::Drain)?;
        // Après le prochain traitement : take_command_result(command).
    }
    Some(client::vfx::SpawnResult::Rejected(reason)) => {
        // Effet inconnu, paramètres invalides, budget ou annulation.
    }
    None => {} // Pas encore traité, ou résultat déjà consommé.
}
```

`stop()` et `update()` retournent un ticket de commande. Son résultat distingue
succès, handle périmé et paramètres invalides. Les résultats se consomment une
seule fois. Les résultats non lus occupent une place bornée : les consommateurs
doivent les récupérer. La saturation refuse la nouvelle demande avec `QueueFull`
et incrémente un compteur. `cancel_pending(ticket)` fonctionne même file pleine.
Les méthodes gameplay utilisent `spawn_detached()` : ces demandes ponctuelles
ne réservent aucun résultat à consommer. Elles restent bornées par la file et
par tous les budgets d'admission. `stop_named()` arrête les occurrences d'un
effet par leur nom, notamment pour les aperçus de debug.

## Presets gameplay migrés

- `movement_dust()` : une particule grise, positionnée sous les pieds, avec
  dispersion cartésienne, vitesse opposée au déplacement, drag, croissance et fondu.
- `entity_hit()` : composite de dix particules dorées et d'une impulsion caméra.
- `slash()` : mesh annulaire orienté par la direction, radii fournis par les
  dimensions de l'attaque, ouverture de 120 degrés et durée de 0,15 seconde.
- `dash_ghost()` : quad teinté de 40 pixels avec fondu sur la durée demandée.
- `trail_point()` : trail manuel par propriétaire, borné à 12 points ; les
  points restent en coordonnées monde et leur largeur/alpha diminuent sur 0,12 seconde.
  Une nouvelle demande conserve l'historique, puis l'absence de points ou la
  disparition du propriétaire permet de libérer l'occurrence.
- `entity_flash()` : feedback de teinte, renouvelé par entité ; les sprites
  utilisent un matériau V2 qui remplace leur couleur et conserve leur transparence.
- `player_hit()` / `screen_flash()` et `blind()` : post-process avec enveloppes
  de poids, renouvellement et arrêt pilotés par le runtime. Le blind conserve
  ses fondus d'entrée/sortie d'une seconde et son rayon de visibilité.
- `camera_shake()` : feedback ponctuel ; l'impulsion est consommée une seule
  fois par la scène. La caméra reste responsable de son mouvement et avance
  la décroissance avec le delta réel de la frame.

Le réseau et les boutons de debug utilisent ces mêmes API. Le protocole
`EntityHit` ne fournit pas l'identité de la victime : aucun flash d'entité
supplémentaire n'est déclenché en devinant son propriétaire. Trails et fantômes
restent disponibles pour les producteurs disposant des événements d'animation
ou de déplacement appropriés.

## Composites, arrêt et attachement

`exemples_vfx/composite.json` montre un composite JSON. Charger ses enfants avant
le parent. Chaque composant désigne un effet et un `delay_seconds`. Une référence
inconnue, un cycle de noms, plus de huit niveaux ou plus de 64 feuilles est refusé.
Les composites peuvent aussi être construits par `register_composite()`.

La simulation tourne à 60 Hz, avec au plus huit sous-pas par frame. Les marqueurs
retardés s'activent sur le premier sous-pas qui atteint leur échéance ; la
précision temporelle est donc celle du pas fixe. Le temps excédentaire est perdu
et compté dans `discarded_seconds`. `with_seed()` rend les tirages reproductibles
à appels et sous-pas identiques sur une même version des dépendances.

`Drain` empêche les nouvelles émissions et les composants encore en attente,
puis laisse disparaître les éléments présents. `Immediate` supprime aussi les
particules appartenant à l'occurrence. `looping` rejoue la séquence après
extinction complète ; le handle reste stable et un arrêt empêche le redémarrage.

Un attachement conserve un identifiant d'entité et un offset. La scène fournit
la position interpolée en lecture. À disparition du propriétaire, la politique
choisie est `Drain`, `Detach` ou `Immediate`. Les particules déjà émises dans le
monde restent à leur position simulée. La façade ne rajoute pas de collisions,
de résolution de socket nommé ou de simulation locale des particules aux
capacités de la simulation préexistante.

## Validation, rechargement, prédiction et budgets

La publication vérifie la version, le nom, les nombres finis représentables en
`f32`, l'ordre des plages, les durées et limites de compte, ainsi que les
références internes de représentation et d'état. Les temps sont limités à
60 secondes et les comptes/taux/dimensions à 4096. Le chargement par fichier
ajoute le chemin au diagnostic. Une erreur conserve la dernière révision valide.
Un nouvel appel à `load()` ou `register_json()` effectue le rechargement ; aucun
watcher de fichiers supplémentaire n'est introduit.

`VisualEventKey { session, event }` déduplique un événement. Une
`PredictionKey { session, action }` associe la prédiction à sa confirmation.
Une confirmation garde le même handle ; un rejet supprime l'occurrence et
mémorise son annulation. Les caches sont bornés à 2048 clés, avec une rétention
de dix secondes ; les prédictions actives restent retenues au-delà de ce délai,
dans la limite de capacité. Le protocole `EntityHit` actuel ne transporte pas
d'identifiant d'événement/action : la scène n'invente pas de clé à partir de la
position. Ces clés sont destinées aux producteurs disposant d'une identité fiable.

Les budgets du runtime limitent demandes, occurrences, composants, particules,
éléments de surface/lumière/post-process et points de trails. L'admission réserve
le maximum émis pendant la séquence, y compris les enfants différés. Elle est
volontairement conservative. Une boucle garde sa réservation entre ses cycles.
Les effets `Low` ne consomment que les premiers 80 % du budget d'occurrences ;
les autres peuvent utiliser le reste. Aucune éviction ou modulation automatique
des taux n'est ajoutée. Tous les effets gameplay utilisent ces budgets communs.

Les diagnostics exposent admissions, refus, doublons, handles périmés,
changements invalides, actifs/pic, saturation, temps perdu et composants dont
le rendu manque. Ils sont accessibles par `VfxSystem::diagnostics`.

## Rendu disponible

Les particules, meshes et decals utilisent des quads, les meshes de représentation
`ring` utilisent une géométrie annulaire et les trails utilisent des segments.
La couleur, la taille, la disparition et le blend proviennent des
descriptions et de la simulation. Les angles en degrés sont convertis en radians
à la frontière Prism.

Les textures, atlases et matériaux fictifs des exemples ne sont pas chargés.
Les lumières restent des intentions inspectables et leur absence de rendu est
comptée à l'admission. Les post-process sont résolus par nom de matériau dans
le renderer ; un matériau absent est signalé une seule fois, sans bloquer les
autres effets. `register_vfx_post_effect()` associe librement un nom, un shader et
un encodeur d'uniforme typé. Le renderer transmet chaque occurrence avec ses
paramètres, son poids et les dimensions courantes. Les noms `hit_flash` et `blind`
sont des enregistrements ordinaires dans l'initialisation du jeu.

Il n'existe plus de structure contenant un champ par effet. L'état d'un matériau
se consulte avec `system.post_process("nom")` : poids maximal et durée restante.
Les contrôles directs ajout/écriture/activation restent disponibles ; voir
[l'intégration Prism V2](renderer-prism-v2.md#ajouter-un-effet-décran).

## Vérification

Les tests du runtime couvrent tickets,
saturation, commandes périmées, arrêt avec résidus, arrêt avant activation,
répétition, rechargement, attachements, budget partagé, composites JSON et cycles,
prédiction, clés de session, pas fixe, seed et indépendance des snapshots. Les
tests gameplay vérifient aussi les presets migrés, la croissance/fade, les
impulsions uniques, les refus atomiques, le renouvellement et le nettoyage des
trails, les fondus du blind, les flashes et l'absence de résultats non lus.
Le test GPU vérifie les pixels du slash, du fantôme, du trail, du flash d'entité
avec masque alpha et du flash d'écran piloté par le runtime.

```sh
cargo test --workspace --offline
cargo test -p client --test prism_v2_rendering --offline -- --ignored
```
