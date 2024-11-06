use bevy::prelude::*;
use cosmos_core::{
    block::{
        block_events::{BlockEventsSet, BlockInteractEvent},
        specific_blocks::door::DoorData,
        Block,
    },
    events::block_events::BlockDataChangedEvent,
    netty::system_sets::NetworkingSystemsSet,
    prelude::{Structure, StructureBlock},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
};

#[derive(Debug, Event)]
struct ToggleDoorEvent(StructureBlock);

fn grav_well_handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    mut ev_writer: EventWriter<ToggleDoorEvent>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(structure) = q_structure.get(s_block.structure()) else {
            warn!("{s_block:?}");
            continue;
        };

        let block = structure.block_at(s_block.coords(), &blocks);

        if block.unlocalized_name() != "cosmos:door" {
            return;
        }

        println!("Toggle door event!");
        ev_writer.send(ToggleDoorEvent(s_block));
    }
}

fn toggle_doors(
    mut q_structure: Query<&mut Structure>,
    mut evr_door_toggle: EventReader<ToggleDoorEvent>,
    mut evw_block_data_changed: EventWriter<BlockDataChangedEvent>,
) {
    for ev in evr_door_toggle.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.0.structure()) else {
            warn!("Not structure?");
            continue;
        };

        let open = structure.block_info_at(ev.0.coords()).is_open();
        println!("Toggle door event recv.");

        // TODO: Iterate over every door adjacent
        let mut block_info = structure.block_info_at(ev.0.coords());
        if open {
            println!("Setting closed");
            block_info.set_closed();
        } else {
            println!("Setting opened");
            block_info.set_open();
        }
        println!("New info: {block_info:?}");
        structure.set_block_info_at(ev.0.coords(), block_info, &mut evw_block_data_changed);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (
            grav_well_handle_block_event.in_set(BlockEventsSet::ProcessEvents),
            toggle_doors.in_set(BlockEventsSet::SendEventsForNextFrame),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .add_event::<ToggleDoorEvent>();
}
