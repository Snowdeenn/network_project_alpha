# Exemples de familles VFX — proposition de format v1

Ces cinq JSON prolongent l'exemple `impact_embers` fourni dans la conversation. Ils constituent des exemples de conception, pas des schémas définitifs ni une garantie de compatibilité avec un moteur existant. Aucun JSON Schema n'est imposé.

## Fichiers

- `mesh.json` : anneau d'onde de choc qui grandit puis s'atténue.
- `trail.json` : ruban d'énergie échantillonné à la pointe d'une arme.
- `decal.json` : marque d'impact fixe qui s'efface.
- `light.json` : éclair lumineux dont l'intensité décroît.
- `post-process.json` : impulsion visuelle appliquée à l'image finale.

## Conventions reprises du particle

Chaque fichier conserve `version`, `name`, `family`, ainsi que la séparation `logical` / `visual`. Les clés utilisent le `snake_case`. Les durées sont en secondes, les angles en degrés. Les durées de vie gardent la forme `{ "min": ..., "max": ... }`, avec des bornes égales pour rendre ces exemples déterministes.

`logical` décrit la création, l'initialisation, l'évolution éventuelle et la mort de l'instance. `visual` décrit les représentations, l'apparence initiale, les états, les transitions et la disparition. Les blocs inutiles sont omis : aucune animation flipbook n'est nécessaire ici, et le post-process n'a ni émission spatiale ni position.

Il n'y a ni script, ni graphe de composition, ni nœuds ou liens entre effets. Les références de représentation et les transitions sont internes à un seul effet ; `weapon_tip` désigne un point d'attache fourni par l'appelant.

## Hypothèses communes à confirmer

- L'appelant crée une instance et démarre son horloge à zéro. Pour mesh, decal et light, `burst` crée un seul objet. Pour le post-process, l'instance est activée directement.
- `normalized_age` est l'âge de l'instance divisé par sa durée de vie logique. Une transition est déclenchée une seule fois depuis son état source. Elle interpole linéairement les propriétés numériques depuis leurs valeurs courantes vers `target`, puis entre dans l'état destination à la fin. Un seuil de zéro permet de commencer immédiatement.
- Les propriétés absentes de `target` restent inchangées. Les objets imbriqués tels que `parameters` sont modifiés propriété par propriété. Les états vides sont des états nommés valides : leur apparence vient de `appearance` puis des transitions.
- Les entrées de `logical_death.when` sont alternatives : l'une d'elles suffit. La mort logique interrompt une transition en cours ; `visual_disappearance` reprend les valeurs courantes et libère la ressource seulement à sa fin. Elle ne redémarre pas le rendu depuis sa valeur initiale.
- Les positions et tailles sont en unités de scène arbitraires. Pour rester proche des vecteurs du particle, les exemples spatiaux supposent un plan 2D ; le mesh est planaire et le decal est une empreinte sur ce plan. Une version 3D devrait préciser les axes, les normales et la projection du decal.
- Les couleurs des surfaces sont RGBA entre 0 et 1 ; la lumière utilise RGB et une intensité séparée. L'espace colorimétrique reste à définir. Les modes de mélange restent des intentions à adapter au moteur.
- Les chemins de textures, meshes et matériaux sont fictifs et les ressources ne sont pas incluses. Les extensions et les noms des nouveaux types sont proposés pour illustrer le format.

## Choix spécifiques aux familles

**Mesh.** `mesh` et `material` désignent une géométrie et son matériau. `size` représente sa taille affichée dans le plan, sans modifier une éventuelle collision. Le mesh reste immobile ; seule son apparence change.

**Trail.** Le cycle de vie logique et l'âge normalisé concernent le ruban complet. L'émission continue ajoute des points à `weapon_tip` pendant 0,45 seconde, à 60 points par seconde. Chaque point capture sa position dans le monde et expire après 0,35 seconde. `max_points` est une limite ; un dépassement retire les points les plus anciens. Le ruban meurt à 0,8 seconde. `width` et `color` s'appliquent à tout le ruban, et `uv_mode: "stretch"` étire la texture sur les points encore présents. La disparition est également utilisée en cas de mort anticipée : elle fige alors la géométrie restante. À l'expiration normale, le ruban peut déjà être vide.

**Decal.** L'exemple suppose une empreinte plane fixe, sans requête de collision ni sélection automatique d'une surface. Il ne définit pas de projecteur 3D ni de profondeur de projection.

**Light.** `point_light`, `radius` et `casts_shadows` sont des propositions de paramètres. L'intensité est un multiplicateur relatif, pas une grandeur photométrique en lumens. Le rayon est en unités de scène. L'extinction agit sur l'intensité, sans alpha de surface.

**Post-process.** Un seul matériau traite la vue active fournie par l'appelant. `parameters` décrit ses paramètres attendus : les valeurs nulles sont supposées neutres. `weight` mélange le résultat de ce passage avec son entrée, de 0 (aucun effet) à 1 (plein effet). Le décalage chromatique est exprimé en pixels de la cible de rendu. L'ordre des passages et le mélange de plusieurs instances restent hors de ces exemples.

Les champs nouveaux (`point_lifetime_seconds`, `width`, `radius`, `parameters`, etc.) et les actions `release_mesh`, `release_trail`, `release_decal`, `release_light`, `release_post_process` sont des extensions proposées du vocabulaire particle. Ils restent à valider avec l'implémentation. Les interpolations sont portées par les transitions, sans ajouter de règle d'interpolation aux animations.

Le token `test1.0` dans la couleur de l'exemple particle fourni semble être une coquille et rend ce texte invalide en JSON. Il n'est pas repris ici ; les cinq exemples utilisent exclusivement des valeurs JSON valides.

## Vérification

Les fichiers sont vérifiés pour leur syntaxe JSON, leurs familles, leurs références internes et la cohérence temporelle de leurs transitions. Leur exécution et leur rendu nécessitent un moteur et les ressources correspondantes ; ils n'ont pas été testés dans un moteur.
