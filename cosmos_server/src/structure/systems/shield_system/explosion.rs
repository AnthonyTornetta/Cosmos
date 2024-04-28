use bevy::{
    app::{App, Update},
    ecs::{
        event::{EventReader, EventWriter},
        schedule::IntoSystemConfigs,
        system::Query,
    },
};
use cosmos_core::{physics::location::Location, structure::shields::Shield};

use crate::projectiles::explosion::ExplosionHitEvent;

use super::{ShieldHitEvent, ShieldHitProcessing};

fn respond_to_explosion_damage(
    mut ev_reader: EventReader<ExplosionHitEvent>,
    mut q_shield: Query<(&mut Shield, &Location)>,
    mut ev_writer: EventWriter<ShieldHitEvent>,
) {
    for ev in ev_reader.read() {
        let Ok((mut shield, shield_location)) = q_shield.get_mut(ev.hit_entity) else {
            continue;
        };

        let damage =
            ev.explosion.power / (shield_location.distance_sqrd(&ev.explosion_location) - (shield.radius * shield.radius)).max(1.0);

        let relative_position = (ev.explosion_location - *shield_location).absolute_coords_f32();

        shield.take_damage(damage * 2.0);
        ev_writer.send(ShieldHitEvent {
            relative_position,
            shield_entity: ev.hit_entity,
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, respond_to_explosion_damage.in_set(ShieldHitProcessing::OnShieldHit));
}
