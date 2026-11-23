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
reprend encore dans `rendering/asset_cache.rs`, `material.rs` et
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
`AlphaRenderer::register_post_effect_with_bindings` ; l'ancienne méthode reste
un raccourci pour les emplacements des effets WGSL fournis avec Alpha.

`register_sets` regroupe les contributions par groupe et enregistre des
`BindingSet` natifs. Une collision de déclarations ou de ressources est une
erreur, jamais un remplacement implicite. Le cache des bindings de textures
inclut les emplacements de la texture **et** du sampler dans sa clé, pour éviter
de réutiliser un set incompatible entre deux programmes. Les propriétaires
des ressources partagées entre écrans restent dans le `RenderData` partagé.

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

La fenêtre utilise `Renderer::new`, le device et la queue du contexte Velum par défaut. Les contextes hors écran enregistrent la paire fournie par l’appelant. Au retour de suspension desktop, Alpha rattache la fenêtre au renderer existant via son instance WGPU et appelle son resize. Le raccordement est actif par défaut. La feature vide `velum` est conservée pour
la compatibilité des commandes précédentes.

Alpha ne dépend plus directement de Prism V2. La surface, le contexte, le renderer
et toutes les ressources GPU sont natifs Velum. Aucun `ResourceManager`,
`GraphRenderer` ni handle de ressource GPU Prism n'est créé. Nodus impose encore
des tokens et commandes CPU historiques ; leur conversion aboutit directement
aux ressources natives Velum. Cette frontière CPU est la seule utilisation de
`prism_legacy` dans Alpha.

## Uploads et vues

`rendering::texture::load_cached` réutilise la texture associée au chemin
tant que son propriétaire existe dans `RenderData`. Le chargeur de fichiers
de la révision Velum utilisée impose le format linéaire `Rgba8Unorm`.
Pour les sprites et les atlas, Alpha décode les images, crée la texture sRGB avec `TextureBuilder::build_in`
et transfère les pixels via `Queue::write_texture`. `texture::register_texture`
associe la texture au contexte Velum. `texture::get` résout ensuite son
identifiant pour lire ses dimensions ou obtenir sa vue.

Les données de sommets, indices, matrices et paramètres restent les mêmes. Les
transferts de buffers utilisent `try_write_buffer`, qui applique les offsets des
allocations Velum. Les `BufferRegion` transmettent les plages correspondantes aux
bindings et géométries ; aucun adaptateur d'identifiant Prism n'intervient.

Les meshes sont transférés par `geometry::try_mesh`. Les instances et les deux caméras utilisent les uploads du backend Velum.

## Rendu et durée de vie

Le menu, le lobby et le jeu sont trois `velum::Scene` enregistrées dans
`velum::Velum`. L’application active le SceneId de l’écran lors de la mise
en file de la frame. Chaque scène possède son SceneRenderer natif et son
RenderData pour les phases, les cibles, les buffers de géométrie, les
caméras et le texte. AlphaRenderer prépare les commandes de dessin.
En fenêtre, Alpha demande une Frame à Velum, appelle `Velum::render`, puis `Frame::finish` après le debug. Les sorties hors écran utilisent `OffscreenFrame::execute`.

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
nulle conserve les cibles existantes. Chaque scène conserve ses cibles intermédiaires et les adapte
aux dimensions de sortie. Velum configure et présente la surface de fenêtre.

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
partage et retrait des textures, cibles et vues par contexte. Le test de surface exerce la présentation native, le HUD, la minimisation, le redimensionnement et le rattachement de la fenêtre. Il a réussi sous Windows.

## Inventaire des programmes et états de pipeline

- Les formes et VFX géométriques utilisent le template standard de mesh Velum avec blending alpha ou additif.
- Les sprites, les tuiles et les VFX texturés ordinaires utilisent le shader standard de sprite de Velum. Les nine-patch utilisent la géométrie tessellée et le fragment texturé. Le flash utilise `entity_flash.frag.wgsl` pour remplacer la couleur en conservant le masque alpha.
- Barre de vie : `default_textured.vert.wgsl` et `progress_bar.frag.wgsl`, avec blending alpha et un uniforme de progression.
- Copie finale, flash de dégâts et aveuglement : `default_post_process.vert.wgsl` combiné respectivement à `default_post_process.frag.wgsl`, `hit_flash_effect.frag.wgsl` et `blind_effect.frag.wgsl`. Ces passes écrivent directement dans la cible, sans blending, et génèrent leurs sommets sans vertex buffer.
- La copie finale de la fenêtre est confiée à `PresentEffect` et `PresentOperation`. La copie sur un device hors écran utilise le matériau plein écran décrit ci-dessus.

Les matériaux alpha utilisent `ALPHA_BLENDING`. La variante additive combine
les couleurs avec les facteurs `SrcAlpha` et `One`, et compose l’alpha avec
`OVER`. Les formes, les sprites et le HUD partagent le layout par sommet de
`velum::geometry::Vertex` : position et UV en `Float32x2`, couleur en
`Float32x4`, aux emplacements 0, 1 et 2.

Chaque passe applicative utilise un attachment couleur au format GPU du
contexte, sans profondeur ni stencil, avec un seul échantillon et sans
alpha-to-coverage. Le renderer valide le format et le nombre d’échantillons
de la texture de sortie avant l’encodage. Aucun pipeline compute applicatif
n’a été identifié.

`try_material_template!` enregistre les templates de scène et plein écran
dans Velum. `try_material!` enregistre les matériaux des formes avec leur
contexte GPU explicite et une liste vide de bindings propres au matériau.
La macro Velum accepte une liste d’expressions entre crochets. Les sprites,
le HUD et les effets plein écran assemblent leurs bindings dans un vecteur
à l’exécution et le transmettent à `material::try_register_in`, l’API
d’enregistrement utilisée par la macro avec contexte explicite. Les sprites
réutilisent un matériau par texture, blending et variante flash.
`RenderData` conserve les propriétaires des ressources du registre Velum.
Les `RenderObject` reçoivent les `MaterialId` natifs à la soumission.

## Instances, passes et diagnostics

`about_to_wait` met à jour le jeu, les animations et l’interface, puis demande un rafraîchissement à winit. À la réception de `RedrawRequested`, Alpha active la scène Velum de l’écran courant et collecte ses objets à partir du dernier état disponible. Les tuiles visibles et les libellés utilisent les dimensions courantes. Le renderer soumet ces données à Velum dans le même événement.

La carte transmet les cellules visibles comme sprites. Alpha renseigne les transformations, couleurs, UV et matériaux. Velum fournit la représentation RenderInstance, le template standard, les uploads, le batching adjacent et le dessin. Les formes dynamiques utilisent son tessellateur et sa géométrie indexée. Les programmes de jeu restent décrits par leurs schémas de bindings.

Les caméras monde et écran sont des standard::Cameras mises à jour avec Camera2D. Pour un shader personnalisé, Alpha fournit les emplacements déclarés par ce shader.

En fenêtre, le texte passe par backend::text (registre, préparation et dessin). PresentEffect échantillonne la dernière cible et écrit dans la surface. Egui encode son panneau de debug sur la Frame Velum avant la présentation. Le panneau lit les compteurs de scène avant cet overlay.

### Limites de la révision publiée 30c0ea3

- Le chargeur de fichiers impose Rgba8Unorm. Le chargement des images sRGB utilise TextureBuilder et l’écriture de pixels dans la queue correspondante.
- Le texte natif et PresentEffect utilisent le device global. Un device isolé fourni pour le rendu hors écran utilise Glyphon et un matériau de copie associés à ce device.
- Egui fournit son propre renderer. Alpha emploie RenderPassBuilder pour lui ouvrir une passe sur la Frame Velum.
- Les commandes Nodus utilisent des types CPU Prism. Le pont les convertit en descriptions pour Velum.
- L’application conserve la sélection des effets de jeu, leurs paramètres, les UV d’atlas et l’ordre des calques. Les effets standard bloom, flou et HDR ne correspondent pas aux effets d’aveuglement et de flash d’Alpha.
