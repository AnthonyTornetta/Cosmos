//! Rotates planets
//!
//! TODO: Planets will not rotate if they are unloaded. To fix this, we will need some sort of
//! global total playtime in the server, and base the rotation off that.

use std::{f32::consts::TAU, time::Duration};

use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    netty::sync::IdentifiableComponent,
    physics::location::Location,
    prelude::{Planet, Structure},
};
use rand::Rng;
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
            Quat::from_axis_angle(
                *planet_rotation.axis,
                TAU * time.delta_secs() / planet_rotation.duration_per_revolution.as_secs_f32(),
            )
        } else {
            Quat::IDENTITY
        };

        transform.rotation = delta_rot * transform.rotation;

        for (mut trans, mut loc) in q_everything_else
            .iter_mut()
            .filter(|x| within_rotation_range(structure, planet_loc, &x.1))
        {
            trans.rotation = delta_rot * trans.rotation;
            let cur_loc = *loc;
            loc.set_from(&(*planet_loc + delta_rot * Vec3::from(cur_loc - *planet_loc)));
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
