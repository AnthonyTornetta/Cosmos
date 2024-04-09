use bevy::{
    ecs::{query::Added, schedule::IntoSystemConfigs},
    prelude::{App, Commands, Entity, Query, Update},
};

use cosmos_core::{
    ecs::NeedsDespawned,
    projectiles::missile::{Explosion, ExplosionSystemSet},
};

fn respond_to_explosion(mut commands: Commands, q_explosions: Query<(Entity,), Added<Explosion>>) {
    for (ent,) in q_explosions.iter() {
        commands.entity(ent).insert(NeedsDespawned);

        println!("TODO: Play cool explosion effect instead of despawning");
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, respond_to_explosion.in_set(ExplosionSystemSet::ProcessExplosions));
}
