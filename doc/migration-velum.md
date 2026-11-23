# Migration des ressources Alpha vers Velum

## Inventaire des conventions de bindings Prism

Recensement effectué dans le code Alpha avant migration Velum (commit `9519fb6`,
`client/src/rendering/assets.rs` et `renderer.rs`), les shaders WGSL actuels et
le checkout de Prism référencé par le lockfile (`9d1f625`). Les numéros ci-dessous
sont des couples **groupe/binding** ; ils ne désignent pas les slots de vertex
buffers ni les attributs `@location`.

### Monde, VFX géométriques et HUD

- **g0/b0 : caméra**, uniform `mat4x4<f32>` de 64 octets, visibilité vertex.
  Les objets monde/VFX utilisent la matrice monde ; le HUD utilise la matrice
  écran au même emplacement, dans ses propres dessins. Déclaré dans
  `client/src/graphic_data/shader/default.vert.wgsl` et
  `default_textured.vert.wgsl`.
- **g1/b0 : texture de sprite**, `texture_2d<f32>` filtrable et non multisamplée,
  visibilité fragment.
- **g1/b1 : sampler de sprite**, sampler de type filtering, visibilité fragment.
  La paire est utilisée par `default_textured.frag.wgsl` et
  `entity_flash.frag.wgsl`, ainsi que pour les nine-patch. Le choix nearest/pixel
  art est un réglage du sampler, pas un autre emplacement de binding.
- **g2/b0 : paramètres propres au matériau HUD**, uniform. Le shader
  `progress_bar.frag.wgsl` lit un `f32` de 4 octets (`u_ratio`), en fragment.
  Son template Alpha contient les groupes **0 et 2**, sans texture au groupe 1.
  Les formes colorées n'ont besoin que du groupe 0 ; leurs couleurs passent
  par les sommets.

Sources Prism V1 : `crates/prism-v1/src/pass/mod.rs` définit
`CAM_BIND_GROUP`, `TEXTURE_BIND_GROUP` et `MATERIAL_BIND_GROUP` ;
`pass/world.rs`, `pass/vfx.rs` et `pass/hud.rs` appliquent ces emplacements
avec `set_bind_group`. Ce sont des conventions imposées par ces passes V1.

### Post-traitements et composition plein écran

- **g0/b0 : texture source de la passe**, `texture_2d<f32>` filtrable,
  visibilité fragment.
- **g0/b1 : sampler de la source**, sampler filtering, visibilité fragment.
- **g1/b0 : paramètres de l'effet**, uniform optionnel. Alpha utilise 4 octets
  pour `hit_flash_effect.frag.wgsl` et 8 octets pour
  `blind_effect.frag.wgsl`, en fragment. Le blit de
  `default_post_process.frag.wgsl` ne déclare aucun paramètre.

Il s'agit d'un layout distinct : **g0/b0 désigne ici une texture, pas une
caméra**. Il ne faut donc pas fusionner aveuglément les contributions des
passes géométriques et des passes plein écran.

Dans Prism V2, `render/passes.rs::FullscreenEffect::source_schema` propose
g0/b0 et g0/b1 par défaut, mais `source_binding`, `sampler_binding` et
`parameter_binding` sont explicites. Alpha les configurait respectivement à
`(0, 0)`, `(0, 1)` et, si nécessaire, `(1, 0)`.

### Offsets dynamiques et autres ressources

Le HUD Prism V1 utilise un scratch uniform en **g2/b0**, visible en
vertex/fragment, avec `has_dynamic_offset: true`. Chaque dessin de matériau
fournit son offset à `set_bind_group(2, ..., &[offset])`. Le binding couvre
256 octets ; les blocs acceptés ont une taille multiple de 4, au plus 256,
et leurs emplacements respectent `min_uniform_buffer_offset_alignment` du
device (`pass/hud.rs`). Les paramètres des post-traitements V1 en g1/b0
n'utilisent pas d'offset dynamique.

Le backend Alpha sous Prism V2 utilisait déjà des uniforms **sans offset
dynamique**, via `uniform_group` et les `ParameterBlock` en g2/b0. Les
helpers Velum actuels conservent ce choix (`has_dynamic_offset: false`).
La convention dynamique V1 est donc historique, pas un mécanisme actif à
reproduire automatiquement dans Alpha.

Aucun storage buffer, storage texture, tableau de bindings (`count: Some`),
binding d'instances ou pipeline compute n'a été relevé dans les schémas et
shaders de rendu Alpha examinés. Les nombreux sprites partagent des bindings
de textures individuels ; ils ne constituent pas un tableau de bindings.
Glyphon et Egui gèrent leurs propres pipelines et layouts, hors de ces
conventions applicatives.

### Portée pour la migration

Prism V2 possède déjà des `BindingSchema` avec groupes et bindings explicites ;
son `bindings/mod.rs::LayoutCache` conserve les trous entre groupes. Les slots
0/1/2 ci-dessus sont donc des choix des passes historiques ou d'Alpha, pas une
contrainte générale de Prism V2 ni de Velum. Le code Velum actuel d'Alpha les
reprend encore dans `rendering/asset_cache.rs`, `resource_loader.rs` et
`renderer.rs`.

Le recensement historique est distinct du raccordement explicite décrit ci-dessous.

## Bindings explicites dans Alpha

`client/src/rendering/shader_bindings.rs` définit les contrats des programmes :
`BindingLocation` représente un couple groupe/binding, `MeshBindings` les
ressources d'un dessin géométrique et `EffectBindings` celles d'un effet plein
écran. Les schémas sont des `BindingSchema` natifs Velum. Les constantes des
shaders fournis décrivent leurs WGSL existants ; elles ne réservent aucun slot
dans le renderer. Les groupes peuvent être partagés entre plusieurs rôles,
séparés par des trous, et les bindings peuvent avoir des indices arbitraires.

Le renderer utilise le contrat du matériau pour raccorder sa caméra, sa texture
et ses paramètres. `Primitive::Material` transporte ce contrat et des
`ObjectBindings` supplémentaires. Le pont Nodus conserve le contrat avec le
template enregistré. Les effets personnalisés passent par
`SceneRenderer::register_post_effect_with_bindings` ; l'ancienne méthode reste
un raccourci pour les emplacements des effets WGSL fournis avec Alpha.

`register_sets` regroupe les contributions par groupe et enregistre des
`BindingSet` natifs. Une collision de déclarations ou de ressources est une
erreur, jamais un remplacement implicite. Le cache des bindings de textures
inclut les emplacements de la texture **et** du sampler dans sa clé, pour éviter
de réutiliser un set incompatible entre deux programmes. Les propriétaires
restent dans le `RenderData` de l'app, sans ajout de `Scene`.

Les uniforms automatiques vérifient leur taille avant l'upload. Les contributions
supplémentaires acceptent les `BindingResource` et `BindingUse` natifs : buffers
uniform/storage, textures, samplers et tableaux. Les offsets dynamiques sont
ceux de `BindingUse::with_offsets`, appliqués par Velum après validation de
l'alignement, du nombre d'offsets et des plages de buffers. Les contributions
ordinaires de matériaux et d'objets restent soumises aux contrôles de collision
Velum ; un remplacement doit être explicitement demandé dans l'API native.

Aucun shader applicatif actuel n'exige de storage buffer ou de tableau de
ressources. Le chemin storage est néanmoins testé sur GPU à travers le renderer
Alpha ; aucun nouveau buffer métier ni besoin de feature de tableaux n'est
introduit. Les paramètres automatiques actuels utilisent des uniforms statiques.

Validation ciblée :

```sh
cargo test -p client --lib --test shader_bindings --test asset_cache --offline --locked -j 1 -- --include-ignored --test-threads=1
```

Les tests CPU analysent tous les shaders applicatifs avec Naga et comparent
emplacements, types, tailles de buffers, visibilité et exhaustivité des schémas.
Le test GPU `shader_bindings` vérifie les pixels avec caméra en g2/b5, texture
en g2/b7, sampler en g3/b9, paramètres en g0/b4, storage en g1/b6 et uniform
dynamique en g1/b8. Il vérifie aussi un effet en groupes 1 et 3, les collisions,
les offsets manquants/surnuméraires/désalignés/hors plage et le rejet d'un
layout incompatible avec le WGSL. Les anciennes suites `resource_storage` et
`prism_v2_rendering` restent séparément bloquées par des API absentes de la
version Velum épinglée, comme relevé avant ce travail.

Sur `refactor/velum-renderer`, les ressources GPU et le dessin utilisent les types
et les fonctions natifs de Velum. Le fichier `rendering/resources.rs`, le
`ResourceStore`, les types de ressources propres à Alpha et les projections vers
le gestionnaire Prism ont été supprimés.

## API native utilisée

| Ressource | API Velum | Propriétaire |
| --- | --- | --- |
| Sommets et indices | `buffer::allocate_in::<u8, Vertex/Index>`, `BufferRegion::from_shared` | `SharedResource<Buffer<…>>` dans les flux du renderer |
| Caméras et uniforms | `buffer::allocate_in::<u8, Uniform>`, `try_write_buffer` | Propriétaires natifs et leases des bindings |
| Textures | `texture::TextureBuilder`, `texture::register_texture`, `TextureId`, `texture::get` | `SharedResource<TextureId>` dans `RenderData` |
| Samplers | `sampler::Sampler<Filtering>`, `sampler::pixel_art` | Handle natif immuable |
| Cibles | `target::RenderTargetDescriptor`, `target::register`, `target::get` | `OwnedResource<RenderTargetId>` |
| Shaders | `backend::try_shader` | Modules natifs détenus par les templates |
| Templates et matériaux | `material::try_template`, `try_register` | `SharedResource<MaterialTemplateId/MaterialId>` |
| Bindings | `bindings::try_register`, `BindingSet`, `BindingResource` | Propriétaires natifs partagés |
| Géométries | `geometry::try_register`, `Geometry`, `VertexBuffer`, `IndexBuffer` | Propriétaires natifs de la frame |
| Objets et phases | `RenderObject`, `RenderPhase` | Descriptions et snapshots préparés par Velum |

Les animations, la carte et les primitives référencent directement
`velum::backend::texture::TextureId`. Les primitives de matériau détiennent un
`SharedResource<MaterialTemplateId>` natif. Les valeurs ne sont pas réenregistrées
sous des types de ressources Alpha dans un registre générique.

Les tables préexistantes des assets conservent les propriétaires des textures par
chemin, bindings et matériaux de sprites. Aucun nouveau cache ou gestionnaire de
ressources n'est ajouté.

## Contexte et dépendance

Alpha utilise la bibliothèque `velum` du dépôt Git `learn_wgpu`. La révision
consignée dans `Cargo.lock` fournit les API de textures, de samplers et de
cibles qui acceptent le contexte GPU d’Alpha.

Le contexte de fenêtre est créé par `Renderer::new_registered`, qui renvoie une
surface et un propriétaire de contexte natifs Velum. Les contextes hors écran
enregistrent directement une paire device/queue fournie par l’appelant.
Tous les appels reçoivent son `ContextId` explicite ; le contexte global par
défaut n'est pas initialisé. Aucun second device ni seconde queue n'est créé.
Le raccordement est actif par défaut. La feature vide `velum` est conservée pour
la compatibilité des commandes précédentes.

Alpha ne dépend plus directement de Prism V2. La surface, le contexte, le renderer
et toutes les ressources GPU sont natifs Velum. Aucun `ResourceManager`,
`GraphRenderer` ni handle de ressource GPU Prism n'est créé. Nodus impose encore
des tokens et commandes CPU historiques ; leur conversion aboutit directement
aux ressources natives Velum. Cette frontière CPU est la seule utilisation de
`prism_legacy` dans Alpha.

## Uploads et vues

Alpha décode les images, crée la texture sRGB avec `TextureBuilder::build_in`
et transfère les pixels via `Queue::write_texture`. `texture::register_texture`
associe la texture au contexte Velum. `texture::get` résout ensuite son
identifiant pour lire ses dimensions ou obtenir sa vue.

Les données de sommets, indices, matrices et paramètres restent les mêmes. Les
transferts de buffers utilisent `try_write_buffer`, qui applique les offsets des
allocations Velum. Les `BufferRegion` transmettent les plages correspondantes aux
bindings et géométries ; aucun adaptateur d'identifiant Prism n'intervient.

Les flux de géométrie conservent leur capacité et grandissent au besoin. Les
allocations et les limites de pages relèvent de l'arène native Velum.

## Rendu et durée de vie

`SceneRenderer` conserve les descriptions CPU d'Alpha, trie les couches de manière
stable, tesselle avec `velum::geometry::Tesselator` et regroupe les dessins
consécutifs compatibles. Il soumet ensuite des `RenderObject` et `RenderPhase`
natifs.

L'ordre reste monde, VFX, post-traitements, géométrie HUD, texte Glyphon, puis
Egui. Les programmes de post-traitement utilisent des templates, uniforms et
cibles Velum ; leurs identifiants de commande sont de simples indices de passes
applicatives, pas des identifiants de ressources GPU.

`OffscreenFrame` possède l'encodeur du rendu. Il soumet les commandes puis libère
les références temporaires de Velum. Une erreur abandonne l'encodeur avant le
nettoyage. Les propriétaires et leases natifs empêchent la réutilisation d'une
allocation pendant son encodage. Les commandes soumises conservent leurs handles
WGPU.

Copier un identifiant ne possède pas la ressource. Partager un propriétaire natif
la maintient vivante ; la destruction du dernier propriétaire retire l'entrée de
son registre. Les snapshots de cibles et les handles GPU déjà résolus peuvent
rester vivants après ce retrait.

Les cibles dépendantes de la fenêtre peuvent être redimensionnées par
`target::resize_window_targets(context, size)`. Les identifiants restent
stables, les révisions changent et les vues sont résolues à nouveau. Une taille
nulle conserve les cibles existantes. Le renderer crée ses cibles transitoires
aux dimensions de la texture de sortie ; la surface reste gérée par le contexte
Alpha et conserve ses formats et modes de présentation.

## Validation

```sh
cargo check --workspace --all-targets --all-features --offline --locked -j 1
cargo test -p client --lib --offline --locked -j 1
cargo test -p client --features velum --test gpu_context --test resource_storage --test gpu_surface --test prism_v2_rendering --offline --locked -j 1 -- --ignored --test-threads=1
```

Le nom historique `prism_v2_rendering` est conservé pour les tests de régression,
mais leurs ressources, cibles et soumissions sont désormais natives Velum. Ils
vérifient les pixels de la caméra, des couches, des VFX, des textures, du
nine-patch, du HUD, du texte et des post-traitements, ainsi que le redimensionnement.

Les tests de stockage utilisent directement l'API native : leases de buffers,
partage et retrait des textures, cibles et vues par contexte. Le test X11 vérifie
la présentation et la recréation de la surface. Avant le dernier remplacement du contexte de présentation, les huit tests GPU
(rendu, ressources, surface et erreurs de validation) et les 77 tests unitaires
du client ont réussi. La dernière exécution GPU demandée après ce remplacement
n’a pas été autorisée ; la validation sur GPU de ce dernier changement reste à
faire. La compilation du workspace avec toutes les cibles et features réussit.
