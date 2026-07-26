# Roadmap Renderer & VFX — Project Alpha

---

## Existant réutilisable

| Système | État |
|---|---|
| `ShaderManager` sur `Arena<Shader, ShaderTag>` | ✅ Solide — à étendre |
| `TextureManager` sur `Arena<Texture2D, TextureTag>` | ✅ Solide — à aligner sur `AssetManager` |
| `AnimationManager` | ✅ À garder — supprimer `texture.rs` (doublon) |
| `BufferManager` partagé | ✅ Transverse — prêt à l'emploi |
| `ParticleSystem` basique | ⚠️ À refactorer en pool fixe |

---

## Phase 0 — Nettoyage

- Supprimer `texture.rs` — doublon de `animation_manager.rs`
- Extraire `InGame` de `main.rs` dans `screens/ingame.rs`
- Réorganiser l'arborescence des modules renderer

---

## Phase 1 — Fondations renderer

Ordre strict à respecter — chaque étape est prérequis de la suivante.

**1. `AssetManager` — registre central**
- Struct unique exposée en ressource Legion
- Contient `TextureManager` + `AnimationManager` comme sous-managers
- Expose `textures()` / `textures_mut()` et `anims()` / `anims_mut()` pour des borrows disjoints propres
- Point d'entrée unique depuis tous les systèmes via `read_resource` / `write_resource`

**2. `TextureManager` refactor**
- `TextureId` générationnel via l'arène
- Registry centralisé — plus aucun système ne stocke de `Texture2D` directement
- API : `load(path) -> TextureId`, `get(id) -> Option<&Texture2D>`

**3. `AnimationManager` refactor**
- Utilise `TextureId` du `TextureManager` au lieu de stocker les `Texture2D` directement
- `AnimId` générationnel via l'arène
- Dépend de `TextureId` — ne peut commencer qu'après le point 2

**4. `ShaderManager` extension**
- Passes nommées (world, vfx, hud, post-process)
- Uniforms par batch
- *(hot_reload → Phase 4)*

**5. `RenderPipeline`**
- Passes ordonnées : `world → vfx → hud → post-process`
- Le slot `post-process` est réservé dès maintenant, même vide — évite un refactor en Phase 3
- Dépend de `ShaderManager` étendu

---

## Phase 2 — Particules & VFX

*Prérequis : Phase 1 complète, notamment `ShaderManager` (flash impact = shader).*

**`ParticlePool`**
- Pool fixe de `Particle` — taille dimensionnée au max de particules simultanées
- API acquire / release — zéro allocation après init
- Ring buffer interne

**Particules gameplay**
- Poussière — run + changement de direction
- Respiration — timer idle
- Impact — `EntityHit`
- Mort ennemi — burst

**`VfxManager`**
- Slash effect via `DrawRing` adapté aux dimensions et à l'orientation de l'`AttackBox`
  *(projection repère monde → repère écran, fonction utilitaire dédiée)*
- Flash impact ennemi — inversion couleur 1 frame via shader pass
- Trail épée — ring buffer de N positions historisées + lerp
  *(N = paramètre explicite, détermine la longueur visuelle)*
- VFX dash — traînée semi-transparente

---

## Phase 3 — Feel & polish

- Lerp arme derrière joueur
- Camera shake sur impacts forts — offset sur la matrice de vue
- Tween apparition UI — scale 0→1 avec rebond
- Glow simulation — sprite dupliqué + `BLEND_ADDITIVE` Raylib
  *(rendu du sprite dupliqué avant le sprite normal)*
- Shader bloom post-process — `RenderTexture2D` Raylib dans le slot post-process réservé en Phase 1
- Lumières douces projectiles ennemis

---

## Phase 4 — DX & perf

- `ShaderManager` hot_reload — file watching + recompilation, derrière un feature flag
- `AnimEntityManager` sur `Arena<AnimEntity, AnimEntityTag>` — remplace `HashMap<u64, AnimEntity>`
- `HudUpdater` — unifie les 5 `send_event` HUD
- `format!` HUD → `write!` sur buffers pré-alloués via `BufferManager`
- Sérialisation réseau client zéro alloc *(spécifique client Project Alpha)*

---

## Dépendances inter-phases

```
TextureId (Ph1)
    └── AnimId (Ph1)
            └── AnimEntityManager (Ph4)

ShaderManager passes (Ph1)
    └── Flash impact (Ph2)
    └── Bloom (Ph3)

RenderPipeline slot post-process réservé (Ph1)
    └── Bloom branché dessus (Ph3)
```
