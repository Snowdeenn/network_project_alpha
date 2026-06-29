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

    pub fn get(&self, id: &NodeId) -> Option<&T> {
        debug_assert!(id.index < self.nodes.len(), "NodeId index hors bornes");
        debug_assert!(
            id.generation == self.nodes[id.index].generation,
            "NodeId périmé - doit être supprimer"
        );

        if id.index > self.nodes.len()
            || id.generation != self.nodes[id.index].generation
            || self.nodes[id.index].value.is_none()
        {
            return None;
        } else {
            self.nodes[id.index].value.as_ref()
        }
    }

    pub fn get_mut(&mut self, id: &NodeId) -> Option<&mut T> {
        debug_assert!(id.index < self.nodes.len(), "NodeId index hors bornes");
        debug_assert!(
            id.generation == self.nodes[id.index].generation,
            "NodeId périmé - doit être supprimer"
        );

        if id.index > self.nodes.len()
            || id.generation != self.nodes[id.index].generation
            || self.nodes[id.index].value.is_none()
        {
            return None;
        } else {
            self.nodes[id.index].value.as_mut()
        }
    }

    pub fn remove(&mut self, id: &NodeId) -> Option<T> {
        if id.generation != self.nodes[id.index].generation {
            return None;
        } else {
            self.free_slot.push(id.index);
            self.nodes[id.index].value.take()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert() {
        let mut arena: Arena<_> = Arena::new();
        let id = arena.insert(10);
        if let Some(value) = arena.get(&id) {
            assert_eq!(*value, 10);
        }
    }

    #[test]
    fn test_multi_insert() {
        let mut arena: Arena<_> = Arena::new();
        let id1 = arena.insert(1);
        let id2 = arena.insert(2);
        let id3 = arena.insert(3);

        if let Some(value1) = arena.get(&id1) {
            assert_eq!(*value1, 1);
        }
        if let Some(value2) = arena.get(&id2) {
            assert_eq!(*value2, 2)
        }
        if let Some(value3) = arena.get(&id3) {
            assert_eq!(*value3, 3);
        }
    }

    #[test]
    fn test_odler_node() {
        let mut arena = Arena::new();
        let id_ancien = arena.insert(1);
        arena.remove(&id_ancien);
        arena.insert(2);
        assert_eq!(arena.get(&id_ancien), None);
    }
}
