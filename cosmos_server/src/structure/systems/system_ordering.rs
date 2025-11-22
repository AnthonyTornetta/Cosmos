use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::{server::ServerLobby, sync::events::server_event::NettyMessageReceived},
    prelude::StructureSystems,
    structure::{
        ship::pilot::Pilot,
        systems::{ChangeSystemSlot, StructureSystemOrdering},
    },
};

fn on_change_system_slot(
    mut nevr_change_system_slot: MessageReader<NettyMessageReceived<ChangeSystemSlot>>,
    mut q_system_order: Query<(&mut StructureSystemOrdering, &StructureSystems, &Pilot)>,
    lobby: Res<ServerLobby>,
) {
    for ev in nevr_change_system_slot.read() {
        let Some(player_ent) = lobby.player_from_id(ev.client_id) else {
            error!("Bad player ({})!", ev.client_id);
            continue;
        };

        let Ok((mut sys_ordering, systems, pilot)) = q_system_order.get_mut(ev.structure) else {
            error!("Bad ship - {:?}!", ev.structure);
            continue;
        };

        if pilot.entity != player_ent {
            error!("Cannot set system ordering of ship you don't pilot ({})!", ev.client_id);
            continue;
        }

        if ev.slot >= 9 {
            error!("Invalid slot - {}!", ev.slot);
            continue;
        }

        if let Some(sys) = ev.system_id {
            if systems.get_system_entity(sys).is_none() {
                error!("Invalid system id - {sys:?}");
                continue;
            }
            if let Some(previous_ordering) = sys_ordering.ordering_for(sys) {
                sys_ordering.clear_slot(previous_ordering);
            }
            sys_ordering.set_slot(ev.slot, sys);
        } else {
            sys_ordering.clear_slot(ev.slot);
        }
    }
}
pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_change_system_slot.in_set(FixedUpdateSet::Main));
}
