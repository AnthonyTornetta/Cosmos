//! Represents all the energy stored on a structure

use bevy::{
    app::Update,
    ecs::entity::Entity,
    math::{Quat, Vec3},
    prelude::{App, Component},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    ecs::name,
    netty::sync::{IdentifiableComponent, SyncableComponent, sync_component},
    structure::coordinates::BlockCoordinate,
};

use super::{StructureSystemImpl, sync::SyncableSystem};

#[derive(Component, Default, Reflect, Serialize, Deserialize, Debug)]
/// Represents the energy storage of a structure
pub struct DockSystem {
    docking_blocks: Vec<BlockCoordinate>,
}

#[derive(Component, Debug, Serialize, Deserialize, Clone, PartialEq)]
/// If a structure is docked to another, it will have this component
pub struct Docked {
    /// The entity this is docked to
    pub to: Entity,
    /// The block on the entity it is docked to that acts as the docking block
    pub to_block: BlockCoordinate,
    /// The block on this entity that acts as the docking block
    pub this_block: BlockCoordinate,

    /// Relative to entity we are docked to
    pub relative_rotation: Quat,
    /// Relative translation to the entity we are docked to
    pub relative_translation: Vec3,
}

impl IdentifiableComponent for Docked {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:docked"
    }
}

impl SyncableComponent for Docked {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(mut self, _mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        self.to = _mapping.client_from_server(&self.to)?;
        Some(self)
    }
}

impl SyncableSystem for DockSystem {}

impl StructureSystemImpl for DockSystem {
    fn unlocalized_name() -> &'static str {
        "cosmos:dock_system"
    }
}

impl DockSystem {
    /// Call this whenever a block is added to the system
    pub fn block_added(&mut self, location: BlockCoordinate) {
        self.docking_blocks.push(location)
    }

    /// Call this whenever a block is removed from the system
    pub fn block_removed(&mut self, location: BlockCoordinate) {
        let Some((idx, _)) = self.docking_blocks.iter().enumerate().find(|(_, x)| **x == location) else {
            return;
        };

        self.docking_blocks.remove(idx);
    }

    /// Returns all the camera locations
    pub fn block_locations(&self) -> &[BlockCoordinate] {
        self.docking_blocks.as_slice()
    }

    /// Returns true if this system has no valid docking blocks
    pub fn is_empty(&self) -> bool {
        self.docking_blocks.is_empty()
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Docked>(app);

    app.register_type::<DockSystem>()
        .add_systems(Update, name::<DockSystem>("Dock System"));
}
