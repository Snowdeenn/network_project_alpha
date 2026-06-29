# CAHIER DES CHARGES : FRAMEWORK UI HAUTE PERFORMANCE (RUST / RAYLIB)

## 1. Présentation du Projet & Objectifs
Le projet consiste à concevoir et développer un framework d'interface utilisateur (UI) spécialisé pour le jeu vidéo 2D, assis sur la bibliothèque graphique Raylib et tirant pleinement parti du langage Rust. 

L'objectif ultime est de fournir un outil permettant de créer des **Heads-Up Displays (HUD) de qualité AAA** : fluides, hautement animés, adaptatifs, tout en déportant un maximum de logique et de vérifications au moment de la compilation (*compile-time*) pour garantir des performances optimales (Zéro-Cost Abstraction).

---

## 2. Spécifications Fonctionnelles (Ce que le framework fait)

### 2.1. Gestion du Layout (Mise en page) et Résolutions
Le HUD doit être parfaitement "responsive" pour s'adapter nativement à n'importe quel écran (du 1080p à la 4K, y compris les ratios Ultra-wide 21:9).
* **Système d'Ancrage :** Positionnement par rapport aux bords ou au centre de l'écran (`TopLeft`, `TopRight`, `BottomLeft`, `BottomRight`, `Center`).
* **Unités Flexibles :** Prise en charge des dimensions en Pixels absolus, Pourcentages (`%`), et unités relatives à la fenêtre (`vw`, `vh`).
* **Marges et Espacements :** Gestion fine des `Margin` (espace extérieur) et `Padding` (espace intérieur).

### 2.2. Gestion des Assets Personnalisés (Custom Assets)
Pour s'affranchir du look "générique", le framework doit intégrer intimement les ressources graphiques du jeu.
* **Rendu 9-Patch (N-Patch) :** Découpage des textures de cadres et de boutons pour permettre leur étirement sans distorsion des bordures.
* **SDF (Signed Distance Fields) pour les Polices :** Rendu des textes vectoriel et ultra-net, sans pixellisation lors des zooms ou des hautes résolutions.
* **Atlas de Textures :** Capacité à lire les coordonnées d'une sous-région d'une texture globale pour afficher icônes et éléments de HUD.

### 2.3. Esthétique et Rendu "AAA"
* **Support des Shaders :** Possibilité d'assigner un *Fragment Shader* personnalisé à n'importe quel composant (effet de jauge d'énergie, distorsion de radar, glitch, surbrillance).
* **Moteur d'Animation (Tweining) :** Système d'interpolation fluide pour animer toutes les propriétés numériques (`Opacité`, `Position`, `Échelle`, `Couleur`) avec des courbes d'assouplissement (*EaseInOut*, *Bounce*, *Elastic*).
* **Effets Post-Process d'Interface :** Gestion du flou d'arrière-plan (Blur) sous les menus et gestion des masques d'affichage (écrans ronds, découpes géométriques).

### 2.4. Interactions et Système Événementiel
* **Data Binding Isolant :** L'UI ne doit pas interroger le moteur de jeu en boucle (*polling*). Elle réagit à des signaux ou des événements envoyés par le jeu.
* **Gestion des Inputs :** Détection du survol (Hover), du clic, du drag-and-drop, et focus pour la navigation à la manette ou au clavier.

---

## 3. Spécifications Techniques (Comment le framework est construit)

### 3.1. Architecture logicielle en Rust
Pour respecter les contraintes du *Borrow Checker* de Rust, l'architecture évitera l'orienté objet traditionnel au profit d'un modèle orienté données.

| Composant | Rôle Technique |
| :--- | :--- |
| **L'Arena (`UIContext`)** | Vecteur central (`Vec<UINode>`) stockant tous les éléments à plat. Les relations parents/enfants se font par indices (`NodeId`). |
| **Le Système de "Dirty Flags"** | Chaque nœud possède un booléen indiquant si son layout ou ses données ont changé. Si `false`, les calculs géométriques sont ignorés à la frame suivante. |
| **Le Pipeline Séparé** | Exécution stricte en 3 passes : `Process_Inputs` (Top-Down) -> `Resolve_Layout` (Bottom-Up/Top-Down) -> `Render` (Top-Down). |

### 3.2. Optimisation au Compile-Time
Le framework doit déléguer un maximum de travail au compilateur Rust (`rustc`) via :
* **Des macros déclaratives/procédurales :** Une macro de type `hud! { ... }` permettant de valider la structure de l'arbre d'UI dès la compilation.
* **Des fonctions `const fn` :** Évaluation au moment de la compilation des styles statiques, des alignements constants et des structures de données fixes.
* **Le typage fort :** Utilisation du système de types de Rust pour empêcher, par exemple, de mélanger des coordonnées en pixels et en pourcentages sans conversion explicite.

### 3.3. Optimisation du Rendu (Pipeline Graphique)
Pour garantir des performances AAA, le framework doit minimiser l'impact sur le GPU.
* **Command Buffer (Batching) :** La passe de rendu ne dessine rien directement. Elle génère une liste de structures de données épurées (`DrawCommand`).
* **Tri des commandes :** Le framework trie les commandes avant exécution pour regrouper les éléments utilisant le même shader ou la même texture, réduisant drastiquement les *Draw Calls* de Raylib.

```rust
// Exemple de structure de commande épurée pour le Batcher
pub struct UIVertex {
    pub position: [f32; 2],
    pub uv: [f32; 2],
    pub color: [u8; 4],
}

## 4. Contraintes et Critères de Performance

* **Fréquence de rafraîchissement :** Le framework ne doit pas impacter la boucle principale du jeu. L'objectif est un coût d'exécution inférieur à **0.5 milliseconde** par frame pour un HUD complet (cible à plus de 144 FPS).
* **Allocation Mémoire :** Zéro allocation mémoire dynamique (`String`, `Vec::push`) dans la boucle de rendu principale une fois l'UI initialisée. Réutilisation de buffers pré-alloués (*Pool Allocation*).
* **Dépendances minimales :** Utiliser uniquement `raylib-rs` pour la couche graphique. L'intégration d'un moteur de layout tiers (ex: `taffy`) est tolérée si elle est justifiée par les performances.

---

## 5. Plan de Développement & Livrables

Le projet sera découpé en 4 phases majeures :

### Phase 1 : Le Cœur de l'Architecture (Fondations)
* Mise en place de l'Arena (`UIContext`) et gestion de la hiérarchie via `NodeId`.
* Implémentation du système de cycle de vie (Update / Draw).
* Création des premières structures de Layout de base (Ancres et marges fixes).

### Phase 2 : Le Moteur de Rendu Évolué
* Création du système de `DrawCommand` (Batching).
* Intégration du composant de rendu de textures personnalisées et support du 9-Patch.
* Mise en place du chargement et de l'application des Shaders Raylib sur les nœuds de l'UI.

### Phase 3 : Dynamique et Animations (Le Look AAA)
* Développement du module de Tweining (interpolations de mouvements et d'opacité).
* Mise en place du système de Dirty Flags pour optimiser les calculs de layout.
* Intégration du système d'événements pour le Data Binding.

### Phase 4 : Ergonomie et Outillage (Compile-Time)
* Création des macros pour instancier l'UI de manière propre et lisible.
* Développement d'une suite de widgets standards (Bouton animé, Barre de progression Shader, Minimap fictive).
* Création d'un projet de démonstration (Demo HUD) mettant en scène les capacités graphiques du framework.