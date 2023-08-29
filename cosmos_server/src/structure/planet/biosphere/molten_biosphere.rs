//! Creates a molten planet

use bevy::prelude::{
    in_state, App, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, Update,
};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::CHUNK_DIMENSIONS,
        coordinates::{BlockCoordinate, ChunkCoordinate, UnboundBlockCoordinate, UnboundCoordinateType},
        planet::Planet,
        rotate, ChunkInitEvent, Structure,
    },
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
pub struct MoltenBiosphereMarker;

/// Marks that a grass chunk needs generated
#[derive(Debug, Event)]
pub struct MoltenChunkNeedsGeneratedEvent {
    coords: ChunkCoordinate,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for MoltenChunkNeedsGeneratedEvent {
    fn new(coords: ChunkCoordinate, structure_entity: Entity) -> Self {
        Self { coords, structure_entity }
    }

    fn get_structure_entity(&self) -> Entity {
        self.structure_entity
    }

    fn get_chunk_coordinates(&self) -> ChunkCoordinate {
        self.coords
    }
}

#[derive(Default, Debug)]
/// Creates a molten planet
pub struct MoltenBiosphere;

impl TBiosphere<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedEvent> for MoltenBiosphere {
    fn get_marker_component(&self) -> MoltenBiosphereMarker {
        MoltenBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, coords: ChunkCoordinate, structure_entity: Entity) -> MoltenChunkNeedsGeneratedEvent {
        MoltenChunkNeedsGeneratedEvent::new(coords, structure_entity)
    }
}

fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
    commands.insert_resource(
        BlockLayers::<MoltenBiosphereMarker>::default()
            .with_sea_level_block("cosmos:cheese", &block_registry, 620)
            .expect("Cheese missing!")
            .add_noise_layer("cosmos:molten_stone", &block_registry, 160, 0.10, 7.0, 9)
            .expect("Molten Stone missing"),
    );
}

// Fills the chunk at the given coordinates with spikes
fn generate_spikes(
    coords: ChunkCoordinate,
    structure: &mut Structure,
    location: &Location,
    block_event_writer: &mut EventWriter<BlockChangedEvent>,
    blocks: &Registry<Block>,
    seed: ServerSeed,
) {
    let sc = coords.first_structure_block();

    let Structure::Dynamic(planet) = structure else {
        panic!("A planet must be dynamic!");
    };

    let s_dimension = planet.block_dimensions();
    let s_dimensions = structure.block_dimensions();
    let molten_stone = blocks.from_id("cosmos:molten_stone").expect("Missing molten_stone");

    let structure_coords = location.absolute_coords_f64();

    let faces = Planet::chunk_planet_faces(sc, s_dimension);
    for block_up in faces.iter() {
        // Getting the noise value for every block in the chunk, to find where to put spikes.
        let noise_height = match block_up {
            BlockFace::Front | BlockFace::Top | BlockFace::Right => s_dimension,
            _ => 0,
        };

        for z in 0..CHUNK_DIMENSIONS {
            for x in 0..CHUNK_DIMENSIONS {
                let (nx, ny, nz) = match block_up {
                    BlockFace::Front | BlockFace::Back => ((sc.x + x) as f64, (sc.y + z) as f64, noise_height as f64),
                    BlockFace::Top | BlockFace::Bottom => ((sc.x + x) as f64, noise_height as f64, (sc.z + z) as f64),
                    BlockFace::Right | BlockFace::Left => (noise_height as f64, (sc.y + x) as f64, (sc.z + z) as f64),
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

                    let coords: BlockCoordinate = match block_up {
                        BlockFace::Front | BlockFace::Back => (sc.x + x, sc.y + z, sc.z),
                        BlockFace::Top | BlockFace::Bottom => (sc.x + x, sc.y, sc.z + z),
                        BlockFace::Right | BlockFace::Left => (sc.x, sc.y + x, sc.z + z),
                    }
                    .into();

                    if let Ok(start_checking) = rotate(
                        coords,
                        UnboundBlockCoordinate::new(0, CHUNK_DIMENSIONS as UnboundCoordinateType - 1, 0),
                        s_dimensions,
                        block_up,
                    ) {
                        'spike_placement: for dy_down in 0..CHUNK_DIMENSIONS as UnboundCoordinateType {
                            if let Ok(rotated) = rotate(start_checking, UnboundBlockCoordinate::new(0, -dy_down, 0), s_dimensions, block_up)
                            {
                                if structure.block_at(rotated, blocks) == molten_stone {
                                    for dy in 1..=rng {
                                        if let Ok(rel_pos) = rotate(
                                            start_checking,
                                            UnboundBlockCoordinate::new(0, dy as UnboundCoordinateType - dy_down, 0),
                                            s_dimensions,
                                            block_up,
                                        ) {
                                            structure.set_block_at(rel_pos, molten_stone, block_up, blocks, Some(block_event_writer));
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
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<MoltenBiosphereMarker>>,
    mut init_event_writer: EventWriter<ChunkInitEvent>,
    mut block_event_writer: EventWriter<BlockChangedEvent>,
    mut structure_query: Query<(&mut Structure, &Location)>,
    blocks: Res<Registry<Block>>,
    seed: Res<ServerSeed>,
) {
    for ev in event_reader.iter() {
        if let Ok((mut structure, location)) = structure_query.get_mut(ev.structure_entity) {
            let coords = ev.chunk_coords;

            generate_spikes(coords, &mut structure, location, &mut block_event_writer, &blocks, *seed);

            init_event_writer.send(ChunkInitEvent {
                structure_entity: ev.structure_entity,
                coords,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedEvent, DefaultBiosphereGenerationStrategy>(
        app,
        "cosmos:biosphere_molten",
        TemperatureRange::new(0.0, 0.0),
    );

    app.add_systems(Update, generate_chunk_features.run_if(in_state(GameState::Playing)))
        .add_systems(OnEnter(GameState::PostLoading), make_block_ranges);
}
