use bevy::prelude::*;

use crate::netty::sync::IdentifiableComponent;

#[derive(Component)]
#[relationship_target(relationship = DataFor, linked_spawn)]
pub struct DataEntities(Vec<Entity>);

impl DataEntities {
    pub fn iter(&self) -> impl Iterator<Item = Entity> {
        self.0.iter().copied()
    }
}

#[derive(Component, Reflect)]
#[relationship(relationship_target = DataEntities)]
pub struct DataFor(pub Entity);
impl IdentifiableComponent for DataFor {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:data_for"
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<DataFor>();
}
