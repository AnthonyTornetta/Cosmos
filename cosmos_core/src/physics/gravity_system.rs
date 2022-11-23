use bevy::prelude::Query;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, RigidBody};

use crate::utils::vec_math::dot;

// Remove these divisions when the radius actually makes sense
const G: f64 = 6.6743e-11 / 637810.0 / 637810.0;

fn gravity_system(
    emitters: Query<(&GravityEmitter, &Transform)>,
    receiver: Query<(Entity, &Transform, &ReadMassProperties, &RigidBody)>,
    mut commands: Commands,
) {
    let mut gravs: Vec<(f32, Vec3)> = Vec::with_capacity(emitters.iter().len());

    for (emitter, trans) in emitters.iter() {
        gravs.push((emitter.mass, trans.translation));
    }

    for (ent, trans, prop, rb) in receiver.iter() {
        if *rb == RigidBody::Dynamic {
            let mut force = Vec3::ZERO;

            for (mass, pos) in gravs.iter() {
                let diff = *pos - trans.translation;
                let rsqrd = dot(&diff, &diff);

                let top = (G * prop.0.mass as f64 * *mass as f64) as f32;
                let direction = Vec3::new(
                    pos.x - trans.translation.x,
                    pos.y - trans.translation.y,
                    pos.z - trans.translation.z,
                )
                .normalize();

                force += (top / rsqrd) * direction;
            }

            commands.entity(ent).insert(ExternalImpulse {
                impulse: force,
                ..Default::default()
            });
        }
    }
}

#[derive(Component)]
pub struct GravityEmitter {
    pub mass: f32,
}

pub fn register(app: &mut App) {
    app.add_system(gravity_system);
}
