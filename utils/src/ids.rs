use crate::arena::Id;
// ======================================
// Tag Client
// ======================================
// Tags vides — zéro dépendance externe
pub struct ShaderTag;
pub struct TextureTag;
pub struct AnimTag;
pub struct AnimEntityTag;
pub struct MaterialTag;

pub type ShaderId = Id<ShaderTag>;
pub type TextureId = Id<TextureTag>;
pub type AnimId = Id<AnimTag>;
pub type AnimEntityId = Id<AnimEntityTag>;
pub type MaterialId = Id<MaterialTag>;
// ======================================
// Tag Server
// ======================================

pub struct EnemyTag;
pub struct CoinTag;
pub struct PlayerTag;
pub struct CooldownTag;
pub struct BufferTag;

pub type EnemyId = Id<EnemyTag>;
pub type CoinId = Id<CoinTag>;
pub type PlayerId = Id<PlayerTag>;
pub type CooldownID = Id<CooldownTag>;
pub type BufferId = Id<BufferTag>;

#[derive(Default)]
pub struct Register {
    inner: std::collections::HashMap<std::any::TypeId, Box<dyn std::any::Any>>,
}

impl Register {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn insert<T: Copy + 'static>(&mut self, key: &'static str, id: T) {
        let type_map = self
            .inner
            .entry(std::any::TypeId::of::<T>())
            .or_insert_with(|| Box::new(std::collections::HashMap::<&'static str, T>::new()));

        let map = type_map
            .downcast_mut::<std::collections::HashMap<&'static str, T>>()
            .expect("Erreur interne lors du downcast de la type map du register");

        if map.insert(key, id).is_some() {
            tracing::warn!(
                "La clé {key} est déjà présente dans le register écrasement de la valeur"
            );
        }
    }

    pub fn get<T: Copy + 'static>(&self, key: &'static str) -> Option<T> {
        let type_map = match self.inner.get(&std::any::TypeId::of::<T>()) {
            Some(type_map) => type_map,
            None => {
                tracing::warn!(
                    "Aucune type map trouver pour {} dans le register",
                    std::any::type_name::<T>()
                );
                return None;
            }
        };
        let map = type_map
            .downcast_ref::<std::collections::HashMap<&'static str, T>>()
            .expect("Echec lors du downcast de la type_map du register");
        match map.get(key) {
            Some(id) => Some(*id),
            None => {
                tracing::warn!("Erreur clé introuvable pour {}", std::any::type_name::<T>());
                None
            }

        }
    }
}
