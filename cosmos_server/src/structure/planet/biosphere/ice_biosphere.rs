//! Creates a ice planet

use bevy::prelude::{
    in_state, App, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, ResMut, Update,
};
use cosmos_core::{
    block::Block,
    physics::location::Location,
    registry::Registry,
    structure::{coordinates::ChunkCoordinate, ChunkInitEvent, Structure},
};

use crate::GameState;

use super::{
    biome::{biome_registry::RegisteredBiome, BiomeParameters, BiosphereBiomesRegistry},
    biosphere_generation::{BlockLayers, GenerateChunkFeaturesEvent},
    register_biosphere, BiosphereMarkerComponent, TBiosphere, TGenerateChunkEvent, TemperatureRange,
};

#[derive(Component, Debug, Default, Clone, Copy)]
/// Marks that this is for a grass biosphere
pub struct IceBiosphereMarker;

impl BiosphereMarkerComponent for IceBiosphereMarker {}

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

// fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
//     commands.insert_resource(
//         BlockLayers::default()
//             .add_noise_layer("cosmos:ice", &block_registry, 160, 0.01, 4.0, 1)
//             .expect("Ice missing")
//             .add_fixed_layer("cosmos:water", &block_registry, 4)
//             .expect("Water missing")
//             .add_fixed_layer("cosmos:stone", &block_registry, 296)
//             .expect("Stone missing"),
//     );
// }

fn register_biosphere_biomes(
    biome_registry: Res<Registry<RegisteredBiome>>,
    mut biosphere_biomes_registry: ResMut<BiosphereBiomesRegistry<IceBiosphereMarker>>,
) {
    if let Some(plains) = biome_registry.from_id("cosmos:plains") {
        biosphere_biomes_registry.register(
            plains.biome(),
            BiomeParameters {
                ideal_elevation: 30.0,
                ideal_humidity: 30.0,
                ideal_temperature: 60.0,
            },
        );
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<IceBiosphereMarker, IceChunkNeedsGeneratedEvent>(app, "cosmos:biosphere_ice", TemperatureRange::new(0.0, 300.0));

    app.add_systems(OnEnter(GameState::PostLoading), register_biosphere_biomes);
}
