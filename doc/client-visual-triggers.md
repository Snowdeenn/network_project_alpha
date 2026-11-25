# Déclenchement et chargement des VFX

La couche client du jeu relie les événements aux effets. Le moteur `VfxSystem`
continue à recevoir un nom, `VfxSpawnParams` et `SpawnOptions` complets.

## Données

- `assets/config/vfx_catalog.json` associe les noms des effets à leurs fichiers,
  relativement à `assets/vfx`. Le nom déclaré dans le fichier doit correspondre.
- `assets/config/visual_profiles.json` déclare des profils avec un identifiant
  numérique stable, un nom lisible et des règles. Les identifiants ne dépendent
  pas de l'ordre des fichiers et ne doivent pas être réattribués.
- Le champ optionnel `visual_profile` d'un sort sélectionne son profil. Le
  serveur le copie sur le projectile via le composant générique `VisualProfile`.
  Ce composant peut aussi être ajouté à un joueur, ennemi ou objet répliqué.

Les profils 1 et 2 illustrent deux projectiles différents (feu et glace). Les
profils 10 à 14 servent aux adaptateurs locaux : impact générique, mêlée, flash
d'écran, poussière et aveuglement. Les presets de l'API directe restent utilisables,
notamment pour le panneau de debug.

## Triggers et actions

Les triggers disponibles sont `spawn`, `despawn`, `cast`, `impact`, `hit`,
`death`, `move`, `stop` et `{"custom":"nom"}`. Déclarer une règle ne crée pas
automatiquement un événement gameplay : un adaptateur doit appeler `emit`.

Chaque règle possède plusieurs actions `spawn` ou `stop`. Exemple :

```json
{
  "id": 1,
  "name": "projectile_feu",
  "rules": [{
    "trigger": "spawn",
    "actions": [{
      "kind": "spawn",
      "effect": "fireball",
      "params": { "lifetime": 10, "scale": 1.2 },
      "options": { "refresh": false, "looping": false },
      "bindings": {
        "entity": true,
        "attachment": {
          "offset": { "x": 0, "y": 4 },
          "on_missing": "immediate"
        }
      },
      "track_as": "flight",
      "max_delay_seconds": null
    }]
  }]
}
```

`params` accepte tous les champs de `VfxSpawnParams` ; `options` accepte tous
ceux de `SpawnOptions`, y compris attachement explicite, clés d'événement et
prédiction. Les champs absents utilisent les valeurs par défaut de l'API.

Les bindings résolvent les informations du contexte : `position` et `direction`
sont activés par défaut ; `entity` est désactivé. `attachment` est optionnel et
résout l'entité du contexte. Désactiver un binding conserve la valeur du JSON.
Un attachement demandé sans entité est refusé. Un effet d'écran ou un impact
dans le monde n'a pas besoin d'entité ni d'attachement.

`track_as` nomme une occurrence dans la portée `(scope, profil, nom)`.
Deux noms permettent plusieurs effets indépendants sur la même entité.
Une nouvelle occurrence sous le même nom remplace l'ancienne. Une action
`{"kind":"stop","tracked":"flight","mode":"immediate"}` annule aussi
une demande encore en chargement ou en attente de traitement par le moteur.
Le mode `drain` reste disponible.

## Personnalisation en Rust

Le trigger résout les bindings, puis appelle la fonction de personnalisation
avant de mettre la demande en attente. Les valeurs personnalisées sont conservées
pendant le chargement. L'attachement continue naturellement à suivre son propriétaire.

```rust,ignore
triggers.emit_with(
    &profiles,
    profile_id,
    &VisualTrigger::Cast,
    &context,
    |request| {
        request.params.scale = 1.5;
        request.params.velocity = velocity;
        request.options.refresh = true;
        request.options.attachment = Some(Attachment {
            entity: projectile_id,
            offset: Vec2::new(0.0, 4.0),
            on_missing: MissingOwner::Immediate,
        });
    },
);
```

`enqueue(ResolvedVfxSpawn)` permet aussi de fournir une demande entièrement
construite par le code, sans passer par une règle. `owner` indique une entité
connue de `sync` dont dépend la validité de la demande en attente ; `None`
permet les événements ponctuels indépendants. `scope` identifie la source pour
le suivi des occurrences nommées. L'API directe `VfxApi::spawn_with` reste libre.

## Cycle de vie et réseau

`sync` est appelé pour chaque snapshot reçu, y compris les snapshots intermédiaires
d'un lot. Il détecte les apparitions, disparitions et changements de profil.
Les positions d'attachement sont résolues par le moteur à partir des snapshots
interpolés. La disparition invalide les spawns liés au propriétaire encore en
attente ; les occurrences existantes suivent leur politique `on_missing`.

Les événements réseau `Visual` transportent un identifiant unique, le profil,
le trigger, l'entité optionnelle, la position et la direction. Ils sont
dédupliqués pendant 60 secondes, puis oubliés. Le serveur émet `cast` lors de
la résolution du sort et `impact` lors d'une collision de projectile, ou à la
position d'un sort ciblé. L'impact utilise la position du projectile au tick de
collision ; il ne s'agit pas d'un calcul supplémentaire de point de contact.
Une expiration de projectile produit une disparition sans impact.

Les demandes ponctuelles expirent après 0,5 seconde par défaut.
`max_delay_seconds: null` permet d'attendre sans délai fixe, notamment pour
le vol d'un projectile, tant que son propriétaire est présent. Un impact peut
jouer après la disparition de sa source puisqu'il n'en dépend pas.

L'exécuteur consomme les tickets et conserve les handles. La sortie de partie
annule les demandes et arrête les occurrences, puis efface les identités connues.

## Chargement et erreurs

`VisualAssets::ensure_loaded(name)` déduplique les demandes. `AssetLoader`
lit et parse les effets élémentaires et les composites en arrière-plan.
`VisualAssets::publish` charge les dépendances récursives et publie les enfants
avant leurs parents. Les cycles, noms incohérents et dépendances inconnues sont
signalés. Les chemins restent relatifs à la racine du catalogue.

Un échec ne publie pas le composite parent. Les enfants déjà publiés restent
réutilisables. `error(name)` expose l'erreur ; `retry_failed()` autorise une
nouvelle tentative explicite. Les demandes encore valides peuvent alors aboutir.
Le chargeur ne relance pas une lecture défaillante à chaque frame.

Le protocole réseau a changé : recompiler et utiliser ensemble le client et
le serveur de cette version. Le catalogue et les profils intégrés utilisent
`include_str!` ; modifier ces configurations nécessite une recompilation.
Les descripteurs d'effets sont lus depuis le disque par le chargeur.

Le déclenchement est commun aux familles VFX existantes. Les capacités de rendu
restent celles de l'adaptateur actuel (notamment les surfaces de remplacement
et l'absence de passe GPU pour les lumières).
