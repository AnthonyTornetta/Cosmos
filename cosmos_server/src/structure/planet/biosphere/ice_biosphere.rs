//! Creates a ice planet

use bevy::prelude::{
    in_state, App, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, Update,
};
use cosmos_core::{
    block::Block,
    physics::location::Location,
    registry::Registry,
    structure::{coordinates::ChunkCoordinate, ChunkInitEvent, Structure},
};

use crate::GameState;

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

pub(super) fn register(app: &mut App) {
    register_biosphere::<IceBiosphereMarker, IceChunkNeedsGeneratedEvent, DefaultBiosphereGenerationStrategy>(
        app,
        "cosmos:biosphere_ice",
        TemperatureRange::new(0.0, 1.0),
    );

    app.add_systems(Update, generate_chunk_features.run_if(in_state(GameState::Playing)))
        .add_systems(OnEnter(GameState::PostLoading), make_block_ranges);
}
