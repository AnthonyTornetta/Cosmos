//! Rotates planets
//!
//! TODO: Planets will not rotate if they are unloaded. To fix this, we will need some sort of
//! global total playtime in the server, and base the rotation off that.

use std::{f32::consts::TAU, time::Duration};

use bevy::{
    app::FixedUpdate,
    math::{Dir3, Quat, Vec3},
    prelude::{App, Commands, Component, Entity, IntoSystemConfigs, Query, Res, Transform, With, Without},
    reflect::Reflect,
    time::Time,
};
use cosmos_core::{
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    prelude::{Planet, StructureTypeSet},
};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::{
    init::init_world::ServerSeed,
    persistence::make_persistent::{make_persistent, PersistentComponent},
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

impl PersistentComponent for PlanetRotation {}

fn rotate_planets(mut q_planets: Query<(&PlanetRotation, &mut Transform), With<Planet>>, time: Res<Time>) {
    for (planet_rotation, mut transform) in q_planets.iter_mut() {
        transform.rotation *= Quat::from_axis_angle(
            *planet_rotation.axis,
            TAU * time.delta_seconds() / planet_rotation.duration_per_revolution.as_secs_f32(),
        );
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
            duration_per_revolution: Duration::from_mins(rng.gen_range(20..=60)),
            axis: Dir3::new(Vec3::new(rng.gen(), rng.gen(), rng.gen()).normalize_or_zero()).unwrap_or(Dir3::Y),
        });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (add_planet_rotation, rotate_planets)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .in_set(StructureTypeSet::Planet),
    )
    .register_type::<PlanetRotation>();

    make_persistent::<PlanetRotation>(app);
}
