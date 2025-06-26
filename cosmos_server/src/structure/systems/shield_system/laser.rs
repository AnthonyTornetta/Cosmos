use bevy::prelude::*;
use cosmos_core::{projectiles::laser::LaserCollideEvent, structure::shields::Shield};

use super::{ShieldHitEvent, ShieldSet};

fn handle_laser_hits(
    mut ev_reader: EventReader<LaserCollideEvent>,
    mut ev_writer: EventWriter<ShieldHitEvent>,
    mut q_shield: Query<(&GlobalTransform, &mut Shield)>,
) {
    for ev in ev_reader.read() {
        let Ok((shield_g_trans, mut shield)) = q_shield.get_mut(ev.entity_hit()) else {
            continue;
        };

        shield.take_damage(ev.laser_strength());
        ev_writer.write(ShieldHitEvent {
            relative_position: shield_g_trans.affine().matrix3.mul_vec3(ev.local_position_hit()),
            shield_entity: ev.entity_hit(),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        handle_laser_hits
            .in_set(ShieldSet::OnShieldHit)
            .ambiguous_with(ShieldSet::OnShieldHit),
    );
}
