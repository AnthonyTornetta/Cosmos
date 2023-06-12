//! Creates a grass planet
use std::{collections::HashSet, mem::swap};

use bevy::{
    prelude::{
        App, Component, Entity, EventReader, EventWriter, IntoSystemConfigs, OnUpdate, Query, Res,
        ResMut,
    },
    tasks::AsyncComputeTaskPool,
};
use cosmos_core::{
    block::{Block, BlockFace},
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::{Chunk, CHUNK_DIMENSIONS},
        planet::Planet,
        ChunkInitEvent, Structure,
    },
    utils::{resource_wrapper::ResourceWrapper, timer::UtilsTimer},
};
use futures_lite::future;
use noise::NoiseFn;

use crate::GameState;

use super::{
    biosphere_generation::{generate_planet, notify_when_done_generating},
    register_biosphere, GeneratingChunk, GeneratingChunks, TBiosphere, TGenerateChunkEvent,
    TemperatureRange,
};

#[derive(Component, Debug, Default)]
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

pub(super) fn register(app: &mut App) {
    register_biosphere::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_grass",
        TemperatureRange::new(0.0, 1000000000.0),
    );

    app.add_systems(
        (
            generate_planet::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>,
            notify_when_done_generating::<GrassBiosphereMarker>,
        )
            .in_set(OnUpdate(GameState::Playing)),
    );
}
