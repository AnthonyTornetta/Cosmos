//! If a player is locked to a ship (ie walking on it) then they will have this component.

use bevy::prelude::{
    App, Changed, Commands, Component, Entity, Parent, Query, RemovedComponents, With,
};

use crate::structure::ship::Ship;

use super::Player;

#[derive(Component, Debug)]
/// If a player is locked to a ship (ie walking on it) then they will have this component.
pub struct ApartOfShip {
    /// The ship they are locked to
    pub ship_entity: Entity,
}

fn add_or_remove_component(
    query: Query<(Entity, &Parent), (With<Player>, Changed<Parent>)>,
    has_apart_of_ship: Query<(), With<ApartOfShip>>,
    mut remove_components: RemovedComponents<Parent>,
    is_ship: Query<(), With<Ship>>,
    mut commands: Commands,
) {
    for entity in remove_components.iter() {
        if has_apart_of_ship.contains(entity) {
            if let Some(mut ecmds) = commands.get_entity(entity) {
                ecmds.remove::<ApartOfShip>();
            }
        }
    }

    for (entity, parent) in query.iter() {
        if is_ship.contains(parent.get()) {
            commands.entity(entity).insert(ApartOfShip {
                ship_entity: parent.get(),
            });
        } else {
            commands.entity(entity).remove::<ApartOfShip>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(add_or_remove_component);
}
