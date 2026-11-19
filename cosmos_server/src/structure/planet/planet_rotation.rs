//! Rotates planets
//!
//! TODO: Planets will not rotate if they are unloaded. To fix this, we will need some sort of
//! global total playtime in the server, and base the rotation off that.

use std::{f64::consts::TAU, time::Duration};

use bevy::{math::DQuat, prelude::*};
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::sync::IdentifiableComponent,
    physics::location::Location,
    prelude::{Planet, Structure},
};
use rand::RngExt;
use serde::{Deserialize, Serialize};

use crate::{
    init::init_world::ServerSeed,
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
    rng::get_rng_for_sector,
};

#[derive(Component, Reflect, Serialize, Deserialize)]
/// Represents the axis of rotation
struct PlanetRotation {
    axis: Dir3,
    /// Radians per second (should be very small)
    duration_per_revolution: Duration,
}

impl IdentifiableComponent for PlanetRotation {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:planet_rotation"
    }
}

impl DefaultPersistentComponent for PlanetRotation {}

// WARNING: This is duplicated in the client's `rotate_around_planet.rs` file.
fn within_rotation_range(planet: &Structure, planet_loc: &Location, your_loc: &Location) -> bool {
    let radius = match planet {
        Structure::Dynamic(d) => d.block_dimensions() as f32,
        _ => panic!("Planet must be a dynamic structure!"),
    };

    let max_radius = radius * 2.0;

    your_loc.is_within_reasonable_range(planet_loc) && Vec3::from(*your_loc - *planet_loc).length_squared() < max_radius * max_radius
}

// NOTE: Logic for rotating players id done client-side in `rotate_around_planet.rs`, so if you
// chagne this update that too.
fn rotate_planets(
    mut q_planets: Query<(&PlanetRotation, &mut Transform, &Location, &Structure), With<Planet>>,
    mut q_everything_else: Query<(&mut Transform, &mut Location), (Without<ChildOf>, Without<Planet>)>,
    time: Res<Time>,
) {
    for (planet_rotation, mut transform, planet_loc, structure) in q_planets.iter_mut() {
        let delta_rot = if planet_rotation.duration_per_revolution != Duration::ZERO {
            DQuat::from_axis_angle(
                planet_rotation.axis.as_vec3().as_dvec3().normalize(),
                TAU * time.delta_secs_f64() / planet_rotation.duration_per_revolution.as_secs_f64(),
            )
        } else {
            DQuat::IDENTITY
        };

        let previous_rotation = transform.rotation.as_dquat().normalize();
        transform.rotation = (delta_rot * previous_rotation).normalize().as_quat().normalize();
        // Carry entities by the same rotation that clients will observe after f32 storage.
        let delta_rot = (transform.rotation.as_dquat().normalize() * previous_rotation.inverse()).normalize();

        for (mut trans, mut loc) in q_everything_else
            .iter_mut()
            .filter(|x| within_rotation_range(structure, planet_loc, &x.1))
        {
            trans.rotation = (delta_rot * trans.rotation.as_dquat()).normalize().as_quat().normalize();
            loc.rotate_around(planet_loc, delta_rot);
        }
    }
}

fn add_planet_rotation(
    mut commands: Commands,
    server_seed: Res<ServerSeed>,
    q_planets_without_rotation: Query<(Entity, &Location), (With<Planet>, Without<PlanetRotation>)>,
) {
    for (ent, location) in q_planets_without_rotation.iter() {
        let mut rng = get_rng_for_sector(&server_seed, &location.sector);

        commands.entity(ent).insert(PlanetRotation {
            duration_per_revolution: Duration::from_mins(rng.random_range(40..=180)),
            axis: Dir3::new(Vec3::new(rng.random(), rng.random(), rng.random()).normalize_or_zero()).unwrap_or(Dir3::Y),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (add_planet_rotation, rotate_planets).chain().in_set(FixedUpdateSet::Main),
    )
    .register_type::<PlanetRotation>();

    make_persistent::<PlanetRotation>(app);
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmos_core::structure::dynamic_structure::DynamicStructure;

    #[test]
    fn repeated_planet_rotation_preserves_entity_offset_and_unit_rotations() {
        let mut app = App::new();
        let mut time = Time::<()>::default();
        time.advance_by(Duration::from_secs_f64(1.0 / 60.0));
        app.insert_resource(time).add_systems(FixedUpdate, rotate_planets);
        let rotation = Quat::from_euler(EulerRot::XYZ, 0.4, -0.7, 1.2);
        let local_position = Vec3::new(15.0, 50.0, 10.0).as_dvec3();
        let planet = app
            .world_mut()
            .spawn((
                Planet::new(300.0),
                Structure::Dynamic(DynamicStructure::new(16)),
                PlanetRotation {
                    axis: Dir3::new(Vec3::new(0.3, 0.7, 0.2)).unwrap(),
                    duration_per_revolution: Duration::from_secs(10),
                },
                Location::ZERO,
                Transform::from_rotation(Quat::from_array(rotation.to_array().map(|value| value * 1.00001))),
            ))
            .id();
        let entity = app
            .world_mut()
            .spawn((
                Location::ZERO + (rotation.as_dquat().normalize() * local_position).as_vec3(),
                Transform::from_rotation(rotation),
            ))
            .id();

        for _ in 0..3600 {
            app.world_mut().run_schedule(FixedUpdate);
            let rotation = app.world().get::<Transform>(planet).unwrap().rotation;
            let location = app.world().get::<Location>(entity).unwrap();
            let transform = app.world().get::<Transform>(entity).unwrap();
            assert!((rotation.length() - 1.0).abs() < 1e-6);
            assert!((transform.rotation.length() - 1.0).abs() < 1e-6);
            assert!(location.local.as_dvec3().distance(rotation.as_dquat().normalize() * local_position) < 0.002);
        }
    }
}
