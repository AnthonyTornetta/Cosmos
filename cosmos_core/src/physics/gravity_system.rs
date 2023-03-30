use bevy::prelude::*;
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, RigidBody};

use crate::structure::loading::ChunksNeedLoaded;

use super::location::Location;

fn gravity_system(
    // Without<ChunksNeedLoaded> to prevent things from falling through
    // the world before it's done loading.
    emitters: Query<(&GravityEmitter, &GlobalTransform, &Location), Without<ChunksNeedLoaded>>,
    mut receiver: Query<(
        Entity,
        &Location,
        &ReadMassProperties,
        &RigidBody,
        Option<&mut ExternalImpulse>,
    )>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let mut gravs: Vec<(f32, f32, Location, Vec3)> = Vec::with_capacity(emitters.iter().len());

    for (emitter, trans, location) in emitters.iter() {
        gravs.push((
            emitter.force_per_kg,
            emitter.radius,
            *location,
            trans.down(),
        ));
    }

    for (ent, location, prop, rb, external_force) in receiver.iter_mut() {
        if *rb == RigidBody::Dynamic {
            let mut force = Vec3::ZERO;

            for (force_per_kilogram, radius, pos, down) in gravs.iter() {
                let r_sqrd = radius * radius;
                let dist_sqrd = location.distance_sqrd(pos);

                let ratio = if dist_sqrd < r_sqrd {
                    1.0
                } else {
                    r_sqrd / dist_sqrd
                };

                if ratio >= 0.1 {
                    force += (prop.0.mass * force_per_kilogram * ratio) * *down;
                }
            }

            force *= time.delta_seconds();

            if let Some(mut external_force) = external_force {
                external_force.impulse += force;
            } else if let Some(mut entity) = commands.get_entity(ent) {
                entity.insert(ExternalImpulse {
                    impulse: force,
                    ..Default::default()
                });
            }
        }
    }
}

#[derive(Component)]
pub struct GravityEmitter {
    pub force_per_kg: f32, // earth is 9.8
    pub radius: f32, // make this something like max(struct width, struct length, struct height)
}

pub fn register(app: &mut App) {
    app.add_system(gravity_system);
}
