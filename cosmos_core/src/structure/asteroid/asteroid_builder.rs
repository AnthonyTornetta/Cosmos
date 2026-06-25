//! Used to build an asteroid

use bevy::prelude::*;
use bevy_rapier3d::prelude::RigidBody;

use super::*;
use crate::{persistence::LoadingDistance, prelude::StructureLoadingSet};

fn add_rigidbody_to_asteroid(mut commands: Commands, q_asteroid_added: Query<(Entity, Has<MovingAsteroid>), Added<Asteroid>>) {
    for (ent, moving) in q_asteroid_added.iter() {
        let mut ecmds = commands.entity(ent);
        ecmds.insert((
            Name::new("Asteroid"),
            LoadingDistance::new(ASTEROID_LOAD_RADIUS, ASTEROID_UNLOAD_RADIUS),
        ));
        if moving {
            ecmds.insert(RigidBody::Dynamic);
        } else {
            ecmds.insert(RigidBody::KinematicVelocityBased);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        add_rigidbody_to_asteroid.in_set(StructureLoadingSet::AddStructureComponents),
    );
}
