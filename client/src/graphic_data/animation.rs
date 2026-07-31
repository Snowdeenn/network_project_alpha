// src/graphic_data/animation.rs

use crate::graphic_data::animation_manager::{AnimData, AnimationManager};
use utils::arena::Arena;
use utils::ids::{AnimEntityId, AnimEntityTag, AnimId, TextureId};
use std::collections::HashMap;

#[derive(Debug)]
pub struct AnimEntity {
    current_id: AnimId,
    current_frame: usize,
    timer: f32,
}

impl AnimEntity {
    pub fn new(id: AnimId) -> Self {
        Self {
            current_id: id,
            current_frame: 0,
            timer: 0.0,
        }
    }

    pub fn set(&mut self, id: AnimId) {
        if self.current_id != id {
            self.current_id = id;
            self.current_frame = 0;
            self.timer = 0.0;
        }
    }

    pub fn tick(&mut self, dt: f32, data: &AnimData) {
        self.timer += dt;
        if self.timer >= data.frame_time {
            self.timer = 0.0;
            let next = self.current_frame + 1;
            self.current_frame = if next < data.frames.len() {
                next
            } else if data.looping {
                0
            } else {
                self.current_frame
            };
        }
    }

    pub fn current_texture_id(&self, data: &AnimData) -> Option<TextureId> {
        data.frames.get(self.current_frame).copied()
    }
}

pub struct AnimEntityManager {
    arena: Arena<AnimEntity, AnimEntityTag>,
    lookup: HashMap<u64, AnimEntityId>,
}

impl AnimEntityManager {
    pub fn new() -> Self {
        Self {
            arena: Arena::new(),
            lookup: HashMap::new(),
        }
    }

    /// Retourne l'`AnimEntity` existante pour cet entity_id, ou en crée une nouvelle.
    pub fn get_or_create(&mut self, entity_id: u64, anim_id: AnimId) -> &mut AnimEntity {
        let arena = &mut self.arena;

        let handle = self
            .lookup
            .entry(entity_id)
            .or_insert_with(|| arena.insert(AnimEntity::new(anim_id)));

        self.arena
            .get_mut(*handle)
            .expect("Handle AnimEntity invalide")
    }

    /// Récupère une référence immutable à l'AnimEntity d'une entité.
    pub fn get(&self, entity_id: u64) -> Option<&AnimEntity> {
        let handle = self.lookup.get(&entity_id)?;
        self.arena.get(*handle)
    }

    /// Avance toutes les animations d'un dt en une seule passe sur l'arène.
    /// Plus cache-friendly que de ticker chaque entité dans la boucle de rendu.
    pub fn tick_all(&mut self, dt: f32, anim_manager: &AnimationManager) {
        for anim in self.arena.iter_mut() {
            if let Some(data) = anim_manager.get(anim.current_id) {
                anim.tick(dt, data);
            }
        }
    }

    /// Supprime les AnimEntity dont l'entity_id réseau n'est plus actif.
    /// À appeler après chaque snapshot avec la liste des entités vivantes.
    pub fn retain(&mut self, is_active: impl Fn(u64) -> bool) {
        let arena = &mut self.arena;
        self.lookup.retain(|&id, handle| {
            if is_active(id) {
                true
            } else {
                arena.remove(*handle);
                false
            }
        });
    }
}
