//! Used to build a planet

use bevy::prelude::*;
use bevy_rapier3d::prelude::RigidBody;

use crate::{
    persistence::LoadingDistance,
    physics::{gravity_system::GravityEmitter, location::Location},
    structure::{
        Structure,
        loading::StructureLoadingSet,
        planet::{PLANET_LOAD_RADIUS, PLANET_UNLOAD_RADIUS},
    },
};

use super::Planet;

fn on_add_planet(
    query: Query<(Entity, &Structure, &Location), Or<(Added<Planet>, (With<Planet>, Added<Location>))>>,
    mut commands: Commands,
) {
    for (entity, structure, loc) in query.iter() {
        let Structure::Dynamic(planet) = structure else {
            panic!("Planet must be dynamic structure type!");
        };

        commands.entity(entity).insert((
            RigidBody::Fixed,
            GravityEmitter {
                force_per_kg: 9.8,
                radius: planet.block_dimensions() as f32 / 2.0,
            },
            Name::new(format!("Planet @ {}", loc.sector())),
            LoadingDistance::new(PLANET_LOAD_RADIUS, PLANET_UNLOAD_RADIUS),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(FixedUpdate, on_add_planet.in_set(StructureLoadingSet::AddStructureComponents));
}
