use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    inventory::{Inventory, held_item_slot::HeldItemSlot},
    item::usable::{PlayerRequestUseHeldItemEvent, UseHeldItemEvent, UseItemSet},
    netty::{
        server::ServerLobby,
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
    },
};

mod blueprint;

fn on_use_item(
    mut nevr_req_use_item: EventReader<NettyEventReceived<PlayerRequestUseHeldItemEvent>>,
    lobby: Res<ServerLobby>,
    mut evw_use_item: EventWriter<UseHeldItemEvent>,
    mut nevw_use_item: NettyEventWriter<UseHeldItemEvent>,
    q_inventory: Query<(&Inventory, &HeldItemSlot), With<Player>>,
) {
    for n_ev in nevr_req_use_item.read() {
        let Some(player) = lobby.player_from_id(n_ev.client_id) else {
            continue;
        };

        let Ok((inventory, held_is)) = q_inventory.get(player) else {
            continue;
        };

        let ev = UseHeldItemEvent {
            player,
            looking_at_block: n_ev.looking_at_block,
            looking_at_any: n_ev.looking_at_any,
            item: inventory.itemstack_at(held_is.slot() as usize).map(|x| x.item_id()),
            held_slot: held_is.slot() as usize,
        };

        evw_use_item.write(ev.clone());

        nevw_use_item.write(ev, n_ev.client_id);
    }
}

pub(super) fn register(app: &mut App) {
    blueprint::register(app);

    app.add_systems(FixedUpdate, on_use_item.in_set(UseItemSet::SendUseItemEvents))
        .add_event::<UseHeldItemEvent>();
}
