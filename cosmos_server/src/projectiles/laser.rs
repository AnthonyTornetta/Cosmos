use bevy::prelude::*;
use cosmos_core::{
    block::{hardness::BlockHardness, Block},
    events::block_events::BlockChangedEvent,
    projectiles::laser::LaserCollideEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{block_health::block_destroyed_event::BlockDestroyedEvent, Structure},
};

use crate::state::GameState;

/**
 * Called when the laser hits a structure at a given position
 */
fn on_laser_hit_structure(
    structure: &mut Structure,
    entity_hit: Entity,
    local_position_hit: Vec3,
    blocks: &Registry<Block>,
    block_change_event_writer: &mut EventWriter<BlockChangedEvent>,
    block_destroy_event_writer: &mut EventWriter<BlockDestroyedEvent>,
    hardness_registry: &Registry<BlockHardness>,
    strength: f32,
) {
    if let Some(chunk) = structure.chunk_from_entity(&entity_hit) {
        let chunk_block_coords = chunk.relative_coords_to_block_coords(&local_position_hit);

        let (bx, by, bz) = structure.block_coords_for_chunk_block_coords(chunk, chunk_block_coords);

        if structure.is_within_blocks(bx, by, bz) {
            let block = structure.block_at(bx, by, bz, blocks);

            if let Some(hardness) = hardness_registry.from_id(block.unlocalized_name()) {
                structure.block_take_damage(
                    bx,
                    by,
                    bz,
                    hardness,
                    strength,
                    Some(block_destroy_event_writer),
                );
            } else {
                println!(
                    "WARNING: Missing block hardness for {}",
                    block.unlocalized_name()
                );
                structure.remove_block_at(bx, by, bz, blocks, Some(block_change_event_writer));
            }
        } else {
            println!("Bad laser hit spot that isn't actually on structure ;(");
            println!("HIT {bx}, {by}, {bz} block coords of structure from {local_position_hit}!");
        }
    }
}

fn respond_laser_hit_event(
    mut reader: EventReader<LaserCollideEvent>,
    parent_query: Query<&Parent>,
    mut structure_query: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut block_change_event_writer: EventWriter<BlockChangedEvent>,
    mut block_destroy_event_writer: EventWriter<BlockDestroyedEvent>,
    hardness_registry: Res<Registry<BlockHardness>>,
) {
    for ev in reader.iter() {
        let entity_hit = ev.entity_hit();
        if let Ok(parent) = parent_query.get(entity_hit) {
            if let Ok(mut structure) = structure_query.get_mut(parent.get()) {
                let local_position_hit = ev.local_position_hit();

                on_laser_hit_structure(
                    &mut structure,
                    entity_hit,
                    local_position_hit,
                    &blocks,
                    &mut block_change_event_writer,
                    &mut block_destroy_event_writer,
                    &hardness_registry,
                    ev.laser_strength(),
                );
            }
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system_set(
        SystemSet::on_update(GameState::Playing).with_system(respond_laser_hit_event),
    );
}
