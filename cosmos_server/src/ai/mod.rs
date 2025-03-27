//! basically agi

use bevy::{
    app::App,
    ecs::{
        component::Component,
        entity::Entity,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Commands, Query},
    },
};

use crate::persistence::{
    loading::{LoadingSystemSet, NeedsLoaded, LOADING_SCHEDULE},
    saving::{SavingSystemSet, SAVING_SCHEDULE},
    SerializedData,
};

mod combat;
mod hit_tracking;
mod pirate;
pub mod quest_npc;

#[derive(Component)]
/// This entity is controlled by NPCs
pub struct AiControlled;

fn on_save_ai_controlled(mut pirate_ai_query: Query<&mut SerializedData, With<AiControlled>>) {
    for mut serialized_data in pirate_ai_query.iter_mut() {
        serialized_data.serialize_data("cosmos:ai_controlled", &true);
    }
}

fn on_load_ai_controlled(mut commands: Commands, query: Query<(Entity, &SerializedData), With<NeedsLoaded>>) {
    for (entity, serialized_data) in query.iter() {
        if serialized_data.deserialize_data::<bool>("cosmos:ai_controlled").unwrap_or(false) {
            commands.entity(entity).insert(AiControlled);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(LOADING_SCHEDULE, on_load_ai_controlled.in_set(LoadingSystemSet::DoLoading));
    app.add_systems(SAVING_SCHEDULE, on_save_ai_controlled.in_set(SavingSystemSet::DoSaving));

    combat::register(app);
    pirate::register(app);
    quest_npc::register(app);
    hit_tracking::register(app);
}
