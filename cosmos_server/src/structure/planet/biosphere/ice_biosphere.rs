//! Creates a ice planet

use bevy::prelude::{
    App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemAppConfig, IntoSystemConfigs, OnEnter, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::{self, Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, coordinates::BlockCoordinate, planet::Planet, rotate, ChunkInitEvent, Structure},
};

use crate::{init::init_world::ServerSeed, GameState};

use super::{
    biosphere_generation::{
        generate_planet, notify_when_done_generating_terrain, BlockLayers, DefaultBiosphereGenerationStrategy, GenerateChunkFeaturesEvent,
    },
    register_biosphere, TBiosphere, TGenerateChunkEvent, TemperatureRange,
};

#[derive(Component, Debug, Default, Clone)]
/// Marks that this is for a grass biosphere
pub struct IceBiosphereMarker;

/// Marks that an ice chunk needs generated
pub struct IceChunkNeedsGeneratedEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for IceChunkNeedsGeneratedEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self {
        Self { x, y, z, structure_entity }
    }

    fn get_structure_entity(&self) -> Entity {
        self.structure_entity
    }

    fn get_chunk_coordinates(&self) -> (usize, usize, usize) {
        (self.x, self.y, self.z)
    }
}

#[derive(Default, Debug)]
/// Creates a ice planet
pub struct IceBiosphere;

impl TBiosphere<IceBiosphereMarker, IceChunkNeedsGeneratedEvent> for IceBiosphere {
    fn get_marker_component(&self) -> IceBiosphereMarker {
        IceBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, block_coords: BlockCoordinate, structure_entity: Entity) -> IceChunkNeedsGeneratedEvent {
        IceChunkNeedsGeneratedEvent::new(block_coords, structure_entity)
    }
}

fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
    commands.insert_resource(
        BlockLayers::<IceBiosphereMarker>::default()
            .add_noise_layer("cosmos:ice", &block_registry, 160, 0.01, 4.0, 1)
            .expect("Ice missing")
            .add_fixed_layer("cosmos:water", &block_registry, 4)
            .expect("Water missing")
            .add_fixed_layer("cosmos:stone", &block_registry, 296)
            .expect("Stone missing"),
    );
}

// Fills the chunk at the given coordinates with spikes
fn generate_spikes(
    (cx, cy, cz): (usize, usize, usize),
    structure: &mut Structure,
    location: &Location,
    block_event_writer: &mut EventWriter<BlockChangedEvent>,
    blocks: &Registry<Block>,
    seed: ServerSeed,
) {
    let (sx, sy, sz) = (cx * CHUNK_DIMENSIONS, cy * CHUNK_DIMENSIONS, cz * CHUNK_DIMENSIONS);
    let s_dimension = structure.blocks_height();

    let molten_stone = blocks.from_id("cosmos:molten_stone").expect("Missing molten_stone");

    let structure_coords = location.absolute_coords_f64();

    let faces = Planet::chunk_planet_faces((sx, sy, sz), s_dimension);
    for block_up in faces.iter() {
        // Getting the noise value for every block in the chunk, to find where to put trees.
        let noise_height = match block_up {
            BlockFace::Front | BlockFace::Top | BlockFace::Right => structure.blocks_height(),
            _ => 0,
        };

        for z in 0..CHUNK_DIMENSIONS {
            for x in 0..CHUNK_DIMENSIONS {
                let (nx, ny, nz) = match block_up {
                    BlockFace::Front | BlockFace::Back => ((sx + x) as f64, (sy + z) as f64, noise_height as f64),
                    BlockFace::Top | BlockFace::Bottom => ((sx + x) as f64, noise_height as f64, (sz + z) as f64),
                    BlockFace::Right | BlockFace::Left => (noise_height as f64, (sy + x) as f64, (sz + z) as f64),
                };

                let rng = seed
                    .chaos_hash(nx + structure_coords.x, ny + structure_coords.y, nz + structure_coords.z)
                    .abs()
                    % 20;

                if rng == 0 {
                    let rng = seed
                        .chaos_hash(
                            2000.0 + nx + structure_coords.x,
                            2000.0 + ny + structure_coords.y,
                            2000.0 + nz + structure_coords.z,
                        )
                        .abs()
                        % 4;

                    let (bx, by, bz) = match block_up {
                        BlockFace::Front | BlockFace::Back => (sx + x, sy + z, sz),
                        BlockFace::Top | BlockFace::Bottom => (sx + x, sy, sz + z),
                        BlockFace::Right | BlockFace::Left => (sx, sy + x, sz + z),
                    };

                    let s_dimensions = (s_dimension, s_dimension, s_dimension);

                    if let Ok(start_checking) = rotate((bx, by, bz), (0, CHUNK_DIMENSIONS as i32 - 1, 0), s_dimensions, block_up) {
                        'spike_placement: for dy_down in 0..CHUNK_DIMENSIONS {
                            if let Ok(rotated) = rotate(start_checking, (0, -(dy_down as i32), 0), s_dimensions, block_up) {
                                if structure.block_at_tuple(rotated, blocks) == molten_stone {
                                    for dy in 1..=rng {
                                        if let Ok(rel_pos) =
                                            rotate(start_checking, (0, dy as i32 - dy_down as i32, 0), s_dimensions, block_up)
                                        {
                                            structure.set_block_at_tuple(rel_pos, molten_stone, block_up, blocks, Some(block_event_writer));
                                        }
                                    }
                                    break 'spike_placement;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating, makes trees.
pub fn generate_chunk_features(
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<IceBiosphereMarker>>,
    mut init_event_writer: EventWriter<ChunkInitEvent>,
    mut block_event_writer: EventWriter<BlockChangedEvent>,
    mut structure_query: Query<(&mut Structure, &Location)>,
    blocks: Res<Registry<Block>>,
    seed: Res<ServerSeed>,
) {
    for ev in event_reader.iter() {
        if let Ok((mut structure, location)) = structure_query.get_mut(ev.structure_entity) {
            let chunk_coords = ev.chunk_coords;

            generate_spikes((cx, cy, cz), &mut structure, location, &mut block_event_writer, &blocks, *seed);

            init_event_writer.send(ChunkInitEvent {
                structure_entity: ev.structure_entity,
                coords: chunk_coords,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<IceBiosphereMarker, IceChunkNeedsGeneratedEvent>(app, "cosmos:biosphere_ice", TemperatureRange::new(0.0, 250.0));

    app.add_systems(
        (
            generate_planet::<IceBiosphereMarker, IceChunkNeedsGeneratedEvent, DefaultBiosphereGenerationStrategy>,
            notify_when_done_generating_terrain::<IceBiosphereMarker>,
            generate_chunk_features,
        )
            .in_set(OnUpdate(GameState::Playing)),
    );

    app.add_system(make_block_ranges.in_schedule(OnEnter(GameState::PostLoading)));
}
