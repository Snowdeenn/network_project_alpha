use std::{any, collections::{HashMap, HashSet}};

use crate::{
    arena::Arena,
    ids::{BufferId, BufferTag},
};

pub trait AnyBuffer {
    fn as_any(&self) -> &dyn any::Any;
    fn as_any_mut(&mut self) -> &mut dyn any::Any;
    fn clear(&mut self);
}

impl<T: 'static> AnyBuffer for Vec<T> {
    fn as_any(&self) -> &dyn any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn any::Any {
        self
    }
    fn clear(&mut self) {
        self.clear();
    }
}

impl<T: 'static> AnyBuffer for HashSet<T> {
    fn as_any(&self) -> &dyn any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn any::Any {
        self
    }
    fn clear(&mut self) {
        self.clear();
    }
}

impl<K: 'static, V: 'static> AnyBuffer for HashMap<K, V> {
    fn as_any(&self) -> &dyn any::Any {
        self
    }
    fn as_any_mut(&mut self) -> &mut dyn any::Any {
        self
    }
    fn clear(&mut self) {
        self.clear();
    }
}

pub struct BufferManager {
    buffers: Arena<Box<dyn AnyBuffer>, BufferTag>,
}

impl BufferManager {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffers: Arena::with_capacity(capacity),
        }
    }

    #[must_use]
    #[inline]
    pub fn acquire<'a, C: AnyBuffer + Default + 'static>(&'a mut self) -> Option<(BufferId, &'a mut C)> {
        let id = match self.buffers.acquire() {
            Some(id) => id,
            None => self.buffers.insert(Box::new(C::default())),
        };

        let buffer = self.buffers.get_mut(id)?;
        let collection = buffer.as_any_mut().downcast_mut::<C>()?;
        Some((id, collection))
    }

    #[inline]
    pub fn release(&mut self, id: BufferId) {
        if let Some(buffer) = self.buffers.get_mut(id) {
            buffer.clear();
        }
        self.buffers.release_index(id);
    }

    #[must_use]
    #[inline]
    pub fn get<'a, C: AnyBuffer + Default + 'static>(&'a self, id: BufferId) -> Option<&'a C> {
        let buffer = self.buffers.get(id)?;
        buffer.as_any().downcast_ref::<C>()
    }

    #[must_use]
    #[inline]
    pub fn get_mut<'a, C: AnyBuffer + Default + 'static>(&'a mut self, id: BufferId) -> Option<&'a mut C> {
        let buffer = self.buffers.get_mut(id)?;
        buffer.as_any_mut().downcast_mut::<C>()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq, Clone, Copy)]
    struct EntityState {
        id: u32,
    }

    #[test]
    fn test_buffer_manager_basic_cycle() {
        let mut manager = BufferManager::with_capacity(2);

        let (id1, vec1) = manager.acquire::<Vec<u8>>().expect("Devrait allouer un buffer");
        assert!(vec1.is_empty(), "Le buffer initial doit être vide");

        vec1.push(10);
        vec1.push(20);
        assert_eq!(vec1.len(), 2);

        manager.release(id1);

        let (id2, vec2) = manager
            .acquire::<Vec<u8>>()
            .expect("Devrait réallouer un buffer");
        assert_eq!(
            id1.index, id2.index,
            "Le gestionnaire aurait dû recycler le même slot"
        );
        assert_ne!(
            id1.generation, id2.generation,
            "La génération doit avoir changé"
        );
        assert!(
            vec2.is_empty(),
            "Le buffer recyclé aurait dû être nettoyé (.clear())"
        );
    }

    #[test]
    fn test_buffer_manager_different_types() {
        let mut manager = BufferManager::with_capacity(2);

        let (id_u8, _vec_u8) = manager.acquire::<Vec<u8>>().unwrap();
        let (id_entity, _vec_entity) = manager.acquire::<Vec<EntityState>>().unwrap();

        assert_ne!(
            id_u8, id_entity,
            "Des types différents doivent résider dans des slots distincts"
        );

        assert!(
            manager.get::<Vec<EntityState>>(id_u8).is_none(),
            "Le downcast vers un mauvais type doit échouer"
        );
        assert!(
            manager.get_mut::<Vec<EntityState>>(id_u8).is_none(),
            "Le downcast mut vers un mauvais type doit échouer"
        );
    }

    #[test]
    fn test_buffer_manager_capacity_overflow() {
        let mut manager = BufferManager::with_capacity(1);

        let (id1, _vec1) = manager.acquire::<Vec<u8>>().unwrap();
        manager.release(id1);

        let (_id2, vec2) = manager.acquire::<Vec<u8>>().expect("Devrait recycler le slot");
        assert!(vec2.is_empty());

        let (_id3, _vec3) = manager
            .acquire::<Vec<u8>>()
            .expect("Devrait créer un nouveau slot");
    }

    #[test]
    fn test_buffer_manager_get_and_get_mut() {
        let mut manager = BufferManager::with_capacity(2);
        let (id, vec) = manager.acquire::<Vec<u8>>().unwrap();
        vec.push(42);

        let vec_ref = manager
            .get::<Vec<u8>>(id)
            .expect("Id valide devrait retourner une référence");
        assert_eq!(vec_ref, &vec![42]);

        let vec_mut = manager
            .get_mut::<Vec<u8>>(id)
            .expect("Id valide devrait retourner une référence mutable");
        vec_mut.push(99);

        assert_eq!(manager.get::<Vec<u8>>(id).unwrap(), &vec![42, 99]);
    }

    #[test]
    fn test_buffer_manager_release_edge_cases() {
        let mut manager = BufferManager::with_capacity(2);
        let (id, vec) = manager.acquire::<Vec<u8>>().unwrap();
        vec.push(1);

        manager.release(id);
        assert!(
            manager.get::<Vec<u8>>(id).is_none(),
            "Un slot libéré ne devrait plus être accessible via get"
        );
        assert!(
            manager.get_mut::<Vec<u8>>(id).is_none(),
            "Un slot libéré ne devrait plus être accessible via get_mut"
        );

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            manager.release(id);
        }));
        assert!(result.is_ok(), "Le double release a provoqué une panique !");
    }
}
