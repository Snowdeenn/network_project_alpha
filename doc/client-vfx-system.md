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
- `rendering/vfx/` possède le rendu historique : pool de 512 particules,
  slashes, flashes d'entité, trails d'épée et fantômes de dash. La caméra possède
  déjà le trauma et le post-process possède déjà le flash d'impact.

Les paramètres des effets utilisés en jeu restent dans `compatibility.rs`,
avec les mêmes calculs et le même backend. Leurs producteurs passent désormais
par `VfxApi`. Un impact réseau ne déclenche donc pas aussi `impact_embers`.
Les flashes d'entité, trails et fantômes disponibles dans l'ancien manager
restent accessibles par la façade ; leurs déclencheurs métier et le rendu de
l'override d'entité ne sont pas inventés à partir d'événements incomplets.

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
le convertit en commandes Prism. Le runtime ne crée aucun pipeline ni binding.

## Chargement et utilisation

Le registre démarre vide. Charger les assets nécessaires à la préparation d'une
scène, avant de demander leur spawn. `registry.load()` est une commodité synchrone,
à ne pas appeler dans la boucle de rendu. Le chargeur asynchrone existant reste
disponible dans le module pour les chargements en arrière-plan.

```rust
use client::vfx::{VfxSpawnParams, VfxSystem};
use client::vfx::registry::Priority;

let mut system = VfxSystem::default();
system.registry.load(
    "assets/config/vfx/exemples_vfx/particle.json",
    Priority::Normal,
)?;
let api = system.api();
let ticket = api.spawn("impact_embers", VfxSpawnParams::default())?;
```

Dans le jeu, `Resources` contient déjà `VfxApi` et `VfxSystem`.
L'appelant fournit position, direction, échelle et durée d'émission continue
via `VfxSpawnParams`. Les durées de vie des éléments proviennent des assets.
`SpawnOptions` ajoute attachement, répétition, clé d'événement et clé de prédiction.
La direction est conservée pour les familles qui l'exploitent ; les calculs
spatiaux de la simulation existante ne sont pas réinterprétés par la façade.

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
Les méthodes de compatibilité sont des demandes ponctuelles sans ticket.

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
des taux n'est ajoutée. Le backend historique garde ses limites propres.

Les diagnostics exposent admissions, refus, doublons, handles périmés,
changements invalides, actifs/pic, saturation, temps perdu et composants dont
le rendu manque. Ils sont accessibles par `VfxSystem::diagnostics`.

## Rendu disponible

Le rendu historique reste inchangé. Pour les assets du nouveau runtime, les
particules, meshes et decals utilisent des quads ; les trails utilisent des
segments. La couleur, la taille, la disparition et le blend proviennent des
descriptions et de la simulation. Les angles en degrés sont convertis en radians
à la frontière Prism.

Les textures, atlases et matériaux fictifs des exemples ne sont pas chargés.
Les lumières et post-process sont extraits comme intentions mais ne créent pas
de passe GPU ; leur absence est comptée à l'admission. Cela permet d'exécuter un
composite sans supprimer ses composants rendables. Le flash et le shake actuels
continuent d'utiliser leurs systèmes dédiés. La parité visuelle se vérifie en jeu ;
les tests CPU ne la prouvent pas.

## Vérification

Les tests existants sont conservés. Les nouveaux tests couvrent tickets,
saturation, commandes périmées, arrêt avec résidus, arrêt avant activation,
répétition, rechargement, attachements, budget partagé, composites JSON et cycles,
prédiction, clés de session, pas fixe, seed et indépendance des snapshots.

```sh
cargo test --workspace --offline
```
