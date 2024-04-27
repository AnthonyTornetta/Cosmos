use bevy::{
    app::{App, Update},
    ecs::{event::EventReader, system::Query},
};
use cosmos_core::{physics::location::Location, structure::shields::Shield};

use crate::projectiles::missile::ExplosionHitEvent;

fn respond_to_explosion_damage(mut ev_reader: EventReader<ExplosionHitEvent>, mut q_shield: Query<(&mut Shield, &Location)>) {
    for ev in ev_reader.read() {
        let Ok((mut shield, shield_location)) = q_shield.get_mut(ev.hit_entity) else {
            continue;
        };

        let damage =
            ev.explosion.power / (shield_location.distance_sqrd(&ev.explosion_location) - (shield.radius * shield.radius)).max(1.0);

        shield.take_damage(damage * 2.0);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, respond_to_explosion_damage);
}
