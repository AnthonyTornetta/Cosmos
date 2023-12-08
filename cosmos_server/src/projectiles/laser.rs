use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    projectiles::laser::{Laser, LaserCollideEvent},
    registry::Registry,
    structure::{
        block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent},
        Structure,
    },
};

use crate::{
    persistence::{
        saving::{NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        SerializedData,
    },
    state::GameState,
};

/// Called when the laser hits a structure at a given position
fn on_laser_hit_structure(
    structure: &mut Structure,
    local_position_hit: Vec3,
    blocks: &Registry<Block>,
    block_take_damage_event_writer: &mut EventWriter<BlockTakeDamageEvent>,
    block_destroy_event_writer: &mut EventWriter<BlockDestroyedEvent>,
    strength: f32,
) {
    if let Ok(coords) = structure.relative_coords_to_local_coords_checked(local_position_hit.x, local_position_hit.y, local_position_hit.z)
    {
        structure.block_take_damage(
            coords,
            blocks,
            strength,
            Some((block_take_damage_event_writer, block_destroy_event_writer)),
        );
    } else {
        warn!("Bad laser hit spot that isn't actually on structure ;(");
    }
}

fn respond_laser_hit_event(
    mut reader: EventReader<LaserCollideEvent>,
    parent_query: Query<&Parent>,
    mut structure_query: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut block_take_damage_event_writer: EventWriter<BlockTakeDamageEvent>,
    mut block_destroy_event_writer: EventWriter<BlockDestroyedEvent>,
) {
    for ev in reader.read() {
        let entity_hit = ev.entity_hit();
        if let Ok(parent) = parent_query.get(entity_hit) {
            if let Ok(mut structure) = structure_query.get_mut(parent.get()) {
                let local_position_hit = ev.local_position_hit();

                on_laser_hit_structure(
                    &mut structure,
                    local_position_hit,
                    &blocks,
                    &mut block_take_damage_event_writer,
                    &mut block_destroy_event_writer,
                    ev.laser_strength(),
                );
            }
        }
    }
}

// Don't bother saving lasers
fn on_save_laser(mut query: Query<&mut SerializedData, (With<NeedsSaved>, With<Laser>)>) {
    for mut sd in query.iter_mut() {
        sd.set_should_save(false);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, respond_laser_hit_event.run_if(in_state(GameState::Playing)))
        .add_systems(SAVING_SCHEDULE, on_save_laser.in_set(SavingSystemSet::DoSaving));
}
