//! Utilities for representing data as children for entities

use bevy::prelude::*;

use crate::netty::sync::IdentifiableComponent;

#[derive(Component)]
#[relationship_target(relationship = DataFor, linked_spawn)]
/// Contains a list of all entities that have data for this entity
///
/// NOTE: These entities will all also be children
pub struct DataEntities(Vec<Entity>);

impl DataEntities {
    /// Iterates over all entities that are [`DataFor`] this entity.
    pub fn iter(&self) -> impl Iterator<Item = Entity> {
        self.0.iter().copied()
    }
}

#[derive(Component, Reflect)]
#[relationship(relationship_target = DataEntities)]
/// This entity is data for this entity
/// This entity will be automatically saved when the parent is saved. Data entities can have
/// sub-data entities, and they will still be properly saved + loaded.
///
/// WARNING: The entity MUST be the parent or everything breaks. This may be changed in the future.
pub struct DataFor(pub Entity);
impl IdentifiableComponent for DataFor {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:data_for"
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<DataFor>();
}
