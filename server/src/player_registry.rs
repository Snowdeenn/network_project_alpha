// src/player_registry.rs
use shared::arena::{Arena, Id};
use legion::Entity;
use std::collections::HashMap;
use shared::ids::PlayerTag; 

#[derive(Debug, Clone)]
pub struct PlayerEntry {
    pub client_id: u64,
    pub entity: Option<Entity>,     // 💡 Optionnel au début !
    pub entity_id: Option<u64>,   // 💡 Optionnel au début !
    pub gold: u32,
}

pub struct PlayerRegistry {
    arena: Arena<PlayerEntry, PlayerTag>,
    client_to_id: HashMap<u64, Id<PlayerTag>>,
    entity_to_id: HashMap<u64, Id<PlayerTag>>, // Ne contiendra le joueur que lorsqu'il aura spawn
}

impl PlayerRegistry {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
            client_to_id: HashMap::with_capacity(capacity),
            entity_to_id: HashMap::with_capacity(capacity),
        }
    }

    // API: add -> Appelé au ClientConnected (Lobby)
    pub fn add(&mut self, client_id: u64) {
        let entry = PlayerEntry {
            client_id,
            entity: None,
            entity_id: None,
            gold: 0,
        };
        let id = self.arena.insert(entry);
        self.client_to_id.insert(client_id, id);
    }

    // 🌟 NOUVELLE API: À appeler dans ton système de lobby quand l'entité est VRAIMENT push dans le world
    pub fn link_entity(&mut self, client_id: u64, entity: Entity, entity_id: u64) {
        if let Some(id) = self.client_to_id.get(&client_id) {
            if let Some(entry) = self.arena.get_mut(*id) {
                entry.entity = Some(entity);
                entry.entity_id = Some(entity_id);
                
                // On indexe l'ID d'entité pour les lookups inverses en O(1)
                self.entity_to_id.insert(entity_id, *id);
            }
        }
    }

    // API: remove -> Gère proprement si le joueur avait spawn ou non
    pub fn remove(&mut self, client_id: u64) -> Option<PlayerEntry> {
        if let Some(id) = self.client_to_id.remove(&client_id) {
            if let Some(entry) = self.arena.remove(id) {
                if let Some(entity_id) = entry.entity_id {
                    self.entity_to_id.remove(&entity_id);
                }
                return Some(entry);
            }
        }
        None
    }

    // API: get_entity -> Renvoie None si le joueur est juste dans le lobby
    pub fn get_entity(&self, client_id: u64) -> Option<Entity> {
        self.client_to_id.get(&client_id)
            .and_then(|id| self.arena.get(*id))
            .and_then(|entry| entry.entity)
    }

    // API: entity_to_client
    pub fn entity_to_client(&self, entity_id: u64) -> Option<u64> {
        self.entity_to_id.get(&entity_id)
            .and_then(|id| self.arena.get(*id))
            .map(|entry| entry.client_id)
    }

    // API: add_gold
    pub fn add_gold(&mut self, client_id: u64, amount: u32) {
        if let Some(id) = self.client_to_id.get(&client_id) {
            if let Some(entry) = self.arena.get_mut(*id) {
                // Pour éviter d'écraser si amount est négatif (via la ruse du shop)
                entry.gold = entry.gold.saturating_add_signed(amount as i32);
            }
        }
    }

    // Helper pour le shop (évite de fouiller l'arène manuellement)
    pub fn get_gold(&self, client_id: u64) -> u32 {
        self.client_to_id.get(&client_id)
            .and_then(|id| self.arena.get(*id))
            .map(|entry| entry.gold)
            .unwrap_or(0)
    }

    pub fn active_client_ids(&self) -> Vec<u64> {
        self.client_to_id.keys().copied().collect()
    }
}

// Tout en bas de src/player_registry.rs

#[cfg(test)]
mod tests {
    use super::*;
    use legion::World;

    #[test]
    fn test_player_registry_lobby_to_spawn_lifecycle() {
        let mut world = World::default();
        let mut registry = PlayerRegistry::with_capacity(2);

        let client_a = 42;
        let client_b = 99;

        // 1. Étape Connexion / Lobby : On ajoute les joueurs sans entité
        registry.add(client_a);
        registry.add(client_b);

        // L'entité doit être à None, mais le client est actif
        assert_eq!(registry.get_entity(client_a), None);
        assert!(registry.active_client_ids().contains(&client_a));

        // 2. Étape Spawn : On crée les entités dans le World et on les link
        let entity_alpha = world.push((),);
        let ent_id_a = 1001;
        registry.link_entity(client_a, entity_alpha, ent_id_a);

        // Maintenant, l'entité doit être récupérable en O(1)
        assert_eq!(registry.get_entity(client_a), Some(entity_alpha));
        assert_eq!(registry.entity_to_client(ent_id_a), Some(client_a));

        // Le joueur B, lui, n'a toujours pas d'entité
        assert_eq!(registry.get_entity(client_b), None);

        // 3. Gestion de l'or (S'accumule correctement via get_gold / add_gold)
        assert_eq!(registry.get_gold(client_a), 0);
        registry.add_gold(client_a, 150);
        registry.add_gold(client_a, 50); 
        assert_eq!(registry.get_gold(client_a), 200);

        // 4. Déconnexion et nettoyage
        let removed_player = registry.remove(client_a).expect("Le retrait devrait réussir");
        assert_eq!(removed_player.client_id, client_a);
        assert_eq!(removed_player.entity, Some(entity_alpha));

        // Vérification des purges d'index
        assert_eq!(registry.get_entity(client_a), None);
        assert_eq!(registry.entity_to_client(ent_id_a), None);
    }

    #[test]
    fn test_player_registry_gold_multi_client_isolation() {
        let mut registry = PlayerRegistry::with_capacity(2);
        
        registry.add(111);
        registry.add(222);

        // On donne de l'or uniquement au premier
        registry.add_gold(111, 500);

        assert_eq!(registry.get_gold(111), 500);
        assert_eq!(registry.get_gold(222), 0, "L'or a fuité sur un autre client !");
    }

    #[test]
    fn test_player_registry_add_gold_signed_math() {
        let mut registry = PlayerRegistry::with_capacity(1);
        registry.add(123);
        
        registry.add_gold(123, 100);
        
        // Simulation d'un achat au shop (valeur négative castée en u32)
        let price = 40u32;
        let negative_amount = 0u32.wrapping_sub(price); // Équivalent à -40
        
        registry.add_gold(123, negative_amount);
        
        // Grâce à saturating_add_signed, 100 - 40 = 60
        assert_eq!(registry.get_gold(123), 60);
    }

    #[test]
    fn test_player_registry_double_remove_idempotence() {
        let mut registry = PlayerRegistry::with_capacity(1);
        registry.add(42);

        assert!(registry.remove(42).is_some());
        assert!(registry.remove(42).is_none(), "Le deuxième remove aurait dû renvoyer None sans crasher");
    }

    #[test]
    fn test_ghost_client_operations() {
        let mut world = World::default(); // 💡 On réutilise un world de test propre
        let mut registry = PlayerRegistry::with_capacity(1);
        
        let dummy_entity = world.push((),); // On génère une vraie entité légitime
        
        // Opérations sur un client qui n'existe pas : aucune ne doit paniquer
        registry.link_entity(999, dummy_entity, 0);
        registry.add_gold(999, 100);
        
        assert_eq!(registry.get_entity(999), None);
        assert_eq!(registry.get_gold(999), 0);
    }
}