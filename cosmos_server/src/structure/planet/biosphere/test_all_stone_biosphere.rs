//! Used just for testing, this makes a planet all stone

use bevy::prelude::{
    App, Component, Entity, EventReader, EventWriter, IntoSystemConfigs, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::Block,
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, ChunkInitEvent, Structure},
};

use crate::structure::planet::generation::planet_generator::check_needs_generated_system;
use crate::GameState;

use super::{TBiosphere, TGenerateChunkEvent};

#[derive(Component)]
/// Used just for testing, this makes a planet all stone
pub struct TestStoneBiosphereMarker;

/// Used just for testing, this makes a planet all stone
pub struct TestStoneChunkNeedsGeneratedEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for TestStoneChunkNeedsGeneratedEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self {
        Self {
            x,
            y,
            z,
            structure_entity,
        }
    }
}

#[derive(Default)]
/// Used just for testing, this makes a planet all stone
pub struct TestStoneBiosphere;

impl TBiosphere<TestStoneBiosphereMarker, TestStoneChunkNeedsGeneratedEvent>
    for TestStoneBiosphere
{
    fn get_marker_component(&self) -> TestStoneBiosphereMarker {
        TestStoneBiosphereMarker
    }

    fn get_generate_chunk_event(
        &self,
        x: usize,
        y: usize,
        z: usize,
        structure_entity: Entity,
    ) -> TestStoneChunkNeedsGeneratedEvent {
        TestStoneChunkNeedsGeneratedEvent::new(x, y, z, structure_entity)
    }
}

fn generate_planet(
    mut query: Query<&mut Structure>,
    mut events: EventReader<TestStoneChunkNeedsGeneratedEvent>,
    mut event_writer: EventWriter<ChunkInitEvent>,
    blocks: Res<Registry<Block>>,
) {
    for ev in events.iter() {
        let mut structure = query.get_mut(ev.structure_entity).unwrap();

        let (start_x, start_y, start_z) = (
            ev.x * CHUNK_DIMENSIONS,
            ev.y * CHUNK_DIMENSIONS,
            ev.z * CHUNK_DIMENSIONS,
        );

        let stone = blocks.from_id("cosmos:stone").unwrap();

        for z in start_z..(start_z + CHUNK_DIMENSIONS) {
            for x in start_x..(start_x + CHUNK_DIMENSIONS) {
                for y in start_y..(start_y + CHUNK_DIMENSIONS) {
                    structure.set_block_at(x, y, z, stone, &blocks, None);
                }
            }
        }

        event_writer.send(ChunkInitEvent {
            structure_entity: ev.structure_entity,
            x: ev.x,
            y: ev.y,
            z: ev.z,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<TestStoneChunkNeedsGeneratedEvent>()
        .add_systems(
            (
                generate_planet,
                check_needs_generated_system::<
                    TestStoneChunkNeedsGeneratedEvent,
                    TestStoneBiosphereMarker,
                >,
            )
                .in_set(OnUpdate(GameState::Playing)),
        );
}
