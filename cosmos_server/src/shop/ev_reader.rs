use bevy::{
    app::{App, Update},
    ecs::{
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        schedule::IntoSystemConfigs,
        system::{Query, Res, ResMut},
    },
    log::{error, info},
};
use bevy_renet::renet::{ClientId, RenetServer};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
    economy::Credits,
    entities::player::Player,
    inventory::Inventory,
    item::Item,
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelClient, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
    shop::{
        netty::{ClientShopMessages, ServerShopMessages, ShopPurchaseError},
        Shop, ShopEntry,
    },
    structure::{coordinates::BlockCoordinate, Structure},
};

use crate::netty::network_helpers::ServerLobby;

fn generate_fake_shop(items: &Registry<Item>) -> Shop {
    Shop {
        name: "Fake Shop".into(),
        contents: vec![
            ShopEntry::Selling {
                item_id: items.from_id("cosmos:laser_cannon").expect("Missing laser_cannon").id(),
                max_quantity_selling: 1000,
                price_per: 100,
            },
            ShopEntry::Selling {
                item_id: items.from_id("cosmos:plasma_drill").expect("Missing plasma_drill").id(),
                max_quantity_selling: 1000,
                price_per: 100,
            },
            ShopEntry::Selling {
                item_id: items.from_id("cosmos:ship_hull_grey").expect("Missing ship_hull_grey").id(),
                max_quantity_selling: 1000,
                price_per: 100,
            },
            ShopEntry::Selling {
                item_id: items.from_id("cosmos:thruster").expect("Missing thruster").id(),
                max_quantity_selling: 1000,
                price_per: 100,
            },
            ShopEntry::Buying {
                item_id: items.from_id("cosmos:laser_cannon").expect("Missing laser_cannon").id(),
                max_quantity_buying: Some(2),
                price_per: 100,
            },
            ShopEntry::Buying {
                item_id: items.from_id("cosmos:plasma_drill").expect("Missing plasma_drill").id(),
                max_quantity_buying: Some(10000),
                price_per: 100,
            },
            ShopEntry::Buying {
                item_id: items.from_id("cosmos:ship_hull_grey").expect("Missing ship_hull_grey").id(),
                max_quantity_buying: None,
                price_per: 100,
            },
            ShopEntry::Buying {
                item_id: items.from_id("cosmos:thruster").expect("Missing thruster").id(),
                max_quantity_buying: None,
                price_per: 100,
            },
        ],
    }
}

fn on_interact_with_shop(
    mut server: ResMut<RenetServer>,
    q_structure: Query<&Structure>,
    q_player: Query<&Player>,
    blocks: Res<Registry<Block>>,
    mut ev_reader: EventReader<BlockInteractEvent>,
    items: Res<Registry<Item>>,
) {
    for ev in ev_reader.read() {
        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        let block = ev.structure_block.block(structure, &blocks);

        if block.unlocalized_name() == "cosmos:shop" {
            let fake_shop_data = generate_fake_shop(&items);

            server.send_message(
                player.id(),
                NettyChannelServer::Shop,
                cosmos_encoder::serialize(&ServerShopMessages::OpenShop {
                    shop_block: ev.structure_block.coords(),
                    structure_entity: ev.structure_entity,
                    shop_data: fake_shop_data,
                }),
            );
            info!("Interacted w/ shop");
        }
    }
}

#[derive(Event)]
struct BuyEvent {
    client_id: ClientId,
    shop_block: BlockCoordinate,
    structure_entity: Entity,
    item_id: u16,
    quantity: u32,
}

fn listen_buy_events(
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<BuyEvent>,
    _q_structure: Query<&Structure>,
    _q_shop_data: Query<&mut Shop>,
    lobby: Res<ServerLobby>,
    mut q_player: Query<(&mut Inventory, &mut Credits)>,
    items: Res<Registry<Item>>,
) {
    for &BuyEvent {
        client_id,
        shop_block,
        structure_entity,
        item_id,
        quantity,
    } in ev_reader.read()
    {
        let Some(player_ent) = lobby.player_from_id(client_id) else {
            error!("Bad player id: {client_id}");
            continue;
        };

        let Ok((mut inventory, mut credits)) = q_player.get_mut(player_ent) else {
            error!("No credits on player entity: {player_ent:?}");
            continue;
        };

        // let Ok(structure) = q_structure.get(structure_entity) else {
        //     continue;
        // };

        // let Some(block_data) = structure.block_data(shop_block) else {
        //     continue;
        // };

        // let Ok(mut shop) = q_shop_data.get_mut(block_data) else {
        //     continue;
        // };

        let mut shop = generate_fake_shop(&items);

        let Some(item) = items.try_from_numeric_id(item_id) else {
            error!("Invalid item id: {item_id}");
            continue;
        };

        if !inventory.can_insert(item, quantity as u16) {
            server.send_message(
                client_id,
                NettyChannelServer::Shop,
                cosmos_encoder::serialize(&ServerShopMessages::Purchase {
                    shop_block,
                    structure_entity,
                    details: Err(ShopPurchaseError::NotEnoughInventorySpace),
                }),
            );
            continue;
        }

        match shop.buy(item_id, quantity, &mut credits) {
            Ok(_) => {
                server.send_message(
                    client_id,
                    NettyChannelServer::Shop,
                    cosmos_encoder::serialize(&ServerShopMessages::Purchase {
                        shop_block,
                        structure_entity,
                        details: Ok(shop.clone()),
                    }),
                );

                inventory.insert(item, quantity as u16);
            }
            Err(msg) => {
                server.send_message(
                    client_id,
                    NettyChannelServer::Shop,
                    cosmos_encoder::serialize(&ServerShopMessages::Purchase {
                        shop_block,
                        structure_entity,
                        details: Err(msg),
                    }),
                );
            }
        }
    }
}

fn listen_client_shop_messages(mut ev_writer: EventWriter<BuyEvent>, mut server: ResMut<RenetServer>) {
    for client_id in server.clients_id() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::Shop) {
            let Ok(msg) = cosmos_encoder::deserialize::<ClientShopMessages>(&message) else {
                error!("Bad shop message from {client_id}");
                continue;
            };

            match msg {
                ClientShopMessages::Buy {
                    shop_block,
                    structure_entity,
                    item_id,
                    quantity,
                } => {
                    ev_writer.send(BuyEvent {
                        client_id,
                        item_id,
                        quantity,
                        shop_block,
                        structure_entity,
                    });
                }
                ClientShopMessages::Sell {
                    shop_block: _,
                    structure_entity: _,
                    item_id: _,
                    quantity: _,
                } => {
                    todo!();
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_interact_with_shop, listen_client_shop_messages, listen_buy_events)
            .chain()
            .after(NetworkingSystemsSet::FlushReceiveMessages),
    )
    .add_event::<BuyEvent>();
}