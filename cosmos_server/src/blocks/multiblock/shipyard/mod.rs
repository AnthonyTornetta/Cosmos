//! The shipyard multiblock logic

use bevy::{platform::collections::HashMap, prelude::*};
use cosmos_core::{
    block::multiblock::prelude::{Shipyard, ShipyardDoingBlueprint, ShipyardState},
    entities::EntityId,
    netty::sync::IdentifiableComponent,
    prelude::BlockCoordinate,
    structure::chunk::BlockInfo,
};
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{DefaultPersistentComponent, PersistentComponent, make_persistent};

mod impls;

#[derive(Component, Debug, Serialize, Deserialize)]
/// If a structure is being built by a shipyard, this component will be added
pub struct StructureBeingBuilt;

impl IdentifiableComponent for StructureBeingBuilt {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:structure_being_built"
    }
}

impl DefaultPersistentComponent for StructureBeingBuilt {}
impl DefaultPersistentComponent for Shipyard {}

#[derive(Debug, Reflect, Serialize, Deserialize)]
pub struct SerializedShipyardDoingBlueprint {
    pub blocks_todo: Vec<(BlockCoordinate, u16, BlockInfo)>,
    pub total_blocks_count: HashMap<u16, u32>,
    pub creating: EntityId,
}

#[derive(Debug, Reflect, Serialize, Deserialize)]
pub enum SerializedShipyardState {
    Paused(SerializedShipyardDoingBlueprint),
    Building(SerializedShipyardDoingBlueprint),
    Deconstructing(EntityId),
}

impl PersistentComponent for ShipyardState {
    type SaveType = SerializedShipyardState;

    fn convert_to_save_type<'a>(
        &'a self,
        q_entity_ids: &Query<&cosmos_core::entities::EntityId>,
    ) -> Option<cosmos_core::utils::ownership::MaybeOwned<'a, Self::SaveType>> {
        match self {
            Self::Deconstructing(e) => q_entity_ids.get(*e).ok().map(|&e| Self::SaveType::Deconstructing(e).into()),
            Self::Paused(d) => q_entity_ids.get(d.creating).ok().map(|&e| {
                Self::SaveType::Paused(SerializedShipyardDoingBlueprint {
                    blocks_todo: d.blocks_todo.clone(),
                    total_blocks_count: d.total_blocks_count.clone(),
                    creating: e,
                })
                .into()
            }),
            Self::Building(d) => q_entity_ids.get(d.creating).ok().map(|&e| {
                Self::SaveType::Building(SerializedShipyardDoingBlueprint {
                    blocks_todo: d.blocks_todo.clone(),
                    total_blocks_count: d.total_blocks_count.clone(),
                    creating: e,
                })
                .into()
            }),
        }
    }

    fn convert_from_save_type(
        save_type: Self::SaveType,
        entity_id_manager: &crate::persistence::make_persistent::EntityIdManager,
    ) -> Option<Self> {
        match save_type {
            SerializedShipyardState::Deconstructing(e) => entity_id_manager.entity_from_entity_id(&e).map(|e| Self::Deconstructing(e)),
            SerializedShipyardState::Paused(d) => entity_id_manager.entity_from_entity_id(&d.creating).map(|e| {
                Self::Paused(ShipyardDoingBlueprint {
                    blocks_todo: d.blocks_todo.clone(),
                    total_blocks_count: d.total_blocks_count.clone(),
                    creating: e,
                })
            }),
            SerializedShipyardState::Building(d) => entity_id_manager.entity_from_entity_id(&d.creating).map(|e| {
                Self::Building(ShipyardDoingBlueprint {
                    blocks_todo: d.blocks_todo.clone(),
                    total_blocks_count: d.total_blocks_count.clone(),
                    creating: e,
                })
            }),
        }
    }
}

pub(super) fn register(app: &mut App) {
    impls::register(app);

    make_persistent::<StructureBeingBuilt>(app);
    make_persistent::<Shipyard>(app);
    make_persistent::<ShipyardState>(app);
}
