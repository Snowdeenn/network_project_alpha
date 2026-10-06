use legion::Entity;
use std::collections::HashMap;
use utils::arena::{Arena, Id};
use utils::ids::PlayerTag;
use utils::protocol::SpellSlot;

use utils::spell_types::{CastCost, SpellId};

#[derive(Debug, Clone, Copy)]
pub struct SpellSlotState {
    pub spell_id: Option<SpellId>,
    pub cooldown_remaining: f32,
    pub charges_remaining: Option<u32>,
}

impl SpellSlotState {
    const EMPTY: Self = Self {
        spell_id: None,
        cooldown_remaining: 0.0,
        charges_remaining: None,
    };
}

#[derive(Debug, Clone)]
pub struct SpellLoadout {
    pub slots: [SpellSlotState; utils::protocol::SPELL_SLOT_COUNT],
}

impl Default for SpellLoadout {
    fn default() -> Self {
        Self {
            slots: [SpellSlotState::EMPTY; utils::protocol::SPELL_SLOT_COUNT],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpellUseError {
    PlayerNotFound,
    SpellNotOwned,
    CooldownActive,
    NotEnoughGold,
    NoMoreCharges,
}

#[derive(Debug, Clone)]
pub struct PlayerEntry {
    pub client_id: u64,
    pub entity: Option<Entity>,
    pub entity_id: Option<u64>,
    pub gold: u32,
    pub spell_loadout: SpellLoadout,
}

pub struct PlayerRegistry {
    arena: Arena<PlayerEntry, PlayerTag>,
    client_to_id: HashMap<u64, Id<PlayerTag>>,
    entity_to_id: HashMap<u64, Id<PlayerTag>>,
}

impl PlayerRegistry {
    pub fn with_capacity(capacity: usize) -> Self {
        Self {
            arena: Arena::with_capacity(capacity),
            client_to_id: HashMap::with_capacity(capacity),
            entity_to_id: HashMap::with_capacity(capacity),
        }
    }

    pub fn add(&mut self, client_id: u64) {
        let entry = PlayerEntry {
            client_id,
            entity: None,
            entity_id: None,
            gold: 0,
            spell_loadout: SpellLoadout::default(),
        };
        let id = self.arena.insert(entry);
        self.client_to_id.insert(client_id, id);
    }

    pub fn link_entity(&mut self, client_id: u64, entity: Entity, entity_id: u64) {
        if let Some(id) = self.client_to_id.get(&client_id) {
            if let Some(entry) = self.arena.get_mut(*id) {
                if let Some(old_entity_id) = entry.entity_id {
                    self.entity_to_id.remove(&old_entity_id);
                }
                entry.entity = Some(entity);
                entry.entity_id = Some(entity_id);
                self.entity_to_id.insert(entity_id, *id);
            }
        }
    }

    pub fn remove(&mut self, client_id: u64) -> Option<PlayerEntry> {
        let id = self.client_to_id.remove(&client_id)?;
        let entry = self.arena.remove(id)?;
        if let Some(entity_id) = entry.entity_id {
            self.entity_to_id.remove(&entity_id);
        }
        Some(entry)
    }

    pub fn get_entity(&self, client_id: u64) -> Option<Entity> {
        self.get_entry(client_id).and_then(|entry| entry.entity)
    }

    pub fn get_entry(&self, client_id: u64) -> Option<&PlayerEntry> {
        self.client_to_id
            .get(&client_id)
            .and_then(|id| self.arena.get(*id))
    }

    fn get_entry_mut(&mut self, client_id: u64) -> Option<&mut PlayerEntry> {
        let id = *self.client_to_id.get(&client_id)?;
        self.arena.get_mut(id)
    }

    pub fn entity_to_client(&self, entity_id: u64) -> Option<u64> {
        self.entity_to_id
            .get(&entity_id)
            .and_then(|id| self.arena.get(*id))
            .map(|entry| entry.client_id)
    }

    pub fn add_gold(&mut self, client_id: u64, amount: u32) {
        if let Some(entry) = self.get_entry_mut(client_id) {
            entry.gold = entry.gold.saturating_add(amount);
        }
    }

    pub fn sub_gold(&mut self, client_id: u64, amount: u32) {
        if let Some(entry) = self.get_entry_mut(client_id) {
            entry.gold = entry.gold.saturating_sub(amount);
        }
    }

    pub fn get_gold(&self, client_id: u64) -> u32 {
        self.get_entry(client_id)
            .map(|entry| entry.gold)
            .unwrap_or(0)
    }

    pub fn iter_clients(&self) -> impl Iterator<Item = u64> + '_ {
        self.client_to_id.keys().copied()
    }

    pub fn first_free_spell_slot(&self, client_id: u64) -> Option<SpellSlot> {
        self.get_entry(client_id)?
            .spell_loadout
            .slots
            .iter()
            .position(|slot| slot.spell_id.is_none())
            .map(SpellSlot::from)
    }

    pub fn spell_at(&self, client_id: u64, spell_slot: SpellSlot) -> Option<SpellId> {
        self.get_entry(client_id)?.spell_loadout.slots[spell_slot.index()].spell_id
    }

    pub fn add_spell(
        &mut self,
        client_id: u64,
        spell_id: SpellId,
        spell_slot: SpellSlot,
        charges: Option<u32>,
    ) -> bool {
        let Some(id) = self.client_to_id.get(&client_id) else {
            tracing::error!("Client {client_id} introuvable dans le registre");
            return false;
        };
        if let Some(entry) = self.arena.get_mut(*id) {
            let slot = &mut entry.spell_loadout.slots[spell_slot.index()];
            if slot.spell_id.is_some() {
                return false;
            }
            *slot = SpellSlotState {
                spell_id: Some(spell_id),
                cooldown_remaining: 0.0,
                charges_remaining: charges,
            };
            return true;
        }
        false
    }

    pub fn try_begin_spell_cast(
        &mut self,
        client_id: u64,
        spell_slot: SpellSlot,
        cost: CastCost,
    ) -> Result<SpellId, SpellUseError> {
        let entry = self
            .get_entry_mut(client_id)
            .ok_or(SpellUseError::PlayerNotFound)?;
        let slot = &mut entry.spell_loadout.slots[spell_slot.index()];
        let spell_id = slot.spell_id.ok_or(SpellUseError::SpellNotOwned)?;

        if slot.cooldown_remaining > 0.0 {
            return Err(SpellUseError::CooldownActive);
        }
        if entry.gold < cost.gold {
            return Err(SpellUseError::NotEnoughGold);
        }
        if matches!(slot.charges_remaining, Some(0)) {
            return Err(SpellUseError::NoMoreCharges);
        }

        entry.gold -= cost.gold;
        slot.cooldown_remaining = cost.cooldown.max(0.0);
        if let Some(charges) = &mut slot.charges_remaining {
            *charges -= 1;
        }
        Ok(spell_id)
    }

    pub fn cooldowns(&self, client_id: u64) -> Option<[f32; utils::protocol::SPELL_SLOT_COUNT]> {
        let entry = self.get_entry(client_id)?;
        Some(std::array::from_fn(|index| {
            entry.spell_loadout.slots[index].cooldown_remaining
        }))
    }

    pub fn update_spell_cooldowns(
        &mut self,
        dt: f32,
    ) -> Vec<(u64, [f32; utils::protocol::SPELL_SLOT_COUNT])> {
        let mut updates = Vec::new();
        for client_id in self.client_to_id.values().copied() {
            let Some(entry) = self.arena.get_mut(client_id) else {
                continue;
            };
            let mut changed = false;
            for slot in &mut entry.spell_loadout.slots {
                if slot.cooldown_remaining > 0.0 {
                    slot.cooldown_remaining = (slot.cooldown_remaining - dt).max(0.0);
                    changed = true;
                }
            }
            if changed {
                updates.push((
                    entry.client_id,
                    std::array::from_fn(|index| {
                        entry.spell_loadout.slots[index].cooldown_remaining
                    }),
                ));
            }
        }
        updates
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use legion::World;

    fn test_spell_id() -> SpellId {
        let mut arena = utils::Arena::<(), utils::ids::SpellTag>::new();
        SpellId::from(arena.insert(()))
    }

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
        assert!(registry.iter_clients().any(|x| x == client_a));

        // 2. Étape Spawn : On crée les entités dans le World et on les link
        let entity_alpha = world.push(());
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
        let removed_player = registry
            .remove(client_a)
            .expect("Le retrait devrait réussir");
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
        assert_eq!(
            registry.get_gold(222),
            0,
            "L'or a fuité sur un autre client !"
        );
    }

    #[test]
    fn test_player_registry_gold_sub() {
        let mut registry = PlayerRegistry::with_capacity(1);
        registry.add(123);

        registry.add_gold(123, 100);
        registry.sub_gold(123, 40);

        assert_eq!(registry.get_gold(123), 60);
    }

    #[test]
    fn test_player_registry_double_remove_idempotence() {
        let mut registry = PlayerRegistry::with_capacity(1);
        registry.add(42);

        assert!(registry.remove(42).is_some());
        assert!(
            registry.remove(42).is_none(),
            "Le deuxième remove aurait dû renvoyer None sans crasher"
        );
    }

    #[test]
    fn test_ghost_client_operations() {
        let mut world = World::default(); // 💡 On réutilise un world de test propre
        let mut registry = PlayerRegistry::with_capacity(1);

        let dummy_entity = world.push(()); // On génère une vraie entité légitime

        // Opérations sur un client qui n'existe pas : aucune ne doit paniquer
        registry.link_entity(999, dummy_entity, 0);
        registry.add_gold(999, 100);

        assert_eq!(registry.get_entity(999), None);
        assert_eq!(registry.get_gold(999), 0);
    }

    #[test]
    fn test_link_entity_twice_cleans_old() {
        let mut world = World::default();
        let mut registry = PlayerRegistry::with_capacity(1);
        registry.add(42);

        let entity_a = world.push(());
        let entity_b = world.push(());

        registry.link_entity(42, entity_a, 1001);
        registry.link_entity(42, entity_b, 1002);

        // L'ancien entity_id doit être purgé
        assert_eq!(registry.entity_to_client(1001), None);
        assert_eq!(registry.entity_to_client(1002), Some(42));
        assert_eq!(registry.get_entity(42), Some(entity_b));
    }

    #[test]
    fn spell_cast_updates_cost_cooldown_and_charges_atomically() {
        let client_id = 42;
        let spell_id = test_spell_id();
        let mut registry = PlayerRegistry::with_capacity(1);
        registry.add(client_id);
        registry.add_gold(client_id, 10);
        assert!(registry.add_spell(client_id, spell_id, SpellSlot::First, Some(2)));

        let cost = CastCost {
            cooldown: 3.0,
            gold: 2,
            charges: Some(2),
        };
        assert_eq!(
            registry.try_begin_spell_cast(client_id, SpellSlot::First, cost),
            Ok(spell_id)
        );
        let entry = registry.get_entry(client_id).unwrap();
        let slot = entry.spell_loadout.slots[SpellSlot::First.index()];
        assert_eq!(entry.gold, 8);
        assert_eq!(slot.cooldown_remaining, 3.0);
        assert_eq!(slot.charges_remaining, Some(1));

        assert_eq!(
            registry.try_begin_spell_cast(client_id, SpellSlot::First, cost),
            Err(SpellUseError::CooldownActive)
        );
        let entry = registry.get_entry(client_id).unwrap();
        assert_eq!(entry.gold, 8);
        assert_eq!(
            entry.spell_loadout.slots[SpellSlot::First.index()].charges_remaining,
            Some(1)
        );

        registry.update_spell_cooldowns(3.0);
        assert_eq!(
            registry.try_begin_spell_cast(client_id, SpellSlot::First, cost),
            Ok(spell_id)
        );
        registry.update_spell_cooldowns(3.0);
        assert_eq!(
            registry.try_begin_spell_cast(client_id, SpellSlot::First, cost),
            Err(SpellUseError::NoMoreCharges)
        );
    }

    #[test]
    fn rejected_cast_does_not_consume_gold_or_charge() {
        let client_id = 7;
        let spell_id = test_spell_id();
        let mut registry = PlayerRegistry::with_capacity(1);
        registry.add(client_id);
        registry.add_gold(client_id, 1);
        assert!(registry.add_spell(client_id, spell_id, SpellSlot::Second, Some(1)));

        let result = registry.try_begin_spell_cast(
            client_id,
            SpellSlot::Second,
            CastCost {
                cooldown: 1.0,
                gold: 2,
                charges: Some(1),
            },
        );
        assert_eq!(result, Err(SpellUseError::NotEnoughGold));
        let entry = registry.get_entry(client_id).unwrap();
        let slot = entry.spell_loadout.slots[SpellSlot::Second.index()];
        assert_eq!(entry.gold, 1);
        assert_eq!(slot.cooldown_remaining, 0.0);
        assert_eq!(slot.charges_remaining, Some(1));
    }
}
