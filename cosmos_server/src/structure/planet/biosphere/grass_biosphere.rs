//! Creates a grass planet

use bevy::prelude::{
    App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemAppConfig,
    IntoSystemConfigs, OnEnter, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, ChunkInitEvent, Structure},
    utils::resource_wrapper::ResourceWrapper,
};
use noise::NoiseFn;

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

const DELTA: f64 = 0.1;
const FOREST: f64 = 0.1;

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating.
pub fn generate_chunk_features(
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<GrassBiosphereMarker>>,
    mut event_writer: EventWriter<ChunkInitEvent>,
    mut structure_query: Query<(&mut Structure, &Location)>,
    blocks: Res<Registry<Block>>,
    noise_generator: Res<ResourceWrapper<noise::OpenSimplex>>,
) {
    for ev in event_reader.iter() {
        if let Ok((mut structure, location)) = structure_query.get_mut(ev.structure_entity) {
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

            let structure_coords = location.absolute_coords_f64();

            let noise_y = structure.blocks_height();
            let mut noise_cache = [[0.0; CHUNK_DIMENSIONS + 2]; CHUNK_DIMENSIONS + 2];
            for (z, slice) in noise_cache.iter_mut().enumerate() {
                let bz = sz + z;
                for (x, noise) in slice.iter_mut().enumerate() {
                    *noise = noise_generator.get([
                        ((sx + x) as f64 - 1.0 + structure_coords.x) * DELTA,
                        (noise_y as f64 + structure_coords.y) * DELTA,
                        (bz as f64 - 1.0 + structure_coords.z) * DELTA,
                    ]);
                }
            }

            for z in 0..CHUNK_DIMENSIONS {
                let bz = sz + z;
                'next: for x in 0..CHUNK_DIMENSIONS {
                    let bx = sx + x;
                    let mut y: i32 = CHUNK_DIMENSIONS as i32 - 1;
                    while y >= 0 && structure.block_at(bx, sy + y as usize, bz, &blocks) == air {
                        y -= 1;
                    }

                    let noise = noise_cache[z + 1][x + 1];
                    if y >= 0
                        && structure.block_at(sx + x, sy + y as usize, sz + z, &blocks) == grass
                        && noise * noise > FOREST
                    {
                        for dz in 0..=2 {
                            for dx in 0..=2 {
                                if noise < noise_cache[z + dz][x + dx] {
                                    continue 'next;
                                }
                            }
                        }

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
