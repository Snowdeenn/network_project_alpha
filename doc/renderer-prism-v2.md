# Renderer Prism V2

Le client dépend directement de `prism-v2`, à la révision
`54a2a3da673371af99c325b2f29909d5e34f327f` de `refactor/prism_v2`.
Il n'instancie plus le renderer, les passes ou les gestionnaires GPU V1.

## Soumission

Les systèmes produisent une `rendering::SceneFrame` possédée, contenant les
primitives monde, VFX et HUD. `SceneRenderer` conserve la dernière scène en attente
du redraw. Il trie les couches de façon stable, tesselle les primitives et regroupe
les dessins consécutifs partageant un matériau. Les changements de texture ne
réordonnent pas les dessins transparents.

Le backend construit une `FrameV2` avec des `RenderObject`, des géométries indexées,
des vues et des uploads possédés. Les buffers de sommets et d'indices sont réutilisés
et agrandis au besoin. Les géométries et les matériaux temporaires de la frame
précédente sont libérés avant de préparer la suivante ; les commandes déjà soumises
conservent leurs ressources GPU.

La vue monde fournit une matrice finale de caméra, incluant le shake, par un
binding uniforme explicite. Les phases monde puis VFX utilisent cette vue. Le HUD
possède une vue en coordonnées écran et utilise la phase overlay après les effets.
Les matrices et les géométries sont transférées par `UploadRequest::Buffer`.

`RenderAssets` possède le `ResourceManager` V2, les textures, les templates, les
matériaux et les bindings. Les textures sont décodées avec `image`, chargées une fois
et réutilisées par chemin. Les uploads de textures d'assets ont lieu au chargement,
avant le rendu, tandis que les données dynamiques appartiennent à la frame.

Les matériaux de post-process sont enregistrés une fois par nom. Chaque occurrence
active transmet ses paramètres et son poids, sans structure globale énumérant les
effets. L'ordre d'enregistrement des passes détermine la composition ; les
occurrences d'un même matériau conservent leur ordre de soumission. Le graphe
assure la composition ; la présentation passe par `GpuContext`.

## Ajouter un effet d'écran

Le shader et son uniforme appartiennent à l'effet. Enregistrer le nom utilisé
dans le champ `material` de sa représentation JSON :

```rust
#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct VignetteUniform { strength: f32 }

renderer.register_vfx_post_effect(
    &gpu_ctx,
    &mut assets,
    "vignette",
    &std::fs::read_to_string("assets/shaders/vignette.frag.wgsl")?,
    |input| VignetteUniform {
        strength: input.scalar("strength").unwrap_or(1.0) * input.weight,
    },
)?;
```

Le fragment utilise la texture de scène en groupe 0/binding 0, son sampler en
groupe 0/binding 1 et l'uniforme propre à l'effet en groupe 1/binding 0. Le type
`Pod` doit respecter le layout WGSL. `PostProcessInput` expose les paramètres
scalaires/vectoriels, le poids, le temps restant, les dimensions et `aspect_ratio()`.

Le JSON de l'effet désigne `"material": "vignette"` et définit par exemple
`"parameters": { "strength": 0.8 }`. Son activation reste un appel ordinaire à
`VfxApi::spawn()` ou `spawn_detached()`. L'extraction et l'envoi GPU sont automatiques.
Aucun champ ni branche de code n'est à ajouter au runtime ou au renderer pour
un nouveau matériau. Le blind utilise seulement `post_process("blind")` pour
son rayon de visibilité et les informations de debug.

## Piloter directement une passe

L'ergonomie ajout / écriture / activation reste disponible, avec des uniforms
possédés côté CPU puis copiés dans les soumissions V2 :

```rust
let pass = renderer.add_post_process_pass(
    &gpu_ctx, &mut assets, fragment_source,
    Some(VignetteUniform { strength: 0.8 }),
)?;
renderer.write_post_process_uniform(pass, VignetteUniform { strength: 0.3 })?;
renderer.disable_post_process_pass(pass)?;
renderer.enable_post_process_pass(pass)?;
```

Les passes directes sont actives dès leur ajout. `None::<()>` crée une passe sans
uniforme. Les contrôles acceptent aussi les identifiants retournés par
`register_vfx_post_effect` : une écriture y remplace l'encodage automatique des
uniforms pour les occurrences soumises. Le renderer refuse les identifiants inconnus
et les écritures d'une taille différente du layout enregistré. Une frame déjà
préparée conserve ses propres octets. Les soumissions V2 explicites dans
`SceneFrame::post_process` restent disponibles et suivent les passes enregistrées.

## Texte et interface

Prism V2 ne fournit pas de soumission de texte. Le client utilise Glyphon directement,
après la géométrie du HUD, puis dessine Egui sur la même surface avant présentation.
Les textes sont façonnés avec les dimensions courantes et leur cache ne conserve
que les entrées du dernier ensemble de textes affiché.

Nodus utilise encore les types CPU de Prism V1. La dépendance `prism_legacy` reste
nécessaire pour ses couleurs, tokens UI et la collecte de ses commandes. Le module
`rendering/nodus_bridge.rs` convertit cette sortie vers les primitives client ;
aucun objet GPU V1 n'est créé. `register_nodus_texture` attribue un token UI à une
texture V2, et la barre de vie utilise un token vers un template V2. Les handles
des deux gestionnaires ne sont jamais convertis par leurs indices numériques.

Cette frontière pourra disparaître quand Nodus exposera une sortie V2 ou un
itérateur indépendant de Prism V1.

## Vérification

```sh
cargo check --workspace --all-targets --offline
cargo test --workspace --offline
cargo test -p client --test prism_v2_rendering --offline -- --ignored --nocapture
```

Les tests GPU lisent les pixels hors écran : caméra et shake, ordre stable des couches,
VFX additifs, shader de vie Nodus, texture et nine-patch, texte Glyphon, enchaînement
hit flash/blind, HUD après effets et frame vide après redimensionnement. Un autre
test ajoute un matériau arbitraire avec des paramètres JSON et vérifie son rendu,
ainsi que l'ajout, l'écriture et l'activation de passes directes. Ces tests exigent
un adaptateur graphique matériel ou logiciel et restent ignorés dans la suite normale.
Ils ne remplacent pas une vérification interactive de toutes les scènes ni une mesure
de performance.
