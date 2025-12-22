//! Creates a molten planet

use bevy::prelude::*;
use cosmos_core::{
    registry::Registry,
    structure::{
        coordinates::ChunkCoordinate,
        planet::generation::biome::{Biome, BiomeParameters, BiosphereBiomesRegistry},
    },
};

use crate::GameState;

use super::{BiosphereMarkerComponent, RegisterBiomesSet, TGenerateChunkMessage, TemperatureRange, register_biosphere};

#[derive(Component, Debug, Default, Clone, Copy, TypePath)]
/// Marks that this is for a grass biosphere
pub struct MoltenBiosphereMarker;

impl BiosphereMarkerComponent for MoltenBiosphereMarker {
    fn unlocalized_name() -> &'static str {
        "cosmos:molten"
    }
}

/// Marks that a grass chunk needs generated
#[derive(Debug, Message)]
pub struct MoltenChunkNeedsGeneratedMessage {
    coords: ChunkCoordinate,
    structure_entity: Entity,
}

impl TGenerateChunkMessage for MoltenChunkNeedsGeneratedMessage {
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

fn register_biosphere_biomes(
    biome_registry: Res<Registry<Biome>>,
    mut biosphere_biomes_registry: ResMut<Registry<BiosphereBiomesRegistry>>,
) {
    let biosphere_registry = biosphere_biomes_registry
        .from_id_mut(MoltenBiosphereMarker::unlocalized_name())
        .expect("Missing molten biosphere registry!");

    if let Some(molten_biome) = biome_registry.from_id("cosmos:molten") {
        biosphere_registry.register(
            molten_biome,
            BiomeParameters {
                ideal_elevation: 30.0,
                ideal_humidity: 30.0,
                ideal_temperature: 60.0,
            },
        );
    } else {
        warn!("Missing molten biome!");
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedMessage>(
        app,
        TemperatureRange::new(450.0, f32::MAX),
        0.75,
        Some("cosmos:lava"),
    );

    app.add_systems(
        OnEnter(GameState::PostLoading),
        register_biosphere_biomes
            .in_set(RegisterBiomesSet::RegisterBiomes)
            .ambiguous_with(RegisterBiomesSet::RegisterBiomes),
    );
}
