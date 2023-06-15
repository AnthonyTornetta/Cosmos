//! Creates a grass planet

use bevy::prelude::{
    App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemAppConfig,
    IntoSystemConfigs, OnEnter, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::{Block, BlockFace},
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, ChunkInitEvent, Structure},
};

use crate::GameState;

use super::{
    biosphere_generation::{
        generate_planet, notify_when_done_generating_terrain, BlockRanges,
        GenerateChunkFeaturesEvent,
    },
    register_biosphere, TBiosphere, TGenerateChunkEvent, TemperatureRange,
};

#[derive(Component, Debug, Default, Clone)]
/// Marks that this is for a grass biosphere
pub struct GrassBiosphereMarker;

/// Marks that a grass chunk needs generated
pub struct GrassChunkNeedsGeneratedEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for GrassChunkNeedsGeneratedEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self {
        Self {
            x,
            y,
            z,
            structure_entity,
        }
    }

    fn get_structure_entity(&self) -> Entity {
        self.structure_entity
    }

    fn get_chunk_coordinates(&self) -> (usize, usize, usize) {
        (self.x, self.y, self.z)
    }
}

#[derive(Default, Debug)]
/// Creates a grass planet
pub struct GrassBiosphere;

impl TBiosphere<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent> for GrassBiosphere {
    fn get_marker_component(&self) -> GrassBiosphereMarker {
        GrassBiosphereMarker {}
    }

    fn get_generate_chunk_event(
        &self,
        x: usize,
        y: usize,
        z: usize,
        structure_entity: Entity,
    ) -> GrassChunkNeedsGeneratedEvent {
        GrassChunkNeedsGeneratedEvent::new(x, y, z, structure_entity)
    }
}

fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
    commands.insert_resource(BlockRanges::<GrassBiosphereMarker>::new(vec![
        (
            block_registry
                .from_id("cosmos:stone")
                .expect("Block missing")
                .clone(),
            5,
        ),
        (
            block_registry
                .from_id("cosmos:dirt")
                .expect("Block missing")
                .clone(),
            1,
        ),
        (
            block_registry
                .from_id("cosmos:grass")
                .expect("Block missing")
                .clone(),
            0,
        ),
    ]));
}

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating.
pub fn generate_chunk_features(
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<GrassBiosphereMarker>>,
    mut event_writer: EventWriter<ChunkInitEvent>,
    mut structure_query: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
) {
    for ev in event_reader.iter() {
        if let Ok(mut structure) = structure_query.get_mut(ev.structure_entity) {
            let (cx, cy, cz) = ev.chunk_coords;
            let sx = cx * CHUNK_DIMENSIONS;
            let sy = cy * CHUNK_DIMENSIONS;
            let sz = cz * CHUNK_DIMENSIONS;
            // Add GenerateChunkFeaturesEvent to mod.rs register_biosphere.

            // [cx * CHUNK_DIMENSIONS, (cx + 1) * CHUNK_DIMENSIONS)
            // #[serde(skip)]

            // Generate chunk features.

            let air = blocks.from_id("cosmos:air").unwrap();
            let grass = blocks.from_id("cosmos:grass").unwrap();
            let log = blocks.from_id("cosmos:cherry_log").unwrap();
            let leaf = blocks.from_id("cosmos:cherry_leaf").unwrap();
            for x in 0..CHUNK_DIMENSIONS {
                for z in 0..CHUNK_DIMENSIONS {
                    let mut y: i32 = 31;
                    while y >= 0
                        && structure.block_at(sx + x, sy + y as usize, sz + z, &blocks) == air
                    {
                        y -= 1;
                    }

                    if y >= 0
                        && structure.block_at(sx + x, sy + y as usize, sz + z, &blocks) == grass
                    {
                        if rand::random::<f32>() > 0.99 {
                            structure.set_block_at(
                                sx + x,
                                sy + y as usize + 1,
                                sz + z,
                                log,
                                BlockFace::Top,
                                &blocks,
                                None,
                            );

                            structure.set_block_at(
                                sx + x,
                                sy + y as usize + 2,
                                sz + z,
                                leaf,
                                BlockFace::Top,
                                &blocks,
                                None,
                            );
                        }
                    }
                }
            }

            // let x = structure.blocks_width() / 2;
            // let y = structure.blocks_height() / 2 + 600;
            // let z = structure.blocks_length() / 2;
            // structure.set_block_at(
            //     x,
            //     y,
            //     z,
            //     &blocks.from_id("cosmos:grass").unwrap(),
            //     BlockFace::Top,
            //     &blocks,
            //     None,
            // );

            event_writer.send(ChunkInitEvent {
                structure_entity: ev.structure_entity,
                x: cx,
                y: cy,
                z: cz,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_grass",
        TemperatureRange::new(0.0, 1000000000.0),
    );

    app.add_systems(
        (
            generate_planet::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>,
            notify_when_done_generating_terrain::<GrassBiosphereMarker>,
            generate_chunk_features,
        )
            .in_set(OnUpdate(GameState::Playing)),
    );

    app.add_system(make_block_ranges.in_schedule(OnEnter(GameState::PostLoading)));
}
