//! Gestionnaire de buffers réutilisables basé sur une arène d'allocation.
//!
//! Ce module fournit le trait [`AnyBuffer`] et la structure [`BufferManager`]
//! permettant de recycler des conteneurs (`Vec`, `HashSet`, `HashMap`) afin d'éviter
//! les réallocations fréquentes sur le tas.

use std::{
    any::{self}, collections::{HashMap, HashSet},
};

use crate::{
    arena::Arena,
    ids::{BufferId, BufferTag},
};

/// Trait d'abstraction pour tout type de buffer réinitialisable.
///
/// Il permet la manipulation opaque et dynamique via [`std::any::Any`],
/// tout en imposant la sécurité multi-thread (`Send + Sync`).
pub trait AnyBuffer: Send + Sync {
    /// Retourne une référence immuable sous forme de [`dyn any::Any`](any::Any).
    fn as_any(&self) -> &dyn any::Any;

    /// Retourne une référence mutable sous forme de [`dyn any::Any`](any::Any).
    fn as_any_mut(&mut self) -> &mut dyn any::Any;

    /// Efface le contenu du buffer sans nécessairement libérer sa mémoire allouée.
    fn clear(&mut self);
}

impl<T: 'static + Send + Sync> AnyBuffer for Vec<T> {
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

impl<T: 'static + Send + Sync> AnyBuffer for HashSet<T> {
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

impl<K: 'static + Send + Sync, V: 'static + Send + Sync> AnyBuffer for HashMap<K, V> {
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

impl AnyBuffer for String {
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

/// Gestionnaire d'allocations et de recyclage de buffers.
///
/// S'appuie sur une [`Arena`] pour réutiliser efficacement les emplacements
/// mémoire libérés et limiter les désallocations.
pub struct BufferManager {
    buffers: Arena<Box<dyn AnyBuffer>, BufferTag>,
}

impl BufferManager {
    /// Crée un nouveau `BufferManager` avec une capacité initiale d'emplacements pré-allouée.
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            buffers: Arena::with_capacity(capacity),
        }
    }

    /// Récupère ou alloue un identifiant ([`BufferId`]) pour un buffer de type `C`.
    ///
    /// Si un emplacement disponible contient un type compatible, son identifiant est réutilisé.
    /// Sinon, une nouvelle instance par défaut (`C::default()`) est insérée.
    #[must_use]
    #[inline]
    pub fn acquire_id<C: AnyBuffer + Default + 'static>(&mut self) -> BufferId {
        if let Some(id) = self.buffers.acquire() {
            if let Some(buffer) = self.buffers.get(id) {
                if buffer.as_any().downcast_ref::<C>().is_some() {
                    return id;
                }
            }

            self.buffers.release_index(id);
        }
        self.buffers.insert(Box::new(C::default()))
    }

    /// Réserve un buffer de type `C` et retourne un tuple contenant son [`BufferId`]
    /// ainsi qu'une référence mutable vers le conteneur.
    ///
    /// Retourne `None` si la tentative de conversion vers le type `C` échoue.
    #[must_use]
    #[inline]
    pub fn acquire<'a, C: AnyBuffer + Default + 'static>(
        &'a mut self,
    ) -> Option<(BufferId, &'a mut C)> {
        let id = self.acquire_id::<C>();
        let buffer = self.buffers.get_mut(id)?;
        let collection = buffer.as_any_mut().downcast_mut::<C>()?;
        Some((id, collection))
    }

    /// Libère le buffer associé à l'identifiant fourni.
    ///
    /// Le buffer est immédiatement réinitialisé via [`AnyBuffer::clear`] et
    /// l'emplacement est remis à disposition dans l'arène.
    #[inline]
    pub fn release(&mut self, id: BufferId) {
        if let Some(buffer) = self.buffers.get_mut(id) {
            buffer.clear();
        }
        self.buffers.release_index(id);
    }

    /// Récupère une référence immuable vers le buffer identifié par `id`.
    ///
    /// Retourne `None` si l'identifiant n'est plus valide ou si le type `C` est incorrect.
    #[must_use]
    #[inline]
    pub fn get<'a, C: AnyBuffer + Default + 'static>(&'a self, id: BufferId) -> Option<&'a C> {
        let buffer = self.buffers.get(id)?;
        buffer.as_any().downcast_ref::<C>()
    }

    /// Récupère une référence mutable vers le buffer identifié par `id`.
    ///
    /// Retourne `None` si l'identifiant n'est plus valide ou si le type `C` est incorrect.
    #[must_use]
    #[inline]
    pub fn get_mut<'a, C: AnyBuffer + Default + 'static>(
        &'a mut self,
        id: BufferId,
    ) -> Option<&'a mut C> {
        let buffer = self.buffers.get_mut(id)?;
        buffer.as_any_mut().downcast_mut::<C>()
    }
}