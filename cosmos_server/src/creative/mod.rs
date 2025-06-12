//! Server creative logic

use bevy::prelude::*;
use cosmos_core::{
    creative::{CreativeTrashHeldItem, GrabCreativeItemEvent},
    entities::player::creative::Creative,
    inventory::{HeldItemStack, Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    netty::{server::ServerLobby, sync::events::server_event::NettyEventReceived, system_sets::NetworkingSystemsSet},
    registry::Registry,
};

fn on_trash_item_creative(
    q_creative: Query<(), With<Creative>>,
    lobby: Res<ServerLobby>,
    mut nevr_grab_item: EventReader<NettyEventReceived<CreativeTrashHeldItem>>,
    mut commands: Commands,
    q_children: Query<&Children>,
    mut q_held_item: Query<&mut Inventory, With<HeldItemStack>>,
) {
    for ev in nevr_grab_item.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        if !q_creative.contains(player) {
            continue;
        };

        if let Some(mut inv) = HeldItemStack::get_held_is_inventory_mut(player, &q_children, &mut q_held_item)
            && let Some(mut is) = inv.remove_itemstack_at(0)
        {
            is.remove(&mut commands);
        }
    }
}

fn on_grab_creative_item(
    q_creative: Query<(), With<Creative>>,
    lobby: Res<ServerLobby>,
    mut nevr_grab_item: EventReader<NettyEventReceived<GrabCreativeItemEvent>>,
    items: Res<Registry<Item>>,
    needs_data: Res<ItemShouldHaveData>,
    mut commands: Commands,
    q_children: Query<&Children>,
    mut q_held_item: Query<&mut Inventory, With<HeldItemStack>>,
) {
    for ev in nevr_grab_item.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            info!("Not player");
            continue;
        };

        if !q_creative.contains(player) {
            info!("Not creative");
            continue;
        };

        let Some(item) = items.try_from_numeric_id(ev.item_id) else {
            info!("Bad item");
            continue;
        };

        if item.category().is_none() {
            info!("Bad category");
            // You can only get items that have a category
            continue;
        };

        if let Some(mut inv) = HeldItemStack::get_held_is_inventory_mut(player, &q_children, &mut q_held_item) {
            inv.take_itemstack_at(0, &mut commands);

            inv.insert_item_at(0, item, ev.quantity.min(item.max_stack_size()), &mut commands, &needs_data);
        } else {
            error!("Missing held item inventory!");
        }
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
