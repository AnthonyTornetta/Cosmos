use bevy::{
    app::{App, Update},
    ecs::{
        event::{EventReader, EventWriter},
        schedule::IntoSystemConfigs,
        system::Query,
    },
};
use cosmos_core::{projectiles::laser::LaserCollideEvent, structure::shields::Shield};

use super::{ShieldHitEvent, ShieldHitProcessing};

fn handle_laser_hits(
    mut ev_reader: EventReader<LaserCollideEvent>,
    mut ev_writer: EventWriter<ShieldHitEvent>,
    mut q_shield: Query<&mut Shield>,
) {
    for ev in ev_reader.read() {
        let Ok(mut shield) = q_shield.get_mut(ev.entity_hit()) else {
            continue;
        };

        shield.take_damage(ev.laser_strength());
        ev_writer.send(ShieldHitEvent {
            relative_position: ev.local_position_hit(),
            shield_entity: ev.entity_hit(),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, handle_laser_hits.in_set(ShieldHitProcessing::OnShieldHit));
}
