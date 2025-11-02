use bevy::prelude::*;
use cosmos_core::{
    block::Block,
    entities::health::Health,
    prelude::BlockCoordinate,
    projectiles::{
        causer::Causer,
        laser::{Laser, LaserCollideEvent, LaserSystemSet},
    },
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        block_health::events::{BlockDestroyedEvent, BlockTakeDamageEvent},
    },
};

use crate::{
    persistence::{
        SerializedData,
        saving::{NeedsSaved, SAVING_SCHEDULE, SavingSystemSet},
    },
    structure::{block_health::BlockHealthSet, systems::shield_system::ShieldSet},
};

/// Called when the laser hits a structure at a given position
fn on_laser_hit_structure(
    structure: &mut Structure,
    coords: BlockCoordinate,
    blocks: &Registry<Block>,
    block_take_damage_event_writer: &mut MessageWriter<BlockTakeDamageEvent>,
    block_destroy_event_writer: &mut MessageWriter<BlockDestroyedEvent>,
    strength: f32,
    causer: Option<&Causer>,
) {
    structure.block_take_damage(
        coords,
        blocks,
        strength,
        Some((block_take_damage_event_writer, block_destroy_event_writer)),
        causer.map(|x| x.0),
    );
}

fn respond_laser_hit_event(
    mut reader: EventReader<LaserCollideEvent>,
    mut structure_query: Query<&mut Structure>,
    blocks: Res<Registry<Block>>,
    mut block_take_damage_event_writer: MessageWriter<BlockTakeDamageEvent>,
    mut block_destroy_event_writer: MessageWriter<BlockDestroyedEvent>,
    mut q_health: Query<&mut Health>,
) {
    for ev in reader.read() {
        let entity_hit = ev.entity_hit();

        if let Ok(mut structure) = structure_query.get_mut(entity_hit) {
            let Some(block) = ev.block_hit() else {
                warn!("Bad laser hit spot that isn't actually on structure ;(");
                continue;
            };

            info!("Dealing damage to {:?}", block.coords());

            on_laser_hit_structure(
                &mut structure,
                block.coords(),
                &blocks,
                &mut block_take_damage_event_writer,
                &mut block_destroy_event_writer,
                ev.laser_strength(),
                ev.causer().as_ref(),
            );
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
        FixedUpdate,
        respond_laser_hit_event
            .in_set(BlockHealthSet::SendHealthChanges)
            .after(LaserSystemSet::SendHitEvents)
            .after(ShieldSet::OnShieldHit)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(SAVING_SCHEDULE, on_save_laser.in_set(SavingSystemSet::DoSaving));
}
