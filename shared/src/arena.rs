use std::marker::PhantomData;
use std::hash::Hash;
use std::fmt::Debug;

pub struct Id<Tag> {
    pub index: usize,
    pub generation: u32,
    _phantom: PhantomData<Tag>,
}

struct Slot<Data> {
    generation: u32,
    value: Option<Data>,
}

pub struct Arena<Data, Tag = Data> {
    nodes: Vec<Slot<Data>>,
    free_slot: Vec<usize>,
    _phantom: PhantomData<Tag>,
}

impl<Data, Tag> Arena<Data, Tag> {
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            free_slot: Vec::new(),
            _phantom: PhantomData,
        }
    }

    pub fn insert(&mut self, value: Data) -> Id<Tag> {
        let index = if let Some(free_index) = self.free_slot.pop() {
            self.nodes[free_index].generation += 1;
            self.nodes[free_index].value = Some(value);
            free_index
        } else {
            let new_index = self.nodes.len();
            self.nodes.push(Slot {
                generation: 0,
                value: Some(value),
            });
            new_index
        };

        Id {
            index,
            generation: self.nodes[index].generation,
            _phantom: PhantomData,
        }
    }

    pub fn get(&self, id: Id<Tag>) -> Option<&Data> {
        if id.index >= self.nodes.len()
            || id.generation != self.nodes[id.index].generation
            || self.nodes[id.index].value.is_none()
        {
            return None;
        }
        self.nodes[id.index].value.as_ref()
    }

    pub fn get_mut(&mut self, id: Id<Tag>) -> Option<&mut Data> {
        if id.index >= self.nodes.len()
            || id.generation != self.nodes[id.index].generation
            || self.nodes[id.index].value.is_none()
        {
            return None;
        }
        self.nodes[id.index].value.as_mut()
    }

    pub fn remove(&mut self, id: Id<Tag>) -> Option<Data> {
        if id.index >= self.nodes.len()
            || id.generation != self.nodes[id.index].generation
        {
            return None;
        }
        self.free_slot.push(id.index);
        self.nodes[id.index].value.take()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Data> {
        self.nodes.iter().filter_map(|slot| slot.value.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut Data> {
        self.nodes.iter_mut().filter_map(|slot| slot.value.as_mut())
    }
}

impl<T> PartialEq for Id<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index && self.generation == other.generation
    }
}
impl<T> Eq for Id<T> {}
impl<T> Hash for Id<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.index.hash(state);
        self.generation.hash(state);
    }
}
impl<T> Debug for Id<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Id {{ index: {}, generation: {} }}", self.index, self.generation)
    }
}

impl<T> Copy for Id<T> {}
impl<T> Clone for Id<T> {
    fn clone(&self) -> Self { *self }
}

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     fn test_insert() {
//         let mut arena: Arena<_> = Arena::new();
//         let id = arena.insert(10);
//         let value = arena.get(id).expect("devrait etre la");
//         assert_eq!(*value, 10);
//     }

//     #[test]
//     fn test_multi_insert() {
//         let mut arena: Arena<_> = Arena::new();
//         let id1 = arena.insert(1);
//         let id2 = arena.insert(2);
//         let id3 = arena.insert(3);

//         let value1 = arena.get(id1).unwrap();
//         let value2 = arena.get(id2).unwrap();
//         let value3 = arena.get(id3).unwrap();

//         assert_eq!(*value1, 1);
//         assert_eq!(*value2, 2);
//         assert_eq!(*value3, 3);
//     }

//     #[test]
//     fn test_remove() {
//         let mut arena = Arena::new();
//         let id = arena.insert(1);
//         arena.remove(id);
//         assert_eq!(arena.get(id), None);
//     }

//     #[test]
//     fn test_remove_insert() {
//         let mut arena = Arena::new();
//         let id_ancien = arena.insert(1);
//         arena.remove(id_ancien);
//         let nouv_id = arena.insert(2);
//         let value = arena.get(nouv_id).unwrap();

//         assert_eq!(*value, 2);
//     }

//     #[test]
//     fn test_ancien_id_invalide_apres_reinsert() {
//         let mut arena = Arena::new();
//         let id_ancien = arena.insert(1);
//         arena.remove(id_ancien);
//         let _id_nouveau = arena.insert(2);

//         assert_eq!(arena.get(id_ancien), None);
//     }

//     #[test]
//     fn test_index_hors_bornes() {
//         let arena: Arena<i32> = Arena::new();
//         let id_bidon = Id {
//             index: 99,
//             generation: 0,
//             _phantom: PhantomData
//         };
//         assert_eq!(arena.get(id_bidon), None);
//     }

//     #[test]
//     fn test_remove_avec_generation_perimee() {
//         let mut arena = Arena::new();
//         let id_ancien = arena.insert(1);
//         arena.remove(id_ancien);
//         let id_nouveau = arena.insert(2);

//         let result = arena.remove(id_ancien);

//         assert_eq!(result, None);
//         assert_eq!(arena.get(id_nouveau), Some(&2));
//     }
// }