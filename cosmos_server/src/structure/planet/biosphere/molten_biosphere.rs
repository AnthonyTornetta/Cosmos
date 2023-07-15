//! Creates a molten planet

use bevy::prelude::{
    App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemAppConfig, IntoSystemConfigs, OnEnter, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{ChunkInitEvent, Structure},
    utils::resource_wrapper::ResourceWrapper,
};

use crate::GameState;

use super::{
    biosphere_generation::{
        generate_planet, notify_when_done_generating_terrain, BlockRanges, DefaultBiosphereGenerationStrategy, GenerateChunkFeaturesEvent,
        GenerationParemeters,
    },
    register_biosphere, TBiosphere, TGenerateChunkEvent, TemperatureRange,
};

#[derive(Component, Debug, Default, Clone)]
/// Marks that this is for a grass biosphere
pub struct MoltenBiosphereMarker;

/// Marks that a grass chunk needs generated
pub struct MoltenChunkNeedsGeneratedEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for MoltenChunkNeedsGeneratedEvent {
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
/// Creates a molten planet
pub struct MoltenBiosphere;

impl TBiosphere<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedEvent> for MoltenBiosphere {
    fn get_marker_component(&self) -> MoltenBiosphereMarker {
        MoltenBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, x: usize, y: usize, z: usize, structure_entity: Entity) -> MoltenChunkNeedsGeneratedEvent {
        MoltenChunkNeedsGeneratedEvent::new(x, y, z, structure_entity)
    }
}

fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
    commands.insert_resource(
        BlockRanges::<MoltenBiosphereMarker>::default()
            .with_sea_level_block("cosmos:cheese", &block_registry, -20)
            .expect("Cheese missing!")
            .with_range("cosmos:molten_stone", &block_registry, 0)
            .expect("Molten Stone missing"),
    );
}

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating, makes trees.
pub fn generate_chunk_features(
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<MoltenBiosphereMarker>>,
    mut init_event_writer: EventWriter<ChunkInitEvent>,
    mut _block_event_writer: EventWriter<BlockChangedEvent>,
    mut structure_query: Query<(&mut Structure, &Location)>,
    _blocks: Res<Registry<Block>>,
    _noise_generator: Res<ResourceWrapper<noise::OpenSimplex>>,
) {
    for ev in event_reader.iter() {
        if let Ok((mut _structure, _location)) = structure_query.get_mut(ev.structure_entity) {
            let (cx, cy, cz) = ev.chunk_coords;
            // trees(
            //     (cx, cy, cz),
            //     &mut structure,
            //     location,
            //     &mut block_event_writer,
            //     &blocks,
            //     &noise_generator,
            // );

            init_event_writer.send(ChunkInitEvent {
                structure_entity: ev.structure_entity,
                x: cx,
                y: cy,
                z: cz,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_molten",
        TemperatureRange::new(0.0, 1000000000.0),
    );

    app.add_systems(
        (
            generate_planet::<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedEvent, DefaultBiosphereGenerationStrategy>,
            notify_when_done_generating_terrain::<MoltenBiosphereMarker>,
            generate_chunk_features,
        )
            .in_set(OnUpdate(GameState::Playing)),
    )
    .insert_resource(GenerationParemeters::<MoltenBiosphereMarker>::new(0.10, 7.0, 9));

    app.add_system(make_block_ranges.in_schedule(OnEnter(GameState::PostLoading)));
}
