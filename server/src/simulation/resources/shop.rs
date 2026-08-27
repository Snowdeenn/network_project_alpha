use rand::prelude::IndexedRandom;
use std::collections::HashMap;
use utils::protocol::ShopItem;

#[derive(Debug)]
pub struct ItemPool {
    pub items: Vec<Option<ShopItem>>,
}

pub struct PlayerShops {
    pub inventories: HashMap<u64, Vec<Option<ShopItem>>>,
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
        item_pool: &[Option<ShopItem>],
    ) -> Vec<Option<ShopItem>> {
        let count = item_pool.len().min(3);
        let items: Vec<Option<ShopItem>> =
            item_pool.sample(&mut rand::rng(), count).cloned().collect();
        self.inventories.insert(player_id, items.clone());
        items
    }

    pub fn buy(&mut self, player_id: u64, slot: usize, gold_avaible: u32) -> Option<ShopItem> {
        let inventory = self.inventories.get_mut(&player_id)?;
        let item = inventory.get(slot)?.as_ref()?;
        if item.price <= gold_avaible {
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
            println!("Client {} a ouvert le shop", client);

            let shop_inventory = {
                let item_pool = res.get::<ItemPool>().unwrap();
                let mut player_shops = res.get_mut::<PlayerShops>().unwrap();
                player_shops.generate(client, &item_pool.items)
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
            println!("Client {} a acheté un item du shop", client);

            let gold = res.get::<crate::session::PlayerRegistry>().unwrap().get_gold(client);
            let item = {
                let mut player_shop = res.get_mut::<PlayerShops>().unwrap();
                player_shop.buy(client, action.slot as usize, gold)
            };

            match item {
                Some(item) => {
                    println!("Client {} a acheté l'item du slot {}", client, action.slot);
                    res.get_mut::<crate::session::PlayerRegistry>()
                        .unwrap()
                        .sub_gold(client, item.price);
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
                    println!(
                        "Client {} n'a pas pu acheter l'item du slot {}",
                        client, action.slot
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
            println!("Client {} a fermé le shop", client);
        }
    }
}
