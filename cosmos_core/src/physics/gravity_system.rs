//! Handles gravity

use bevy::prelude::*;
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, RigidBody, RigidBodyDisabled};

use crate::structure::planet::Planet;

use super::location::Location;

fn gravity_system(
    emitters: Query<(Entity, &GravityEmitter, &GlobalTransform, &Location)>,
    mut receiver: Query<(Entity, &Location, &ReadMassProperties, &RigidBody, Option<&mut ExternalImpulse>), Without<RigidBodyDisabled>>,
    time: Res<Time>,
    mut commands: Commands,
) {
    let gravs = emitters
        .iter()
        .map(|(ent, emitter, global_transform, location)| {
            (
                ent,
                emitter.force_per_kg,
                emitter.radius,
                *location,
                Quat::from_affine3(&global_transform.affine()),
            )
        })
        .collect::<Vec<(Entity, f32, f32, Location, Quat)>>();

    for (ent, location, prop, rb, external_force) in receiver.iter_mut() {
        if *rb == RigidBody::Dynamic {
            let mut force = Vec3::ZERO;

            for (_, force_per_kilogram, radius, pos, rotation) in gravs.iter().filter(|emitter| emitter.0 != ent) {
                let relative_position = pos.relative_coords_to(location);
                let dist = relative_position.abs().max_element();

                let ratio = ((radius * radius) / (dist * dist)).min(1.0);

                if ratio >= 0.9 {
                    let face = Planet::planet_face_relative(relative_position);

                    let grav_dir = -rotation.mul_vec3(face.direction_vec3());

                    force += (prop.get().mass * force_per_kilogram * ratio) * grav_dir;
                } else if ratio >= 0.1 {
                    let grav_dir = -relative_position.normalize();

                    force += (prop.get().mass * force_per_kilogram * ratio) * grav_dir;
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

#[derive(Component, Reflect, Debug)]
/// If something emits gravity, it should have this component.
pub struct GravityEmitter {
    /// How much force to apply per kg (Earth is 9.8)
    pub force_per_kg: f32,
    /// Determines how far away you can be before gravity starts to deminish.
    ///
    /// For structures, make this something like max(struct width, struct length, struct height).
    pub radius: f32,
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, gravity_system);
}
