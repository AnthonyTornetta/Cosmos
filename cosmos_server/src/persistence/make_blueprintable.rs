//! Streamlines the serialization & deserialization of blueprintable components

use bevy::prelude::*;
use cosmos_core::{entities::EntityId, structure::persistence::DeserializationError};

use crate::persistence::{
    loading::NeedsBlueprintLoaded,
    make_persistent::{EntityIdManager, PersistentComponent},
    saving::{BlueprintingSystemSet, NeedsBlueprinted},
};

use super::{
    SerializedData,
    loading::{LOADING_SCHEDULE, LoadingSystemSet},
    saving::SAVING_SCHEDULE,
};

fn save_component<T: PersistentComponent>(
    mut q_needs_saved: Query<(Entity, &mut SerializedData, &T), With<NeedsBlueprinted>>,
    q_entity_ids: Query<&EntityId>,
) {
    q_needs_saved.iter_mut().for_each(|(entity, mut serialized_data, component)| {
        let Some(save_type) = component.convert_to_save_type(&q_entity_ids) else {
            error!(
                "Unable to convert to entity id type for {} on entity {entity:?}. This component will not be saved.",
                T::get_component_unlocalized_name()
            );
            return;
        };

        serialized_data.serialize_data(T::get_component_unlocalized_name(), save_type.as_ref());
    });
}

fn load_component<T: PersistentComponent>(
    mut commands: Commands,
    q_needs_loaded: Query<(Entity, &SerializedData), With<NeedsBlueprintLoaded>>,
    entity_id_manager: EntityIdManager,
    q_name: Query<&Name>,
) {
    q_needs_loaded.iter().for_each(|(entity, serialized_data)| {
        let component_save_data = match serialized_data.deserialize_data::<T::SaveType>(T::get_component_unlocalized_name()) {
            Ok(data) => data,
            Err(DeserializationError::NoEntry) => return,
            Err(DeserializationError::ErrorParsing(e)) => {
                let id = q_name
                    .get(entity)
                    .map(|x| format!("{x} ({entity:?})"))
                    .unwrap_or_else(|_| format!("{entity:?}"));
                error!(
                    "Error deserializing component {} on entity {id}\n{e:?}.",
                    T::get_component_unlocalized_name()
                );
                return;
            }
        };

        let Some(mut component) = T::convert_from_save_type(component_save_data, &entity_id_manager) else {
            error!(
                "Error getting entities needed for component {} for entity {entity:?}",
                T::get_component_unlocalized_name()
            );
            return;
        };

        component.initialize(entity, &mut commands);
        commands.entity(entity).insert(component);
    });
}

/// Makes so that when an entity with an entity with this component is blueprinted, it will also be
/// blueprinted.
///
/// When this entity is loaded from a blueprint again, the component will also be loaded.
pub fn make_blueprintable<T: PersistentComponent>(app: &mut App) {
    app.add_systems(SAVING_SCHEDULE, save_component::<T>.in_set(BlueprintingSystemSet::DoBlueprinting))
        .add_systems(LOADING_SCHEDULE, load_component::<T>.in_set(LoadingSystemSet::LoadBasicComponents));
}
