//! Registre et conteneur de ressources pour l'injection de dépendances.
//!
//! Fournit la structure [`Resources`] qui permet le stockage et le partage
//! de structures uniques (singletons) via de la mutabilité interne.

use std::any::{Any, TypeId};
use std::cell::{Ref, RefCell, RefMut};
use std::collections::HashMap;

/// Registre centralisé pour l'injection de dépendances et la gestion des ressources clients.
///
/// Chaque type de ressource est identifié de façon unique par son [`TypeId`].
/// L'accès concurrent au sein d'un même thread est géré dynamiquement par [`RefCell`].
pub struct Resources {
    map: HashMap<TypeId, RefCell<Box<dyn Any>>>,
}

impl Resources {
    /// Crée une nouvelle instance vide du registre de ressources.
    pub fn new() -> Self {
        Self {
            map: HashMap::new(),
        }
    }

    /// Insère une ressource unique dans le conteneur.
    ///
    /// Si une ressource du même type `T` existait déjà, elle sera écrasée.
    pub fn insert<T: 'static>(&mut self, resource: T) {
        self.map.insert(TypeId::of::<T>(), RefCell::new(Box::new(resource)));
    }

    /// Fournit un accès en lecture seule à une ressource de type `T`.
    ///
    /// # Panics
    ///
    /// Panique si :
    /// - La ressource de type `T` n'a pas été insérée au préalable dans le conteneur.
    /// - La ressource est actuellement empruntée en écriture ([`write_resource`](Self::write_resource)).
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

    /// Fournit un accès en écriture (mutable) à une ressource de type `T`.
    ///
    /// # Panics
    ///
    /// Panique si :
    /// - La ressource de type `T` n'a pas été insérée au préalable dans le conteneur.
    /// - La ressource est déjà empruntée (en lecture ou en écriture).
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