// src/renderer/resources.rs

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

/// Registre centralisé pour l'injection de dépendances et ressources client
pub struct Resources {
    map: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Resources {
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Insère une ressource unique dans le conteneur
    pub fn insert<T: 'static>(&mut self, resource: T) {
        self.map.insert(TypeId::of::<T>(), RefCell::new(Box::new(resource)));
    }

    /// Point d'entrée unique en LECTURE seule depuis tous les systèmes
    pub fn read_resource<T: 'static>(&self) -> Ref<'_, T> {
        let cell = self
            .map
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("Ressource introuvable : {}", std::any::type_name::<T>()));

        Ref::map(cell.borrow(), |b| {
            b.downcast_ref::<T>()
                .expect("Erreur de downcast de la ressource")
        })
    }

    /// Point d'entrée unique en ÉCRITURE depuis tous les systèmes
    pub fn write_resource<T: 'static>(&self) -> RefMut<'_, T> {
        let cell = self
            .map
            .get(&TypeId::of::<T>())
            .unwrap_or_else(|| panic!("Ressource introuvable : {}", std::any::type_name::<T>()));

        RefMut::map(cell.borrow_mut(), |b| {
            b.downcast_mut::<T>()
                .expect("Erreur de downcast de la ressource")
        })
    }
}