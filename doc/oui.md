Phase 1 — Préparation et infrastructure
1. Préparer la branche de migration
- [x] Créer une branche refactor/velum-renderer dans Alpha.
- [x] Ajouter Velum depuis learn_wgpu, branche codex/velum-prism-alpha.
- [x] Conserver Prism comme dépendance temporaire.
- [x] Identifier les modules d'Alpha qui utilisent directement Prism.
- [x] Recenser les types, traits et macros Prism utilisés.
- [x] Identifier les appels de rendu effectués à chaque frame.
- [x] Identifier les ressources graphiques créées au démarrage et pendant l'exécution.
- [x] Préparer un mécanisme de sélection du backend, si les deux peuvent coexister sans trop de complexité.
- [x] Vérifier que le projet compile sans modifier son comportement.
Validation : Alpha démarre encore avec Prism, mais Velum est disponible.
2. Migrer le contexte GPU
- [x] Identifier qui possède actuellement wgpu::Device et wgpu::Queue.
- [x] Raccorder Velum au contexte GPU utilisé par Alpha.
- [x] Éviter toute duplication inutile du device ou de la queue.
- [x] Vérifier la gestion de la surface de rendu.
- [x] Conserver les formats de textures et les paramètres de présentation compatibles.
- [x] Gérer correctement le redimensionnement de la fenêtre.
- [x] Vérifier la destruction et la recréation des ressources dépendantes de la surface.
- [x] Vérifier la propagation des erreurs GPU.
Validation : Velum peut accéder au GPU et préparer une frame dans Alpha.
3. Migrer le stockage des ressources
- [x] Identifier les collections de ressources utilisées par Prism.
- [x] Migrer le stockage des buffers vers les mécanismes Velum retenus.
- [x] Migrer les textures et leurs identifiants.
- [x] Migrer les samplers.
- [x] Migrer les render targets.
- [x] Raccorder les identifiants de ressources aux structures applicatives d'Alpha.
- [x] Vérifier le partage des ressources entre plusieurs objets.
- [x] Vérifier la destruction des ressources après libération du dernier propriétaire (tests GPU de stockage).
- [x] Conserver les systèmes d'upload et de vues existants, sans refonte.
- [x] Ne pas ajouter de nouveau système de cache.
Validation : les ressources nécessaires à un rendu peuvent être créées, retrouvées, utilisées et libérées correctement.

Phase 2 — Construction du rendu
4. Migrer les bindings
C'est une étape fondamentale, mais la nouvelle branche de Velum possède déjà une API de bindings explicites.
- [x] Recenser les conventions de groupes et de bindings utilisées par Prism. Voir [l'inventaire](migration-velum.md#inventaire-des-conventions-de-bindings-prism).
- [x] Supprimer leur dépendance dans les nouveaux composants d'Alpha.
- [x] Définir les BindingSchema des shaders.
- [x] Définir les BindingSet nécessaires.
- [x] Raccorder les uniform buffers.
- [x] Raccorder les storage buffers. Contributions natives par objet, validées sur GPU ; aucun buffer métier supplémentaire introduit.
- [x] Raccorder les textures et samplers.
- [x] Raccorder les tableaux de ressources si Alpha en utilise. Aucun tableau utilisé par les shaders applicatifs actuels ; contributions natives conservées.
- [x] Gérer les dynamic offsets lorsque nécessaire.
- [x] Vérifier les collisions entre plusieurs contributions de bindings.
- [x] Vérifier que les ressources déclarées correspondent aux shaders WGSL.
- [x] Tester plusieurs groupes non consécutifs. Les groupes 1 et 3 sont validés avec Naga et dans le rendu de post-traitement sur GPU.
- [x] Tester des bindings à des indices arbitraires. Le rendu GPU utilise notamment les indices 5, 7 et 9.
- [x] Vérifier que les macros permettent de construire les schémas et les ensembles de bindings. Le test unitaire compare le résultat de `binding_schema!` au schéma Alpha et le test GPU utilise un ensemble créé par `try_binding_set!`.
Validation : Alpha peut utiliser des groupes et bindings arbitraires sans convention imposée par Velum.
5. Migrer les shaders, matériaux et pipelines
- [x] Inventorier les shaders utilisés dans Alpha.
- [x] Identifier les variantes de shaders.
- [x] Migrer les shaders vers les nouveaux matériaux Velum.
- [x] Construire les MaterialTemplate.
- [x] Raccorder les MaterialId aux objets applicatifs.
- [x] Migrer les états de blending.
- [x] Migrer les états de profondeur et de stencil.
- [x] Migrer les formats des attachments.
- [x] Migrer les layouts de vertex buffers.
- [x] Vérifier les paramètres de multisampling.
- [x] Migrer les éventuels pipelines compute.
- [x] Vérifier que les matériaux restent constructibles par macros.
- [x] Vérifier la compatibilité des shaders avec les nouveaux schémas de bindings.
Validation : un objet simple peut être rendu dans Alpha avec un matériau et un pipeline Velum.
6. Migrer la géométrie et le dessin
- [x] Identifier les primitives graphiques utilisées par Alpha.
- [x] Migrer les vertex buffers.
- [x] Migrer les index buffers.
- [x] Migrer les formats de sommets.
- [x] Migrer les meshes.
- [x] Raccorder les géométries générées dynamiquement.
- [x] Migrer les formes 2D.
- [x] Migrer les rectangles, triangles et autres primitives nécessaires.
- [x] Migrer les formes à neuf zones (nine-patch) si utilisées.
- [x] Vérifier les UV et les couleurs.
- [x] Vérifier les coordonnées locales.
- [x] Vérifier le dessin indexé et non indexé.
- [x] Vérifier la génération de géométrie à partir des structures d'Alpha.
Validation : les principales primitives d'Alpha sont dessinées correctement par Velum.

Phase 3 — Instances et données applicatives
7. Migrer les instances GPU
Cette étape doit préserver un point important de ton architecture : Alpha fournit ses propres structures de données, qui implémentent les traits Velum.
- [x] Identifier les structures d'instances actuelles d'Alpha.
- [x] Conserver les structures métier d'Alpha autant que possible.
- [x] Implémenter les traits Velum nécessaires, notamment InstanceSource et ObjectInstance.
- [x] Définir les représentations GPU des instances.
- [ ] Vérifier les conversions entre données CPU et GPU.
- [x] Raccorder les géométries aux instances.
- [x] Raccorder les matériaux aux instances.
- [x] Configurer librement les groupes et bindings des instances en storage buffer.
- [x] Configurer librement les slots des instances en vertex buffer.
- [ ] Vérifier les layouts et les strides.
- [x] Migrer le batching des instances.
- [x] Vérifier les draw calls instanciés. Le test GPU dessine deux tuiles avec un seul appel.
- [ ] Vérifier les mises à jour des données d'instances.
- [ ] Tester plusieurs types d'instances avec des bindings différents.
- [ ] Vérifier que les macros restent utilisables là où elles étaient prévues.
Validation : Alpha peut soumettre ses propres objets à Velum sans dépendre de structures métier imposées par le moteur de rendu.

Phase 4 — Caméras et orchestration
8. Migrer les caméras
Deux modes doivent être disponibles : caméra monde et caméra écran.
- [x] Identifier les caméras actuellement utilisées par Alpha.
- [x] Définir les données nécessaires à une caméra monde.
- [x] Définir les données nécessaires à une caméra écran.
- [x] Raccorder les matrices de vue.
- [x] Raccorder les matrices de projection.
- [ ] Vérifier la projection orthographique.
- [x] Prévoir la projection perspective si Alpha en a besoin. Les scènes actuelles utilisent une projection 2D orthographique.
- [x] Raccorder les dimensions de la fenêtre.
- [x] Gérer les changements de résolution.
- [x] Raccorder les données des caméras aux bindings Velum.
- [x] Ne pas réserver de groupe ou binding obligatoire aux caméras.
- [ ] Vérifier que plusieurs matériaux peuvent utiliser la même caméra.
- [ ] Vérifier les coordonnées écran et monde.
- [ ] Tester les deux modes de caméra dans une même frame.
Validation : les objets monde et les éléments écran sont affichés dans les bons espaces de coordonnées.
9. Migrer la scène et l'orchestration
- [x] Identifier comment Alpha organise actuellement ses objets à rendre.
- [x] Raccorder les objets applicatifs aux RenderObject Velum.
- [x] Migrer les phases de rendu.
- [x] Définir les règles de tri.
- [x] Raccorder les objets opaques.
- [x] Raccorder les objets transparents.
- [x] Raccorder les éléments d'interface.
- [x] Préserver l'ordre de dessin attendu.
- [x] Migrer les soumissions d'objets à chaque frame.
- [x] Déclencher la collecte et le rendu de la scène sur `RedrawRequested` et retirer la file de frames d’`about_to_wait`.
- [x] Utiliser les mécanismes de scène et d'orchestration Velum. Menu, lobby et jeu ont chacun leur Scene, leur SceneRenderer et leur RenderData natifs.
- [x] Éviter de reconstruire une deuxième orchestration dans Alpha.
- [ ] Vérifier les ajouts et suppressions d'objets en cours d'exécution.
- [ ] Vérifier la gestion des objets qui changent de matériau ou de géométrie.
Validation : une frame complète d'Alpha peut être décrite et exécutée par l'orchestration Velum.
10. Migrer les passes et les effets
La gestion des passes doit rester celle de Velum.
- [x] Recenser les passes actuellement utilisées dans Alpha.
- [x] Identifier les passes de rendu principales.
- [x] Identifier les éventuelles passes offscreen.
- [x] Identifier les effets de post-traitement.
- [x] Raccorder les render targets.
- [x] Raccorder les opérations de clear et load.
- [x] Migrer les dépendances entre passes.
- [x] Vérifier les formats et les dimensions des attachments.
- [x] Raccorder la présentation finale à la surface.
- [ ] Vérifier les effets de composition.
- [ ] Vérifier les effets de flou ou autres effets utilisés.
- [x] Vérifier que les passes fonctionnent avec les matériaux et bindings explicites.
- [x] Conserver les fonctionnalités historiques de gestion des passes Velum.
Validation : Velum produit la frame complète d'Alpha, effets compris.

Phase 5 — Finalisation
11. Migrer le rapport de frame et les diagnostics
- [x] Raccorder le début et la fin de frame au reporting Velum.
- [x] Compter les draw calls.
- [x] Compter les instances.
- [x] Compter les passes.
- [x] Mesurer les octets transférés lorsque les informations sont disponibles.
- [x] Raccorder les diagnostics d'erreur.
- [ ] Vérifier le comportement d'une frame abandonnée ou échouée.
- [x] Adapter les outils de diagnostic d'Alpha.
- [ ] Comparer les métriques Prism et Velum sur des scènes équivalentes.
Validation : Alpha peut diagnostiquer une frame Velum et suivre ses principales métriques.
12. Vérifier la compatibilité et retirer Prism
- [ ] Vérifier les principaux écrans et scènes d'Alpha.
- [ ] Comparer visuellement les rendus Prism et Velum.
- [ ] Vérifier les textures, couleurs, transparences et profondeurs.
- [ ] Vérifier les changements de résolution.
- [ ] Vérifier la création et destruction dynamique des objets.
- [ ] Vérifier les performances sur des scènes représentatives.
- [ ] Vérifier l'absence de fuites de ressources.
- [ ] Exécuter les tests de Velum.
- [ ] Vérifier que l'ancienne API Velum fonctionne toujours.
- [ ] Vérifier que les nouvelles fonctionnalités restent constructibles par macros lorsque prévu.
- [ ] Retirer les appels à Prism dans Alpha.
- [ ] Supprimer la dépendance Prism. Le DrawCommandBuffer de la révision Nodus utilisée exporte ses commandes via une Frame Prism.
- [ ] Supprimer les adaptateurs temporaires devenus inutiles.
- [x] Nettoyer les anciens shaders et types inutilisés.
- [x] Documenter la nouvelle architecture dans [migration-velum.md](migration-velum.md).
- [ ] Fusionner la branche de migration après validation.
Validation : Alpha fonctionne entièrement avec Velum, sans dépendance à Prism.
