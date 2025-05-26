use bevy::prelude::*;
use cosmos_core::{
    creative::{CreativeTrashHeldItem, GrabCreativeItemEvent},
    entities::player::creative::Creative,
    inventory::{
        HeldItemStack,
        itemstack::{ItemShouldHaveData, ItemStack},
        netty::ServerInventoryMessages,
    },
    item::Item,
    netty::{
        NettyChannelServer, cosmos_encoder, server::ServerLobby, sync::events::server_event::NettyEventReceived,
        system_sets::NetworkingSystemsSet,
    },
    registry::Registry,
};
use renet::RenetServer;

fn on_trash_item_creative(
    q_creative: Query<(), With<Creative>>,
    lobby: Res<ServerLobby>,
    mut q_holding: Query<&mut HeldItemStack>,
    mut nevr_grab_item: EventReader<NettyEventReceived<CreativeTrashHeldItem>>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
) {
    for ev in nevr_grab_item.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        if !q_creative.contains(player) {
            continue;
        };

        if let Ok(mut held_is) = q_holding.get_mut(player) {
            held_is.0.remove(&mut commands);
        }

        server.send_message(
            ev.client_id,
            NettyChannelServer::Inventory,
            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack { itemstack: None }),
        );
    }
}

fn on_grab_creative_item(
    q_creative: Query<(), With<Creative>>,
    lobby: Res<ServerLobby>,
    mut q_holding: Query<&mut HeldItemStack>,
    mut nevr_grab_item: EventReader<NettyEventReceived<GrabCreativeItemEvent>>,
    items: Res<Registry<Item>>,
    needs_data: Res<ItemShouldHaveData>,
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
) {
    for ev in nevr_grab_item.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        if !q_creative.contains(player) {
            continue;
        };

        let Some(item) = items.try_from_numeric_id(ev.item_id) else {
            continue;
        };

        if item.category().is_none() {
            // You can only get items that have a category
            continue;
        };

        if let Ok(mut held_is) = q_holding.get_mut(player) {
            held_is.0.remove(&mut commands);
        }

        let is = ItemStack::with_quantity(
            item,
            ev.quantity.min(item.max_stack_size()),
            // !! This will probably not work well for items with actual data.
            // TODO: Think about this later
            (player, u32::MAX),
            &mut commands,
            &needs_data,
        );

        let held_is = HeldItemStack(is).clone();

        commands.entity(player).insert(held_is.clone());

        server.send_message(
            ev.client_id,
            NettyChannelServer::Inventory,
            cosmos_encoder::serialize(&ServerInventoryMessages::HeldItemstack {
                itemstack: Some(held_is.clone()),
            }),
        );
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (on_trash_item_creative, on_grab_creative_item)
            .in_set(NetworkingSystemsSet::Between)
            .chain(),
    );
}
