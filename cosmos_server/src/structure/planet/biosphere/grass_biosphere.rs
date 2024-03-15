//! Creates a grass planet

use bevy::{
    log::warn,
    prelude::{App, Commands, Component, Entity, Event, OnEnter, Res, ResMut},
    reflect::TypePath,
};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::{
        coordinates::ChunkCoordinate,
        planet::generation::biome::{Biome, BiomeParameters, BiosphereBiomesRegistry},
    },
};

use crate::GameState;

use super::{register_biosphere, BiosphereMarkerComponent, BiosphereSeaLevel, TBiosphere, TGenerateChunkEvent, TemperatureRange};

#[derive(Component, Debug, Default, Clone, Copy, TypePath)]
/// Marks that this is for a grass biosphere
pub struct GrassBiosphereMarker;

impl BiosphereMarkerComponent for GrassBiosphereMarker {
    fn unlocalized_name() -> &'static str {
        "cosmos:grass"
    }
}

/// Marks that a grass chunk needs generated
#[derive(Debug, Event)]
pub struct GrassChunkNeedsGeneratedEvent {
    coords: ChunkCoordinate,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for GrassChunkNeedsGeneratedEvent {
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
/// Creates a grass planet
pub struct GrassBiosphere;

impl TBiosphere<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent> for GrassBiosphere {
    fn get_marker_component(&self) -> GrassBiosphereMarker {
        GrassBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, coords: ChunkCoordinate, structure_entity: Entity) -> GrassChunkNeedsGeneratedEvent {
        GrassChunkNeedsGeneratedEvent::new(coords, structure_entity)
    }
}

fn register_biosphere_biomes(
    biome_registry: Res<Registry<Biome>>,
    mut biosphere_biomes_registry: ResMut<Registry<BiosphereBiomesRegistry>>,
) {
    let biosphere_registry = biosphere_biomes_registry
        .from_id_mut(GrassBiosphereMarker::unlocalized_name())
        .expect("Missing grass biosphere registry!");

    if let Some(ocean) = biome_registry.from_id("cosmos:ocean") {
        biosphere_registry.register(
            ocean,
            BiomeParameters {
                ideal_elevation: 49.0,
                ideal_humidity: 0.0,
                ideal_temperature: 30.0,
            },
        );
    } else {
        warn!("Missing ocean biome!");
    }

    if let Some(plains) = biome_registry.from_id("cosmos:plains") {
        biosphere_registry.register(
            plains,
            BiomeParameters {
                ideal_elevation: 50.0,
                ideal_humidity: 0.0,
                ideal_temperature: 30.0,
            },
        );
    } else {
        warn!("Missing plains biome!");
    }

    if let Some(desert) = biome_registry.from_id("cosmos:desert") {
        biosphere_registry.register(
            desert,
            BiomeParameters {
                ideal_elevation: 50.0,
                ideal_humidity: 0.0,
                ideal_temperature: 100.0,
            },
        );
    } else {
        warn!("Missing desert biome!");
    }
}

fn add_ocean_level(mut commands: Commands, blocks: Res<Registry<Block>>) {
    if let Some(water) = blocks.from_id("cosmos:water") {
        commands.insert_resource(BiosphereSeaLevel::<GrassBiosphereMarker> {
            block: Some(water.clone()),
            ..Default::default()
        });
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>(app, TemperatureRange::new(0.0, 400.0));

    app.add_systems(OnEnter(GameState::PostLoading), (register_biosphere_biomes, add_ocean_level));
}
