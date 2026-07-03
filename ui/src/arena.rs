#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug, Hash)]
pub struct NodeId {
    pub index: usize,
    pub generation: u32,
}

struct Slot<T> {
    generation: u32,
    value: Option<T>,
}

pub struct Arena<T> {
    nodes: Vec<Slot<T>>,
    free_slot: Vec<usize>,
}

impl<T> Arena<T> {
    pub fn new() -> Self {
        Arena {
            nodes: Vec::new(),
            free_slot: Vec::new(),
        }
    }

    pub fn insert(&mut self, value: T) -> NodeId {
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

        NodeId {
            index,
            generation: self.nodes[index].generation,
        }
    }

    pub fn get(&self, id: NodeId) -> Option<&T> {
        if id.index > self.nodes.len()
            || id.generation != self.nodes[id.index].generation
            || self.nodes[id.index].value.is_none()
        {
            return None;
        } else {
            self.nodes[id.index].value.as_ref()
        }
    }

    pub fn get_mut(&mut self, id: NodeId) -> Option<&mut T> {
        if id.index > self.nodes.len()
            || id.generation != self.nodes[id.index].generation
            || self.nodes[id.index].value.is_none()
        {
            return None;
        } else {
            self.nodes[id.index].value.as_mut()
        }
    }

    pub fn remove(&mut self, id: NodeId) -> Option<T> {
        if id.generation != self.nodes[id.index].generation {
            return None;
        } else {
            self.free_slot.push(id.index);
            self.nodes[id.index].value.take()
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.nodes.iter().filter_map(|slot| slot.value.as_ref())
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.nodes.iter_mut().filter_map(|slot| slot.value.as_mut())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut arena: Arena<_> = Arena::new();
        let id = arena.insert(10);
        let value = arena.get(id).expect("devrait etre la");
        assert_eq!(*value, 10);
    }

    #[test]
    fn test_multi_insert() {
        let mut arena: Arena<_> = Arena::new();
        let id1 = arena.insert(1);
        let id2 = arena.insert(2);
        let id3 = arena.insert(3);

        let value1 = arena.get(id1).unwrap();
        let value2 = arena.get(id2).unwrap();
        let value3 = arena.get(id3).unwrap();

        assert_eq!(*value1, 1);
        assert_eq!(*value2, 2);
        assert_eq!(*value3, 3);
    }

    #[test]
    fn test_remove() {
        let mut arena = Arena::new();
        let id = arena.insert(1);
        arena.remove(id);
        assert_eq!(arena.get(id), None);
    }

    #[test]
    fn test_remove_insert() {
        let mut arena = Arena::new();
        let id_ancien = arena.insert(1);
        arena.remove(id_ancien);
        let nouv_id = arena.insert(2);
        let value = arena.get(nouv_id).unwrap();

        assert_eq!(*value, 2);
    }

    #[test]
    fn test_ancien_id_invalide_apres_reinsert() {
        let mut arena = Arena::new();
        let id_ancien = arena.insert(1);
        arena.remove(id_ancien);
        let _id_nouveau = arena.insert(2);

        assert_eq!(arena.get(id_ancien), None); // id_ancien doit rester mort
    }

    #[test]
    fn test_index_hors_bornes() {
        let arena: Arena<i32> = Arena::new();
        let id_bidon = NodeId {
            index: 99,
            generation: 0,
        };
        assert_eq!(arena.get(id_bidon), None); // pas de panic
    }

    #[test]
    fn test_remove_avec_generation_perimee() {
        let mut arena = Arena::new();
        let id_ancien = arena.insert(1);
        arena.remove(id_ancien); // id_ancien périmé après ça
        let id_nouveau = arena.insert(2); // réutilise le slot, nouvelle gen

        let result = arena.remove(id_ancien); // tentative remove avec gen périmée

        assert_eq!(result, None); // ne doit RIEN retirer
        assert_eq!(arena.get(id_nouveau), Some(&2)); // le nouveau doit survivre intact
    }
}
