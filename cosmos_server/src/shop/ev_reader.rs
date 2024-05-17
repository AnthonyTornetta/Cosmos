use bevy::prelude::*;
use bevy_renet::renet::{ClientId, RenetServer};
use cosmos_core::{
    block::{block_events::BlockInteractEvent, Block},
    economy::Credits,
    entities::player::Player,
    inventory::{itemstack::ItemStackNeedsDataCreatedEvent, Inventory},
    item::Item,
    netty::{cosmos_encoder, server::ServerLobby, system_sets::NetworkingSystemsSet, NettyChannelClient, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
    shop::{
        netty::{ClientShopMessages, ServerShopMessages, ShopPurchaseError, ShopSellError},
        Shop,
    },
    structure::{coordinates::BlockCoordinate, Structure},
};

use super::prices::DefaultShopEntries;

use crate::GameState;

fn generate_fake_shop(default: &DefaultShopEntries) -> Shop {
    Shop {
        name: "Cool Shop".into(),
        contents: default.0.clone(),
    }
}

fn on_interact_with_shop(
    mut server: ResMut<RenetServer>,
    q_structure: Query<&Structure>,
    q_player: Query<&Player>,
    blocks: Res<Registry<Block>>,
    mut ev_reader: EventReader<BlockInteractEvent>,
    default_shop_entries: Res<DefaultShopEntries>,
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
            let fake_shop_data = generate_fake_shop(&default_shop_entries);

            server.send_message(
                player.id(),
                NettyChannelServer::Shop,
                cosmos_encoder::serialize(&ServerShopMessages::OpenShop {
                    shop_block: ev.structure_block.coords(),
                    structure_entity: ev.structure_entity,
                    shop_data: fake_shop_data,
                }),
            );
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

#[derive(Event)]
struct SellEvent {
    client_id: ClientId,
    shop_block: BlockCoordinate,
    structure_entity: Entity,
    item_id: u16,
    quantity: u32,
}

fn get_shop(
    _structure_entity: Entity,
    _shop_block: BlockCoordinate,
    default_shop_entries: &DefaultShopEntries,
    _q_structure: &Query<&Structure>,
    _q_shop_data: &mut Query<&mut Shop>,
) -> Option<Shop> {
    // let structure = q_structure.get(structure_entity).ok()?;

    // let block_data = structure.block_data(shop_block)?;

    // let mut shop = q_shop_data.get_mut(block_data).ok()?;

    Some(generate_fake_shop(default_shop_entries))
}

fn listen_sell_events(
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<SellEvent>,
    q_structure: Query<&Structure>,
    mut q_shop_data: Query<&mut Shop>,
    lobby: Res<ServerLobby>,
    mut q_player: Query<(&mut Inventory, &mut Credits)>,
    items: Res<Registry<Item>>,
    default_shop_entries: Res<DefaultShopEntries>,
) {
    for &SellEvent {
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

        let Some(item) = items.try_from_numeric_id(item_id) else {
            error!("Invalid item id: {item_id}");
            continue;
        };

        if !inventory.can_take_item(item, quantity as usize) {
            server.send_message(
                client_id,
                NettyChannelServer::Shop,
                cosmos_encoder::serialize(&ServerShopMessages::SellResult {
                    shop_block,
                    structure_entity,
                    details: Err(ShopSellError::NotEnoughItems),
                }),
            );
            continue;
        }

        let Some(mut shop) = get_shop(structure_entity, shop_block, &default_shop_entries, &q_structure, &mut q_shop_data) else {
            continue;
        };

        server.send_message(
            client_id,
            NettyChannelServer::Shop,
            cosmos_encoder::serialize(&ServerShopMessages::SellResult {
                shop_block,
                structure_entity,
                details: if let Err(error) = shop.sell(item_id, quantity, &mut credits) {
                    Err(error)
                } else {
                    inventory.take_item(item, quantity as usize);

                    Ok(shop.clone())
                },
            }),
        );
    }
}

fn listen_buy_events(
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<BuyEvent>,
    q_structure: Query<&Structure>,
    mut q_shop_data: Query<&mut Shop>,
    lobby: Res<ServerLobby>,
    mut q_player: Query<(&mut Inventory, &mut Credits)>,
    items: Res<Registry<Item>>,
    default_shop_entries: Res<DefaultShopEntries>,
    mut is_ev_writer: EventWriter<ItemStackNeedsDataCreatedEvent>,
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

        let Some(item) = items.try_from_numeric_id(item_id) else {
            error!("Invalid item id: {item_id}");
            continue;
        };

        if !inventory.can_insert(item, quantity as u16) {
            server.send_message(
                client_id,
                NettyChannelServer::Shop,
                cosmos_encoder::serialize(&ServerShopMessages::PurchaseResult {
                    shop_block,
                    structure_entity,
                    details: Err(ShopPurchaseError::NotEnoughInventorySpace),
                }),
            );
            continue;
        }

        let Some(mut shop) = get_shop(structure_entity, shop_block, &default_shop_entries, &q_structure, &mut q_shop_data) else {
            continue;
        };

        match shop.buy(item_id, quantity, &mut credits) {
            Ok(_) => {
                server.send_message(
                    client_id,
                    NettyChannelServer::Shop,
                    cosmos_encoder::serialize(&ServerShopMessages::PurchaseResult {
                        shop_block,
                        structure_entity,
                        details: Ok(shop.clone()),
                    }),
                );

                inventory.insert_item(item, quantity as u16, Some((player_ent, &mut is_ev_writer)));
            }
            Err(msg) => {
                server.send_message(
                    client_id,
                    NettyChannelServer::Shop,
                    cosmos_encoder::serialize(&ServerShopMessages::PurchaseResult {
                        shop_block,
                        structure_entity,
                        details: Err(msg),
                    }),
                );
            }
        }
    }
}

fn listen_client_shop_messages(
    mut ev_writer_buy: EventWriter<BuyEvent>,
    mut ev_writer_sell: EventWriter<SellEvent>,
    mut server: ResMut<RenetServer>,
) {
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
                    ev_writer_buy.send(BuyEvent {
                        client_id,
                        item_id,
                        quantity,
                        shop_block,
                        structure_entity,
                    });
                }
                ClientShopMessages::Sell {
                    shop_block,
                    structure_entity,
                    item_id,
                    quantity,
                } => {
                    ev_writer_sell.send(SellEvent {
                        client_id,
                        item_id,
                        quantity,
                        shop_block,
                        structure_entity,
                    });
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            on_interact_with_shop,
            listen_client_shop_messages,
            listen_buy_events,
            listen_sell_events,
        )
            .chain()
            .run_if(in_state(GameState::Playing))
            .after(NetworkingSystemsSet::ProcessReceivedMessages),
    )
    .add_event::<BuyEvent>()
    .add_event::<SellEvent>();
}
