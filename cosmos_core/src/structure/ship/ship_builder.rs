//! Used to build ships

use bevy::prelude::*;
use bevy_rapier3d::prelude::{ExternalImpulse, ReadMassProperties, RigidBody};

use crate::{
    persistence::{Blueprintable, LoadingDistance},
    structure::loading::StructureLoadingSet,
};

use super::{Ship, ship_movement::ShipMovement};

fn on_add_ship(query: Query<Entity, Added<Ship>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).insert((
            ShipMovement::default(),
            RigidBody::Dynamic,
            ReadMassProperties::default(),
            ExternalImpulse::default(),
            Blueprintable,
            LoadingDistance::new(6, 7),
            Name::new("Ship"),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_ship.before(StructureLoadingSet::LoadStructure));
}
