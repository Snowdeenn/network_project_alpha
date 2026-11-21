# Migration des ressources Alpha vers Velum

Sur `refactor/velum-renderer`, les ressources GPU et le dessin utilisent les types
et les fonctions natifs de Velum. Le fichier `rendering/resources.rs`, le
`ResourceStore`, les types de ressources propres à Alpha et les projections vers
le gestionnaire Prism ont été supprimés.

## API native utilisée

| Ressource | API Velum | Propriétaire |
| --- | --- | --- |
| Sommets et indices | `buffer::allocate_in::<u8, Vertex/Index>`, `BufferRegion::from_shared` | `SharedResource<Buffer<…>>` dans les flux du renderer |
| Caméras et uniforms | `buffer::allocate_in::<u8, Uniform>`, `try_write_buffer` | Propriétaires natifs et leases des bindings |
| Textures | `texture::Texture`, `try_register_in`, `TextureId`, `get` | Propriétaires natifs partagés, conservés par les assets |
| Samplers | `sampler::Sampler<Filtering>`, `pixel_art_in` | Handle natif immuable |
| Cibles | `target::RenderTargetDescriptor`, `try_register_in`/`register_in`, `get` | `OwnedResource<RenderTargetId>` |
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

Alpha référence le checkout local `../../learn_wgpu`, sur `main`, dont la
bibliothèque s'appelle `velum`. Ce checkout accompagne la migration : il expose
les API natives nécessaires à un contexte externe pour les textures, samplers
et cibles. Il faut conserver les modifications des deux dépôts pour reproduire
le build.

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

Le décodage des images, les formats sRGB et `Queue::write_texture` sont conservés.
Le résultat est un `texture::Texture` natif, avec sa vue native, enregistré dans
le contexte Velum approprié.

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
`target::resize_window_targets_in(context, size)`. Les identifiants restent
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
