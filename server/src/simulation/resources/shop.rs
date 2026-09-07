use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use utils::spell_types::{Spell, SpellId};

pub struct SpellPool {
    pub items: Vec<SpellId>,
}

pub struct PlayerShops {
    pub inventories: HashMap<u64, Vec<Option<SpellId>>>,
}

impl PlayerShops {
    pub fn new() -> Self {
        Self {
            inventories: HashMap::new(),
        }
    }

    pub fn generate(&mut self, player_id: u64, item_pool: &[SpellId]) -> Vec<Option<SpellId>> {
        let count = item_pool.len().min(3);
        let items: Vec<Option<SpellId>> = item_pool
            .sample(&mut rand::rng(), count)
            .copied()
            .map(Some)
            .collect();
        self.inventories.insert(player_id, items.clone());
        items
    }

    pub fn get(&self, player_id: u64, slot: usize) -> Option<SpellId> {
        self.inventories
            .get(&player_id)?
            .get(slot)
            .copied()
            .flatten()
    }

    pub fn remove(&mut self, player_id: u64, slot: usize) -> Option<SpellId> {
        self.inventories.get_mut(&player_id)?.get_mut(slot)?.take()
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

            let offered_ids = {
                let item_pool = res.get::<SpellPool>().unwrap().items.clone();
                let mut player_shops = res.get_mut::<PlayerShops>().unwrap();
                player_shops.generate(client, &item_pool)
            };
            let shop_inventory: Vec<Option<Spell>> = {
                let spell_register = res
                    .get::<crate::simulation::resources::spells::SpellRegister>()
                    .unwrap();
                offered_ids
                    .into_iter()
                    .map(|id| id.and_then(|id| spell_register.get_spell(id).cloned()))
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

            // Validate capacity before mutating the shop inventory or the player's gold.
            let Some(spell_slot) = res
                .get::<crate::session::PlayerRegistry>()
                .unwrap()
                .first_free_spell_slot(client)
            else {
                // TODO(spell-swap): lorsqu'aucun slot n'est libre, demander au joueur quel sort
                // remplacer. L'achat devra rester atomique : ne retirer l'or et l'offre du shop
                // qu'après confirmation du slot de destination, puis notifier le client du nouveau
                // contenu du loadout.
                tracing::warn!("Client {} n'a pas de slot de sort libre", client);
                server.send_event(
                    client,
                    &utils::protocol::GameEvent {
                        kind: utils::protocol::GameEventKind::PurchaseFailed {
                            slot: action.slot as usize,
                        },
                    },
                );
                return;
            };

            let spell_id = {
                let player_shop = res.get::<PlayerShops>().unwrap();
                player_shop.get(client, action.slot as usize)
            };

            match spell_id {
                Some(spell_id) => {
                    tracing::info!("Client {} a acheté l'item du slot {}", client, action.slot);
                    let spell = res
                        .get::<crate::simulation::resources::spells::SpellRegister>()
                        .unwrap()
                        .get_spell(spell_id)
                        .cloned()
                        .expect("Le pool ne doit contenir que des sorts enregistrés");

                    let gold = res
                        .get::<crate::session::PlayerRegistry>()
                        .unwrap()
                        .get_gold(client);
                    if gold < spell.purchase_cost.gold {
                        server.send_event(
                            client,
                            &utils::protocol::GameEvent {
                                kind: utils::protocol::GameEventKind::PurchaseFailed {
                                    slot: action.slot as usize,
                                },
                            },
                        );
                        return;
                    }

                    let equipped = res
                        .get_mut::<crate::session::PlayerRegistry>()
                        .unwrap()
                        .add_spell(client, spell_id, spell_slot, spell.cast_cost.charges);

                    if equipped {
                        res.get_mut::<crate::session::PlayerRegistry>()
                            .unwrap()
                            .sub_gold(client, spell.purchase_cost.gold);
                        res.get_mut::<PlayerShops>()
                            .unwrap()
                            .remove(client, action.slot as usize);
                        server.send_event(
                            client,
                            &utils::protocol::GameEvent {
                                kind: utils::protocol::GameEventKind::SpellAcquired {
                                    slot: spell_slot,
                                    config: utils::protocol::SpellClientConfig {
                                        targeting_kind: spell.targeting.kind,
                                        range: spell.targeting.range,
                                        aoe: spell.targeting.aoe,
                                        cooldown: spell.cast_cost.cooldown,
                                    },
                                },
                            },
                        );
                    } else {
                        tracing::warn!(
                            "Client {} n'a pas de slot libre pour le sort acheté",
                            client
                        );
                        server.send_event(
                            client,
                            &utils::protocol::GameEvent {
                                kind: utils::protocol::GameEventKind::PurchaseFailed {
                                    slot: action.slot as usize,
                                },
                            },
                        );
                        return;
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
