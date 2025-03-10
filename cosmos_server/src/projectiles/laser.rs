use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    entities::health::Health,
    netty::system_sets::NetworkingSystemsSet,
    projectiles::{
        causer::Causer,
        laser::{Laser, LaserCollideEvent, LaserSystemSet},
    },
    registry::Registry,
    state::GameState,
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
    structure::{block_health::BlockHealthSet, systems::shield_system::ShieldSet},
};

/// Called when the laser hits a structure at a given position
fn on_laser_hit_structure(
    structure: &mut Structure,
    local_position_hit: Vec3,
    blocks: &Registry<Block>,
    block_take_damage_event_writer: &mut EventWriter<BlockTakeDamageEvent>,
    block_destroy_event_writer: &mut EventWriter<BlockDestroyedEvent>,
    strength: f32,
    causer: Option<&Causer>,
) {
    if let Ok(coords) = structure.relative_coords_to_local_coords_checked(local_position_hit.x, local_position_hit.y, local_position_hit.z)
    {
        structure.block_take_damage(
            coords,
            blocks,
            strength,
            Some((block_take_damage_event_writer, block_destroy_event_writer)),
            causer.map(|x| x.0),
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
    mut q_health: Query<&mut Health>,
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
                    ev.causer().as_ref(),
                );
            }
        } else if let Ok(mut health) = q_health.get_mut(entity_hit) {
            health.take_damage(ev.laser_strength() as u32 / 2);
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
    app.add_systems(
        Update,
        respond_laser_hit_event
            .in_set(NetworkingSystemsSet::Between)
            .in_set(BlockHealthSet::SendHealthChanges)
            .after(LaserSystemSet::SendHitEvents)
            .after(ShieldSet::OnShieldHit)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(SAVING_SCHEDULE, on_save_laser.in_set(SavingSystemSet::DoSaving));
}
