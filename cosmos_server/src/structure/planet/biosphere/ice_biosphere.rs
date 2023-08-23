//! Creates a ice planet

use bevy::prelude::{
    in_state, App, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, Resource, Update,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType},
        ChunkInitEvent, Structure,
    },
};

use crate::GameState;

use super::{
    biosphere_generation::{
        generate_planet, notify_when_done_generating_terrain, BiosphereGenerationStrategy, BlockLayers, GenerateChunkFeaturesEvent,
    },
    register_biosphere, TBiosphere, TGenerateChunkEvent, TemperatureRange,
};

#[derive(Component, Debug, Default, Clone)]
/// Marks that this is for a grass biosphere
pub struct IceBiosphereMarker;

/// Marks that an ice chunk needs generated
#[derive(Event, Debug)]
pub struct IceChunkNeedsGeneratedEvent {
    chunk_coords: ChunkCoordinate,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for IceChunkNeedsGeneratedEvent {
    fn new(chunk_coords: ChunkCoordinate, structure_entity: Entity) -> Self {
        Self {
            chunk_coords,
            structure_entity,
        }
    }

    fn get_structure_entity(&self) -> Entity {
        self.structure_entity
    }

    fn get_chunk_coordinates(&self) -> ChunkCoordinate {
        self.chunk_coords
    }
}

#[derive(Default, Debug)]
/// Creates a ice planet
pub struct IceBiosphere;

impl TBiosphere<IceBiosphereMarker, IceChunkNeedsGeneratedEvent> for IceBiosphere {
    fn get_marker_component(&self) -> IceBiosphereMarker {
        IceBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, chunk_coords: ChunkCoordinate, structure_entity: Entity) -> IceChunkNeedsGeneratedEvent {
        IceChunkNeedsGeneratedEvent::new(chunk_coords, structure_entity)
    }
}

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating, makes trees.
pub fn generate_chunk_features(
    // mut event_reader: EventReader<GenerateChunkFeaturesEvent<IceBiosphereMarker>>,
    // mut init_event_writer: EventWriter<ChunkInitEvent>,
    // _block_event_writer: EventWriter<BlockChangedEvent>,
    // mut structure_query: Query<(&mut Structure, &Location)>,
    // _blocks: Res<Registry<Block>>,
    // _seed: Res<ServerSeed>,
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<IceBiosphereMarker>>,
    mut init_event_writer: EventWriter<ChunkInitEvent>,
    mut structure_query: Query<(&mut Structure, &Location)>,
) {
    for ev in event_reader.iter() {
        if let Ok((_structure, _location)) = structure_query.get_mut(ev.structure_entity) {
            let chunk_coords = ev.chunk_coords;

            init_event_writer.send(ChunkInitEvent {
                structure_entity: ev.structure_entity,
                coords: chunk_coords,
            });
        }
    }
}

#[derive(Debug, Resource)]
pub struct IceBiosphereGenerationStrategy {
    iceberg_layers: BlockLayers,
    ocean_layers: BlockLayers,
}

impl BiosphereGenerationStrategy for IceBiosphereGenerationStrategy {
    fn get_layers(
        &self,
        s_dimensions: CoordinateType,
        seed_coords: BlockCoordinate,
        structure_coords: (f64, f64, f64),
        noise_generator: &noise::OpenSimplex,
        up: BlockFace,
        blocks: &Registry<Block>,
    ) -> Vec<(Block, CoordinateType)> {
        // Make edge min bob up and down with noise randomness.
        let iceberg_min = s_dimensions - 180;
        let middle_air_start = s_dimensions - 160;
        let iceberg_max = s_dimensions - 120;

        let iceberg_height = (self.get_block_height(noise_generator, seed_coords, structure_coords, middle_air_start, 1000.0, 0.01, 1)
            as CoordinateType)
            .max(iceberg_max);
        if iceberg_height > iceberg_min {
            // This column is in an iceberg.
            vec![]
        } else {
            // This column is not in an iceberg.
            let mut height = s_dimensions;
            let mut layers = Vec::new();
            for (block, level) in self.ocean_layers.ranges.iter() {
                let level_top = self.get_top_height(
                    up,
                    seed_coords,
                    structure_coords,
                    s_dimensions,
                    noise_generator,
                    height - level.middle_depth,
                    level.amplitude,
                    level.delta,
                    level.iterations,
                );
                layers.push((*block, level_top));
                height = level_top;
            }
            layers
        }
    }
}

fn create_strategy(mut commands: Commands, blocks: Res<Registry<Block>>) {
    commands.insert_resource(IceBiosphereGenerationStrategy {
        // Change iceberg layers.
        iceberg_layers: BlockLayers::default()
            .add_noise_layer("cosmos:ice", &blocks, 160, 0.01, 4.0, 1)
            .expect("Ice missing")
            .add_fixed_layer("cosmos:water", &blocks, 4)
            .expect("Water missing")
            .add_fixed_layer("cosmos:stone", &blocks, 296)
            .expect("Stone missing"),
        ocean_layers: BlockLayers::default()
            .add_noise_layer("cosmos:ice", &blocks, 160, 0.01, 4.0, 1)
            .expect("Ice missing")
            .add_fixed_layer("cosmos:water", &blocks, 4)
            .expect("Water missing")
            .add_fixed_layer("cosmos:stone", &blocks, 296)
            .expect("Stone missing"),
    });
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<IceBiosphereMarker, IceChunkNeedsGeneratedEvent>(app, "cosmos:biosphere_ice", TemperatureRange::new(0.0, 250.0));

    app.add_systems(
        Update,
        (
            generate_planet::<IceBiosphereMarker, IceChunkNeedsGeneratedEvent, IceBiosphereGenerationStrategy>,
            notify_when_done_generating_terrain::<IceBiosphereMarker>,
            generate_chunk_features,
        )
            .run_if(in_state(GameState::Playing)),
    );

    app.add_systems(OnEnter(GameState::PostLoading), create_strategy);
}
