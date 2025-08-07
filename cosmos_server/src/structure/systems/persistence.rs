use bevy::{platform::collections::HashMap, prelude::*};
use cosmos_core::{
    entities::EntityId,
    netty::sync::IdentifiableComponent,
    prelude::{StructureSystem, StructureSystems},
    structure::systems::{StructureSystemId, StructureSystemOrdering, StructureSystemTypeId},
};
use serde::{Deserialize, Serialize};

use crate::persistence::make_persistent::{DefaultPersistentComponent, PersistentComponent, make_persistent};

#[derive(Debug, Reflect, Serialize, serde::Deserialize, Component)]
/// Stores all the systems a structure has
pub struct SerializedStructureSystems {
    /// These entities should have the `StructureSystem` component
    systems: Vec<StructureSystemId>,
    activatable_systems: Vec<StructureSystemId>,
    /// The system ids
    ids: HashMap<StructureSystemId, EntityId>,
}

impl IdentifiableComponent for SerializedStructureSystems {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:structure_systems"
    }
}

impl PersistentComponent for StructureSystems {
    type SaveType = SerializedStructureSystems;

    fn initialize(&mut self, self_entity: Entity, _commands: &mut Commands) {
        self.set_entity(self_entity)
    }

    fn convert_to_save_type<'a>(
        &'a self,
        q_entity_ids: &Query<&EntityId>,
    ) -> Option<cosmos_core::utils::ownership::MaybeOwned<'a, Self::SaveType>> {
        Some(
            SerializedStructureSystems {
                activatable_systems: self.activatable_systems().iter().copied().collect::<Vec<_>>(),
                systems: self.systems().iter().copied().collect::<Vec<_>>(),
                ids: self
                    .ids()
                    .iter()
                    .flat_map(|(id, e)| q_entity_ids.get(*e).map(|e| (*id, *e)))
                    .collect::<HashMap<StructureSystemId, EntityId>>(),
            }
            .into(),
        )
    }

    fn convert_from_save_type(
        serialized_version: Self::SaveType,
        entity_id_manager: &crate::persistence::make_persistent::EntityIdManager,
    ) -> Option<Self> {
        Some(StructureSystems::new_from_raw(
            serialized_version.systems,
            serialized_version.activatable_systems,
            serialized_version
                .ids
                .into_iter()
                .flat_map(|(id, eid)| entity_id_manager.entity_from_entity_id(&eid).map(|e| (id, e)))
                .collect::<HashMap<StructureSystemId, Entity>>(),
        ))
    }
}

impl DefaultPersistentComponent for StructureSystemOrdering {}

#[derive(Serialize, Deserialize)]
pub struct SerializedStructureSystem {
    structure_entity: EntityId,
    system_id: StructureSystemId,
    system_type_id: StructureSystemTypeId,
}

impl PersistentComponent for StructureSystem {
    type SaveType = SerializedStructureSystem;

    fn convert_to_save_type<'a>(
        &'a self,
        q_entity_ids: &Query<&EntityId>,
    ) -> Option<cosmos_core::utils::ownership::MaybeOwned<'a, Self::SaveType>> {
        q_entity_ids
            .get(self.structure_entity())
            .map(|&e| {
                SerializedStructureSystem {
                    system_id: self.id(),
                    structure_entity: e,
                    system_type_id: self.system_type_id(),
                }
                .into()
            })
            .ok()
    }

    fn convert_from_save_type(
        serialized: Self::SaveType,
        entity_id_manager: &crate::persistence::make_persistent::EntityIdManager,
    ) -> Option<Self> {
        entity_id_manager
            .entity_from_entity_id(&serialized.structure_entity)
            .map(|e| Self::from_raw(e, serialized.system_id, serialized.system_type_id))
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<StructureSystemOrdering>(app);
    make_persistent::<StructureSystems>(app);
    make_persistent::<StructureSystem>(app);
}
