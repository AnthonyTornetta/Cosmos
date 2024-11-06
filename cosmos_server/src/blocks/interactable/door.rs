use bevy::{prelude::*, utils::hashbrown::HashSet};
use cosmos_core::{
    block::{
        block_direction::ALL_BLOCK_DIRECTIONS,
        block_events::{BlockEventsSet, BlockInteractEvent},
        specific_blocks::door::DoorData,
        Block,
    },
    events::block_events::BlockDataChangedEvent,
    netty::system_sets::NetworkingSystemsSet,
    prelude::{BlockCoordinate, Structure, StructureBlock},
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

        ev_writer.send(ToggleDoorEvent(s_block));
    }
}

fn toggle_doors(
    mut q_structure: Query<&mut Structure>,
    mut evr_door_toggle: EventReader<ToggleDoorEvent>,
    mut evw_block_data_changed: EventWriter<BlockDataChangedEvent>,
    blocks: Res<Registry<Block>>,
) {
    for ev in evr_door_toggle.read() {
        let Ok(mut structure) = q_structure.get_mut(ev.0.structure()) else {
            warn!("Not structure?");
            continue;
        };

        let Some(door) = blocks.from_id("cosmos:door") else {
            return;
        };
        let door_id = door.id();

        let open = structure.block_info_at(ev.0.coords()).is_open();

        let mut todo = HashSet::new();
        todo.insert(ev.0.coords());

        let mut done = HashSet::new();
        while !todo.is_empty() {
            let mut new_todo = HashSet::new();

            for c in &todo {
                done.insert(*c);
            }

            for coord in todo {
                if structure.block_id_at(coord) != door_id {
                    continue;
                }

                let mut block_info = structure.block_info_at(coord);

                if open {
                    block_info.set_closed();
                } else {
                    block_info.set_open();
                }

                structure.set_block_info_at(coord, block_info, &mut evw_block_data_changed);

                done.insert(coord);

                for dir in ALL_BLOCK_DIRECTIONS {
                    if let Ok(coord) = BlockCoordinate::try_from(dir.to_coordinates() + coord) {
                        if !done.contains(&coord) {
                            new_todo.insert(coord);
                        }
                    }
                }
            }

            todo = new_todo;
        }
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
