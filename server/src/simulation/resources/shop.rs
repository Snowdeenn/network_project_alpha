use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use utils::{
    protocol::SpellSlot,
    spell_types::{RawSpell, Spell},
};

pub struct SpellPool {
    pub items: Vec<Option<RawSpell>>,
}

pub struct PlayerShops {
    pub inventories: HashMap<u64, Vec<Option<(String, Spell)>>>,
}

impl PlayerShops {
    pub fn new() -> Self {
        Self {
            inventories: HashMap::new(),
        }
    }

    pub fn generate(
        &mut self,
        player_id: u64,
        item_pool: &[Option<RawSpell>],
    ) -> Vec<Option<(String, Spell)>> {
        let count = item_pool.len().min(3);
        let items: Vec<Option<(String, Spell)>> = item_pool
            .sample(&mut rand::rng(), count)
            .map(|opt| opt.as_ref().map(|raw| raw.clone().into_spell()))
            .collect();
        self.inventories.insert(player_id, items.clone());
        items
    }

    pub fn buy(
        &mut self,
        player_id: u64,
        slot: usize,
        gold_avaible: u32,
    ) -> Option<(String, Spell)> {
        let inventory = self.inventories.get_mut(&player_id)?;
        let (_, item) = inventory.get(slot)?.as_ref()?;
        if item.costs.gold <= gold_avaible {
            return inventory.get_mut(slot)?.take();
        }
        None
    }
}

pub fn process_shop_action(net: &mut crate::net::GameNetServer, resources: &mut legion::Resources) {
    let actions: Vec<(u64, utils::protocol::ShopAction)> = {
        let mut buff_manager = resources.get_mut::<utils::buffer::BufferManager>().unwrap();
        let action_id = buff_manager.acquire_id::<Vec<(u64, utils::protocol::ShopAction)>>();
        let actions = buff_manager
            .get_mut::<Vec<(u64, utils::protocol::ShopAction)>>(action_id)
            .unwrap();
        net.drain_shop_actions_into(actions);
        let owned = std::mem::take(actions);
        buff_manager.release(action_id);
        owned
    };
    for (client_id, shop_action) in actions {
        handle_shop_action(client_id, net, shop_action, resources);
    }
}

fn handle_shop_action(
    client: u64,
    server: &mut crate::net::GameNetServer,
    action: utils::protocol::ShopAction,
    res: &mut legion::Resources,
) {
    match action.kind {
        utils::protocol::ShopActionKind::Open => {
            tracing::info!("Client {} a ouvert le shop", client);

            let shop_inventory: Vec<Option<Spell>> = {
                let item_pool = res.get::<SpellPool>().unwrap();
                let mut player_shops = res.get_mut::<PlayerShops>().unwrap();
                player_shops
                    .generate(client, &item_pool.items)
                    .into_iter()
                    .map(|opt| opt.map(|(_, spell)| spell))
                    .collect()
            };
            server.send_event(
                client,
                &utils::protocol::GameEvent {
                    kind: utils::protocol::GameEventKind::ShopOpened {
                        inventory: shop_inventory,
                    },
                },
            );
        }
        utils::protocol::ShopActionKind::Buy => {
            tracing::info!("Client {} a acheté un item du shop", client);

            let gold = res
                .get::<crate::session::PlayerRegistry>()
                .unwrap()
                .get_gold(client);
            let item = {
                let mut player_shop = res.get_mut::<PlayerShops>().unwrap();
                player_shop.buy(client, action.slot as usize, gold)
            };

            match item {
                Some((id, spell)) => {
                    tracing::info!("Client {} a acheté l'item du slot {}", client, action.slot);
                    let spell_register = res
                        .get::<crate::simulation::resources::spells::SpellRegister>()
                        .unwrap();
                    let spell_id = *spell_register.resolve_string(&id).unwrap();

                    // Soustraire l'or
                    res.get_mut::<crate::session::PlayerRegistry>()
                        .unwrap()
                        .sub_gold(client, spell.costs.gold);

                    // Assigner le sort au premier slot libre
                    let slot_idx = {
                        let registry = res.get::<crate::session::PlayerRegistry>().unwrap();
                        let player_entry = registry.get_entry(client).unwrap();
                        player_entry.spells.iter().position(|s| s.is_none())
                    };

                    if let Some(slot_idx) = slot_idx {
                        let slot = SpellSlot::from(slot_idx);
                        res.get_mut::<crate::session::PlayerRegistry>()
                            .unwrap()
                            .add_spell(client, spell_id, slot);

                        server.send_event(
                            client,
                            &utils::protocol::GameEvent {
                                kind: utils::protocol::GameEventKind::SpellAcquired {
                                    slot,
                                    config: utils::protocol::SpellClientConfig {
                                        targeting_kind: spell.targeting.kind,
                                        range: spell.targeting.range,
                                        aoe: spell.targeting.aoe,
                                    },
                                },
                            },
                        );
                    } else {
                        // TODO: gérer le cas où le joueur n'a pas de slot libre pour le sort acheté
                        tracing::warn!(
                            "Client {} n'a pas de slot libre pour le sort acheté",
                            client
                        );
                    }
                    server.send_event(
                        client,
                        &utils::protocol::GameEvent {
                            kind: utils::protocol::GameEventKind::ItemBought {
                                slot: action.slot as usize,
                            },
                        },
                    );
                }
                None => {
                    tracing::warn!(
                        "Client {} n'a pas pu acheter l'item du slot {}",
                        client,
                        action.slot
                    );
                    server.send_event(
                        client,
                        &utils::protocol::GameEvent {
                            kind: utils::protocol::GameEventKind::PurchaseFailed {
                                slot: action.slot as usize,
                            },
                        },
                    );
                }
            }
        }
        utils::protocol::ShopActionKind::Close => {
            tracing::info!("Client {} a fermé le shop", client);
        }
    }
}
