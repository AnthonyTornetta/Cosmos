use bevy::prelude::{
    in_state, App, Commands, Entity, EventWriter, IntoSystemConfig, Query, Res, With,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{
        asteroid::loading::AsteroidNeedsCreated, loading::ChunksNeedLoaded,
        structure_iterator::ChunkIteratorResult, ChunkInitEvent, Structure,
    },
    utils::resource_wrapper::ResourceWrapper,
};
use noise::NoiseFn;

use crate::state::GameState;

fn generate_asteroid(
    mut query: Query<(Entity, &mut Structure, &Location), With<AsteroidNeedsCreated>>,
    noise: Res<ResourceWrapper<noise::OpenSimplex>>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    mut chunk_init_event_writer: EventWriter<ChunkInitEvent>,
) {
    for (entity, mut structure, loc) in query.iter_mut() {
        let (cx, cy, cz) = (loc.local.x as f64, loc.local.y as f64, loc.local.z as f64);

        let distance_threshold =
            structure.blocks_length() as f64 / 2.0 * (noise.get([cx, cy, cz]) + 1.0).min(25.0);

        let stone = blocks.from_id("cosmos:stone").unwrap();

        for z in 0..structure.blocks_length() {
            for y in 0..structure.blocks_height() {
                for x in 0..structure.blocks_width() {
                    let block_here = distance_threshold
                        / (x as f64 - structure.blocks_width() as f64 / 2.0)
                            .max(y as f64 - structure.blocks_height() as f64 / 2.0)
                            .max(z as f64 - structure.blocks_length() as f64 / 2.0)
                            .max(1.0);

                    let noise_here = noise
                        .get([
                            x as f64 * 0.01 + cx,
                            y as f64 * 0.01 + cy,
                            y as f64 * 0.01 + cy,
                        ])
                        .abs()
                        * block_here;

                    if noise_here > 0.5 {
                        structure.set_block_at(x, y, z, stone, BlockFace::Top, &blocks, None)
                    }
                }
            }
        }

        let itr = structure.all_chunks_iter(false);

        commands
            .entity(entity)
            .remove::<AsteroidNeedsCreated>()
            .insert(ChunksNeedLoaded {
                amount_needed: itr.len(),
            });

        for res in itr {
            // This will always be true because include_empty is false
            if let ChunkIteratorResult::FilledChunk {
                position: (x, y, z),
                chunk: _,
            } = res
            {
                chunk_init_event_writer.send(ChunkInitEvent {
                    structure_entity: entity,
                    x,
                    y,
                    z,
                });
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(generate_asteroid.run_if(in_state(GameState::Playing)));
}
