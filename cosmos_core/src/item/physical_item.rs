//! Items that are thrown on the ground

use bevy::prelude::*;
use bevy_rapier3d::prelude::{Collider, ReadMassProperties, RigidBody};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::sets::FixedUpdateSet,
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
};

#[derive(Component, Reflect, Debug, Serialize, Deserialize, PartialEq, Eq, Clone)]
/// An item that is currently in the physical world (ie a dropped item)
pub struct PhysicalItem;

impl IdentifiableComponent for PhysicalItem {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:physical_item"
    }
}

impl SyncableComponent for PhysicalItem {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

fn on_add_physical_item(mut commands: Commands, q_added: Query<Entity, Added<PhysicalItem>>) {
    for ent in q_added.iter() {
        commands.entity(ent).insert((
            RigidBody::Dynamic,
            Collider::cuboid(0.1, 0.1, 0.1),
            ReadMassProperties::default(),
            Name::new("Physical Item"),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<PhysicalItem>(app);

    app.add_systems(FixedUpdate, on_add_physical_item.in_set(FixedUpdateSet::Main));
}
