//! Creates a ice planet

use bevy::prelude::*;
use cosmos_core::{
    registry::Registry,
    structure::{
        coordinates::ChunkCoordinate,
        planet::generation::biome::{Biome, BiomeParameters, BiosphereBiomesRegistry},
    },
};

use crate::GameState;

use super::{BiosphereMarkerComponent, TGenerateChunkMessage, TemperatureRange, biome::RegisterBiomesSet, register_biosphere};

#[derive(Component, Debug, Default, Clone, Copy, TypePath)]
/// Marks that this is for a grass biosphere
pub struct IceBiosphereMarker;

impl BiosphereMarkerComponent for IceBiosphereMarker {
    fn unlocalized_name() -> &'static str {
        "cosmos:ice"
    }
}

/// Marks that an ice chunk needs generated
#[derive(Message, Debug)]
pub struct IceChunkNeedsGeneratedMessage {
    chunk_coords: ChunkCoordinate,
    structure_entity: Entity,
}

impl TGenerateChunkMessage for IceChunkNeedsGeneratedMessage {
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
    register_biosphere::<IceBiosphereMarker, IceChunkNeedsGeneratedMessage>(
        app,
        TemperatureRange::new(0.0, 1.0),
        0.75,
        Some("cosmos:water"),
    );

    app.add_systems(
        OnEnter(GameState::PostLoading),
        register_biosphere_biomes
            .in_set(RegisterBiomesSet::RegisterBiomes)
            .ambiguous_with(RegisterBiomesSet::RegisterBiomes),
    );
}
