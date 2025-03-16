//! Ship -> Ship communication

use bevy::prelude::*;
use serde::{Deserialize, Serialize};

use crate::netty::sync::{sync_component, IdentifiableComponent, SyncableComponent};

pub mod events;
mod systems;

#[derive(Serialize, Deserialize, Debug, Clone, Component, Reflect, PartialEq, Eq)]
pub struct ComsChannel {
    pub messages: Vec<ComsMessage>,
    pub with: Entity,
}

impl IdentifiableComponent for ComsChannel {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:coms_channel"
    }
}

impl SyncableComponent for ComsChannel {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        mapping.client_from_server(&self.with).map(|with| Self {
            messages: self.messages,
            with,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
pub struct ComsMessage {
    pub text: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, Component, Reflect, PartialEq)]
pub struct RequestedComs {
    pub from: Entity,
    pub time: f32,
}

pub(super) fn register(app: &mut App) {
    events::register(app);
    systems::register(app);

    sync_component::<ComsChannel>(app);

    app.register_type::<ComsChannel>();
}
