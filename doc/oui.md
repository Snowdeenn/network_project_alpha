Phase 1 — Préparation et infrastructure
1. Préparer la branche de migration
- [ ] Créer une branche refactor/velum-renderer dans Alpha.
- [ ] Ajouter Velum depuis learn_wgpu, branche codex/velum-prism-alpha.
- [ ] Conserver Prism comme dépendance temporaire.
- [ ] Identifier les modules d'Alpha qui utilisent directement Prism.
- [ ] Recenser les types, traits et macros Prism utilisés.
- [ ] Identifier les appels de rendu effectués à chaque frame.
- [ ] Identifier les ressources graphiques créées au démarrage et pendant l'exécution.
- [ ] Préparer un mécanisme de sélection du backend, si les deux peuvent coexister sans trop de complexité.
- [ ] Vérifier que le projet compile sans modifier son comportement.
Validation : Alpha démarre encore avec Prism, mais Velum est disponible.
2. Migrer le contexte GPU
- [ ] Identifier qui possède actuellement wgpu::Device et wgpu::Queue.
- [ ] Raccorder Velum au contexte GPU utilisé par Alpha.
- [ ] Éviter toute duplication inutile du device ou de la queue.
- [ ] Vérifier la gestion de la surface de rendu.
- [ ] Conserver les formats de textures et les paramètres de présentation compatibles.
- [ ] Gérer correctement le redimensionnement de la fenêtre.
- [ ] Vérifier la destruction et la recréation des ressources dépendantes de la surface.
- [ ] Vérifier la propagation des erreurs GPU.
Validation : Velum peut accéder au GPU et préparer une frame dans Alpha.
3. Migrer le stockage des ressources
- [ ] Identifier les collections de ressources utilisées par Prism.
- [ ] Migrer le stockage des buffers vers les mécanismes Velum retenus.
- [ ] Migrer les textures et leurs identifiants.
- [ ] Migrer les samplers.
- [ ] Migrer les render targets.
- [ ] Raccorder les identifiants de ressources aux structures applicatives d'Alpha.
- [ ] Vérifier le partage des ressources entre plusieurs objets.
- [ ] Vérifier la destruction des ressources lorsqu'elles ne sont plus utilisées.
- [ ] Conserver les systèmes d'upload et de vues existants, sans refonte.
- [ ] Ne pas ajouter de nouveau système de cache.
Validation : les ressources nécessaires à un rendu peuvent être créées, retrouvées, utilisées et libérées correctement.

Phase 2 — Construction du rendu
4. Migrer les bindings
C'est une étape fondamentale, mais la nouvelle branche de Velum possède déjà une API de bindings explicites.
- [ ] Recenser les conventions de groupes et de bindings utilisées par Prism.
- [ ] Supprimer leur dépendance dans les nouveaux composants d'Alpha.
- [ ] Définir les BindingSchema des shaders.
- [ ] Définir les BindingSet nécessaires.
- [ ] Raccorder les uniform buffers.
- [ ] Raccorder les storage buffers.
- [ ] Raccorder les textures et samplers.
- [ ] Raccorder les tableaux de ressources si Alpha en utilise.
- [ ] Gérer les dynamic offsets lorsque nécessaire.
- [ ] Vérifier les collisions entre plusieurs contributions de bindings.
- [ ] Vérifier que les ressources déclarées correspondent aux shaders WGSL.
- [ ] Tester plusieurs groupes non consécutifs.
- [ ] Tester des bindings à des indices arbitraires.
- [ ] Vérifier que les macros permettent de construire les schémas et les ensembles de bindings.
Validation : Alpha peut utiliser des groupes et bindings arbitraires sans convention imposée par Velum.
5. Migrer les shaders, matériaux et pipelines
- [ ] Inventorier les shaders utilisés dans Alpha.
- [ ] Identifier les variantes de shaders.
- [ ] Migrer les shaders vers les nouveaux matériaux Velum.
- [ ] Construire les MaterialTemplate.
- [ ] Raccorder les MaterialId aux objets applicatifs.
- [ ] Migrer les états de blending.
- [ ] Migrer les états de profondeur et de stencil.
- [ ] Migrer les formats des attachments.
- [ ] Migrer les layouts de vertex buffers.
- [ ] Vérifier les paramètres de multisampling.
- [ ] Migrer les éventuels pipelines compute.
- [ ] Vérifier que les matériaux restent constructibles par macros.
- [ ] Vérifier la compatibilité des shaders avec les nouveaux schémas de bindings.
Validation : un objet simple peut être rendu dans Alpha avec un matériau et un pipeline Velum.
6. Migrer la géométrie et le dessin
- [ ] Identifier les primitives graphiques utilisées par Alpha.
- [ ] Migrer les vertex buffers.
- [ ] Migrer les index buffers.
- [ ] Migrer les formats de sommets.
- [ ] Migrer les meshes.
- [ ] Raccorder les géométries générées dynamiquement.
- [ ] Migrer les formes 2D.
- [ ] Migrer les rectangles, triangles et autres primitives nécessaires.
- [ ] Migrer les formes à neuf zones (nine-patch) si utilisées.
- [ ] Vérifier les UV et les couleurs.
- [ ] Vérifier les coordonnées locales.
- [ ] Vérifier le dessin indexé et non indexé.
- [ ] Vérifier la génération de géométrie à partir des structures d'Alpha.
Validation : les principales primitives d'Alpha sont dessinées correctement par Velum.

Phase 3 — Instances et données applicatives
7. Migrer les instances GPU
Cette étape doit préserver un point important de ton architecture : Alpha fournit ses propres structures de données, qui implémentent les traits Velum.
- [ ] Identifier les structures d'instances actuelles d'Alpha.
- [ ] Conserver les structures métier d'Alpha autant que possible.
- [ ] Implémenter les traits Velum nécessaires, notamment InstanceSource et ObjectInstance.
- [ ] Définir les représentations GPU des instances.
- [ ] Vérifier les conversions entre données CPU et GPU.
- [ ] Raccorder les géométries aux instances.
- [ ] Raccorder les matériaux aux instances.
- [ ] Configurer librement les groupes et bindings des instances en storage buffer.
- [ ] Configurer librement les slots des instances en vertex buffer.
- [ ] Vérifier les layouts et les strides.
- [ ] Migrer le batching des instances.
- [ ] Vérifier les draw calls instanciés.
- [ ] Vérifier les mises à jour des données d'instances.
- [ ] Tester plusieurs types d'instances avec des bindings différents.
- [ ] Vérifier que les macros restent utilisables là où elles étaient prévues.
Validation : Alpha peut soumettre ses propres objets à Velum sans dépendre de structures métier imposées par le moteur de rendu.

Phase 4 — Caméras et orchestration
8. Migrer les caméras
Deux modes doivent être disponibles : caméra monde et caméra écran.
- [ ] Identifier les caméras actuellement utilisées par Alpha.
- [ ] Définir les données nécessaires à une caméra monde.
- [ ] Définir les données nécessaires à une caméra écran.
- [ ] Raccorder les matrices de vue.
- [ ] Raccorder les matrices de projection.
- [ ] Vérifier la projection orthographique.
- [ ] Prévoir la projection perspective si Alpha en a besoin.
- [ ] Raccorder les dimensions de la fenêtre.
- [ ] Gérer les changements de résolution.
- [ ] Raccorder les données des caméras aux bindings Velum.
- [ ] Ne pas réserver de groupe ou binding obligatoire aux caméras.
- [ ] Vérifier que plusieurs matériaux peuvent utiliser la même caméra.
- [ ] Vérifier les coordonnées écran et monde.
- [ ] Tester les deux modes de caméra dans une même frame.
Validation : les objets monde et les éléments écran sont affichés dans les bons espaces de coordonnées.
9. Migrer la scène et l'orchestration
- [ ] Identifier comment Alpha organise actuellement ses objets à rendre.
- [ ] Raccorder les objets applicatifs aux RenderObject Velum.
- [ ] Migrer les phases de rendu.
- [ ] Définir les règles de tri.
- [ ] Raccorder les objets opaques.
- [ ] Raccorder les objets transparents.
- [ ] Raccorder les éléments d'interface.
- [ ] Préserver l'ordre de dessin attendu.
- [ ] Migrer les soumissions d'objets à chaque frame.
- [ ] Utiliser les mécanismes de scène et d'orchestration Velum.
- [ ] Éviter de reconstruire une deuxième orchestration dans Alpha.
- [ ] Vérifier les ajouts et suppressions d'objets en cours d'exécution.
- [ ] Vérifier la gestion des objets qui changent de matériau ou de géométrie.
Validation : une frame complète d'Alpha peut être décrite et exécutée par l'orchestration Velum.
10. Migrer les passes et les effets
La gestion des passes doit rester celle de Velum.
- [ ] Recenser les passes actuellement utilisées dans Alpha.
- [ ] Identifier les passes de rendu principales.
- [ ] Identifier les éventuelles passes offscreen.
- [ ] Identifier les effets de post-traitement.
- [ ] Raccorder les render targets.
- [ ] Raccorder les opérations de clear et load.
- [ ] Migrer les dépendances entre passes.
- [ ] Vérifier les formats et les dimensions des attachments.
- [ ] Raccorder la présentation finale à la surface.
- [ ] Vérifier les effets de composition.
- [ ] Vérifier les effets de flou ou autres effets utilisés.
- [ ] Vérifier que les passes fonctionnent avec les matériaux et bindings explicites.
- [ ] Conserver les fonctionnalités historiques de gestion des passes Velum.
Validation : Velum produit la frame complète d'Alpha, effets compris.

Phase 5 — Finalisation
11. Migrer le rapport de frame et les diagnostics
- [ ] Raccorder le début et la fin de frame au reporting Velum.
- [ ] Compter les draw calls.
- [ ] Compter les instances.
- [ ] Compter les passes.
- [ ] Mesurer les octets transférés lorsque les informations sont disponibles.
- [ ] Raccorder les diagnostics d'erreur.
- [ ] Vérifier le comportement d'une frame abandonnée ou échouée.
- [ ] Adapter les outils de diagnostic d'Alpha.
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
- [ ] Supprimer la dépendance Prism.
- [ ] Supprimer les adaptateurs temporaires devenus inutiles.
- [ ] Nettoyer les anciens shaders et types inutilisés.
- [ ] Documenter la nouvelle architecture.
- [ ] Fusionner la branche de migration après validation.
Validation : Alpha fonctionne entièrement avec Velum, sans dépendance à Prism.