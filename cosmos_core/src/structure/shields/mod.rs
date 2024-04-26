use bevy::{app::App, ecs::component::Component, reflect::Reflect};
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, SyncableComponent};

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

pub(super) fn register(app: &mut App) {
    sync_component::<Shield>(app);

    app.register_type::<Shield>();
}
