use bevy::prelude::*;
use cosmos_core::{
    entities::health::{Health, HealthSet},
    physics::location::Location,
    structure::shields::Shield,
};

use crate::projectiles::explosion::ExplosionHitMessage;

use super::{ShieldHitMessage, ShieldSet};

fn respond_to_explosion_damage(
    mut ev_reader: MessageReader<ExplosionHitMessage>,
    mut q_shield: Query<(&mut Shield, &Location)>,
    mut ev_writer: MessageWriter<ShieldHitMessage>,
    mut q_health: Query<(&mut Health, &Location)>,
) {
    for ev in ev_reader.read() {
        if let Ok((mut shield, shield_location)) = q_shield.get_mut(ev.hit_entity) {
            let damage =
                ev.explosion.power / (shield_location.distance_sqrd(&ev.explosion_location) - (shield.radius * shield.radius)).max(1.0);

            let relative_position = (ev.explosion_location - *shield_location).absolute_coords_f32();

            shield.take_damage(damage * 2.0);
            ev_writer.write(ShieldHitMessage {
                relative_position,
                shield_entity: ev.hit_entity,
            });
        } else if let Ok((mut health, loc)) = q_health.get_mut(ev.hit_entity) {
            let damage = ev.explosion.power / (loc.distance_sqrd(&ev.explosion_location)).max(1.0);

            health.take_damage(damage as u32 * 2);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        respond_to_explosion_damage
            .before(HealthSet::ProcessHealthChange)
            .in_set(ShieldSet::OnShieldHit),
    );
}
