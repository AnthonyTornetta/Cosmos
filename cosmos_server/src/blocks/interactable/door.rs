use bevy::{platform::collections::HashSet, prelude::*};
use cosmos_core::{
    block::{
        Block,
        block_direction::ALL_BLOCK_DIRECTIONS,
        block_events::{BlockEventsSet, BlockInteractEvent},
    },
    events::block_events::BlockChangedEvent,
    prelude::{BlockCoordinate, Structure, StructureBlock},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
};

#[derive(Debug, Event)]
struct ToggleDoorEvent(StructureBlock);

fn handle_door_block_event(
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
        let un = block.unlocalized_name();
        if un != "cosmos:door" && un != "cosmos:door_open" {
            return;
        }

        ev_writer.write(ToggleDoorEvent(s_block));
    }
}

fn toggle_doors(
    mut q_structure: Query<&mut Structure>,
    mut evr_door_toggle: EventReader<ToggleDoorEvent>,
    mut evw_block_changed: EventWriter<BlockChangedEvent>,
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
        let Some(door_open) = blocks.from_id("cosmos:door_open") else {
            return;
        };

        let door_id = door.id();
        let door_open_id = door_open.id();

        let open = structure.block_id_at(ev.0.coords()) == door_open_id;
        let block = if open { door } else { door_open };

        let mut todo = HashSet::new();
        todo.insert(ev.0.coords());

        let mut done = HashSet::new();
        while !todo.is_empty() {
            let mut new_todo = HashSet::new();

            for c in &todo {
                done.insert(*c);
            }

            for coord in todo {
                let block_id_here = structure.block_id_at(coord);
                if block_id_here != door_id && block_id_here != door_open_id {
                    continue;
                }

                let block_info = structure.block_info_at(coord);

                structure.set_block_and_info_at(coord, block, block_info, &blocks, Some(&mut evw_block_changed));

                done.insert(coord);

                for dir in ALL_BLOCK_DIRECTIONS {
                    if let Ok(coord) = BlockCoordinate::try_from(dir.to_coordinates() + coord)
                        && !done.contains(&coord)
                    {
                        new_todo.insert(coord);
                    }
                }
            }

            todo = new_todo;
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            handle_door_block_event.in_set(BlockEventsSet::ProcessEvents),
            toggle_doors.in_set(BlockEventsSet::SendEventsForNextFrame),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .add_event::<ToggleDoorEvent>();
}
