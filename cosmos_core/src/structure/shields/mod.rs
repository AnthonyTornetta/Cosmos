use bevy::{
    app::{App, PostUpdate},
    ecs::{
        component::Component,
        entity::Entity,
        query::Added,
        system::{Commands, Query},
    },
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, SyncableComponent};

use super::shared::DespawnWithStructure;

#[derive(Component, Reflect, Clone, Debug, Serialize, Deserialize)]
pub struct Shield {
    pub radius: f32,
    pub strength: f32,
    pub max_strength: f32,
}

impl SyncableComponent for Shield {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:shield"
    }

    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

fn on_add_shield(q_added_shield: Query<Entity, Added<Shield>>, mut commands: Commands) {
    for ent in q_added_shield.iter() {
        commands.entity(ent).insert(DespawnWithStructure);
    }
}

pub(super) fn register(app: &mut App) {
    sync_component::<Shield>(app);

    app.add_systems(PostUpdate, on_add_shield);

    app.register_type::<Shield>();
}
