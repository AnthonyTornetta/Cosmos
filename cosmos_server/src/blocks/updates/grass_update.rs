use bevy::prelude::{in_state, App, EventReader, EventWriter, IntoSystemConfigs, Query, Res, Update};

use cosmos_core::{
    block::{block_update::BlockUpdate, Block},
    ecs::mut_events::MutEvent,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, Structure},
};

use crate::state::GameState;

fn monitor_grass_updated(
    mut structure_query: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut event_reader: EventReader<MutEvent<BlockUpdate>>,
    mut event_writer: EventWriter<BlockChangedEvent>,
) {
    for ev in event_reader.read() {
        let ev = ev.read();

        if ev.cancelled() {
            continue;
        }

        let Ok(mut structure) = structure_query.get_mut(ev.structure_entity()) else {
            continue;
        };

        let block = ev.block().block(&structure, &blocks);

        if block.unlocalized_name() == "cosmos:short_grass" {
            let block_up = ev.block().block_up(&structure);
            let down_coord = block_up.face_pointing_pos_y.inverse().to_direction_coordinates() + ev.block().coords();

            let Ok(down_coord) = BlockCoordinate::try_from(down_coord) else {
                structure.remove_block_at(ev.block().coords(), &blocks, Some(&mut event_writer));
                continue;
            };

            if !structure.block_at(down_coord, &blocks).is_full() {
                structure.remove_block_at(ev.block().coords(), &blocks, Some(&mut event_writer));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, monitor_grass_updated.run_if(in_state(GameState::Playing)));
}
