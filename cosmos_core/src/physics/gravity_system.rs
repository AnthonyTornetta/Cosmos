use bevy::prelude::Query;
use bevy::prelude::*;
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, RigidBody};

fn gravity_system(
    emitters: Query<(&GravityEmitter, &GlobalTransform)>,
    receiver: Query<(Entity, &GlobalTransform, &ReadMassProperties, &RigidBody)>,
    mut commands: Commands,
) {
    println!("ASDF");

    let mut gravs: Vec<(f32, f32, Vec3, Vec3)> = Vec::with_capacity(emitters.iter().len());

    for (emitter, trans) in emitters.iter() {
        gravs.push((
            emitter.force_per_kg,
            emitter.radius,
            trans.translation(),
            trans.down(),
        ));
    }

    for (ent, trans, prop, rb) in receiver.iter() {
        if *rb == RigidBody::Dynamic {
            if let Some(mut entity) = commands.get_entity(ent) {
                let mut force = Vec3::ZERO;
                let translation = trans.translation();

                for (force_per_kilogram, radius, pos, down) in gravs.iter() {
                    let r_sqrd = radius * radius;
                    let dist_sqrd = translation.distance_squared(*pos);

                    let ratio = if dist_sqrd < r_sqrd {
                        1.0
                    } else {
                        r_sqrd / dist_sqrd
                    };

                    force += (prop.0.mass * force_per_kilogram * ratio) / 100.0 * *down;
                }

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
