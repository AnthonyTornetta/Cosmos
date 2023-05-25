//! Handles gravity

use bevy::prelude::*;
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, RigidBody};

use crate::structure::planet::Planet;

use super::location::Location;

fn gravity_system(
    emitters: Query<(Entity, &GravityEmitter, &GlobalTransform, &Location)>,
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
    let mut gravs: Vec<(Entity, f32, f32, Location, Quat)> =
        Vec::with_capacity(emitters.iter().len());

    for (entity, emitter, trans, location) in emitters.iter() {
        gravs.push((
            entity,
            emitter.force_per_kg,
            emitter.radius,
            *location,
            Quat::from_affine3(&trans.affine()),
        ));
    }

    for (ent, location, prop, rb, external_force) in receiver.iter_mut() {
        if *rb == RigidBody::Dynamic {
            let mut x = 0;
            let mut force = Vec3::ZERO;

            for (entity, force_per_kilogram, radius, pos, rotation) in gravs.iter() {
                let relative_position = pos.relative_coords_to(location);
                let dist = relative_position.abs().max_element();

                let ratio = ((radius * radius) / (dist * dist)).min(1.0);

                if ratio >= 0.9 {
                    let face = Planet::planet_face_relative(relative_position);

                    let grav_dir = -rotation.mul_vec3(face.direction_vec3());

                    force += (prop.0.mass * force_per_kilogram * ratio) * grav_dir;

                    x += 1;

                    println!("{x} {}", entity.index());
                } else if ratio >= 0.1 {
                    let grav_dir = -relative_position.normalize();

                    force += (prop.0.mass * force_per_kilogram * ratio) * grav_dir;
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

#[derive(Component, Reflect, FromReflect, Debug)]
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
    app.add_system(gravity_system);
}
