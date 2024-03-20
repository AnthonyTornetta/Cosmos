//! Creates a ice planet

use bevy::{
    log::warn,
    prelude::{App, Component, Entity, Event, OnEnter, Res, ResMut},
    reflect::TypePath,
};
use cosmos_core::{
    registry::Registry,
    structure::{
        coordinates::ChunkCoordinate,
        planet::generation::biome::{Biome, BiomeParameters, BiosphereBiomesRegistry},
    },
};

use crate::GameState;

use super::{register_biosphere, BiosphereMarkerComponent, TBiosphere, TGenerateChunkEvent, TemperatureRange};

#[derive(Component, Debug, Default, Clone, Copy, TypePath)]
/// Marks that this is for a grass biosphere
pub struct IceBiosphereMarker;

impl BiosphereMarkerComponent for IceBiosphereMarker {
    fn unlocalized_name() -> &'static str {
        "cosmos:ice"
    }
}

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
struct IceBiosphere;

impl TBiosphere<IceBiosphereMarker, IceChunkNeedsGeneratedEvent> for IceBiosphere {
    fn get_marker_component(&self) -> IceBiosphereMarker {
        IceBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, chunk_coords: ChunkCoordinate, structure_entity: Entity) -> IceChunkNeedsGeneratedEvent {
        IceChunkNeedsGeneratedEvent::new(chunk_coords, structure_entity)
    }
}

fn register_biosphere_biomes(
    biome_registry: Res<Registry<Biome>>,
    mut biosphere_biomes_registry: ResMut<Registry<BiosphereBiomesRegistry>>,
) {
    let biosphere_registry = biosphere_biomes_registry
        .from_id_mut(IceBiosphereMarker::unlocalized_name())
        .expect("Missing ice biosphere registry!");

    if let Some(plains) = biome_registry.from_id("cosmos:ice") {
        biosphere_registry.register(
            plains,
            BiomeParameters {
                ideal_elevation: 30.0,
                ideal_humidity: 30.0,
                ideal_temperature: 60.0,
            },
        );
    } else {
        warn!("Missing ice biome!");
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<IceBiosphereMarker, IceChunkNeedsGeneratedEvent>(
        app,
        TemperatureRange::new(0.0, 300.0),
        0.75,
        Some("cosmos:water"),
    );

    app.add_systems(OnEnter(GameState::PostLoading), register_biosphere_biomes);
}
