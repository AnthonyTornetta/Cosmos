use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    inventory::{Inventory, held_item_slot::HeldItemSlot},
    item::usable::PlayerRequestUseHeldItemEvent,
    netty::{server::ServerLobby, sync::events::server_event::NettyEventReceived, system_sets::NetworkingSystemsSet},
    prelude::StructureBlock,
};

mod blueprint;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum UseItemSet {
    SendUseItemEvents,
}

#[derive(Event, Debug, Clone, PartialEq, Eq)]
pub struct UseHeldItemEvent {
    player: Entity,
    looking_at_block: Option<StructureBlock>,
    looking_at_any: Option<StructureBlock>,
    item: Option<u16>,
    held_slot: usize,
}

fn on_use_item(
    mut nevr_req_use_item: EventReader<NettyEventReceived<PlayerRequestUseHeldItemEvent>>,
    lobby: Res<ServerLobby>,
    mut evw_use_item: EventWriter<UseHeldItemEvent>,
    q_inventory: Query<(&Inventory, &HeldItemSlot), With<Player>>,
) {
    for ev in nevr_req_use_item.read() {
        let Some(player) = lobby.player_from_id(ev.client_id) else {
            continue;
        };

        let Ok((inventory, held_is)) = q_inventory.get(player) else {
            continue;
        };

        evw_use_item.write(UseHeldItemEvent {
            player,
            looking_at_block: ev.looking_at_block,
            looking_at_any: ev.looking_at_any,
            item: inventory.itemstack_at(held_is.slot() as usize).map(|x| x.item_id()),
            held_slot: held_is.slot() as usize,
        });
    }
}

pub(super) fn register(app: &mut App) {
    blueprint::register(app);

    app.configure_sets(
        FixedUpdate,
        UseItemSet::SendUseItemEvents.after(NetworkingSystemsSet::ReceiveMessages),
    );

    app.add_systems(FixedUpdate, on_use_item.in_set(UseItemSet::SendUseItemEvents))
        .add_event::<UseHeldItemEvent>();
}
