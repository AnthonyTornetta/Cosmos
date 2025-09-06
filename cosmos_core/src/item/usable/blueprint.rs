use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::netty::sync::{
    IdentifiableComponent, SyncableComponent,
    events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
    sync_component,
};

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Reflect)]
pub enum BlueprintType {
    Ship,
    Station,
}

impl BlueprintType {
    pub fn blueprint_directory(&self) -> &'static str {
        match self {
            Self::Ship => "ship",
            Self::Station => "station",
        }
    }

    pub fn path_for(&self, blueprint_name: &str) -> String {
        format!("blueprints/{}/{}.bp", self.blueprint_directory(), blueprint_name)
    }
}

#[derive(Component, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
pub struct BlueprintItemData {
    pub blueprint_id: Uuid,
    pub blueprint_type: BlueprintType,
    pub name: String,
}

impl IdentifiableComponent for BlueprintItemData {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:blueprint_item_data"
    }
}

impl SyncableComponent for BlueprintItemData {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
pub struct DownloadBlueprint {
    pub blueprint_id: Uuid,
    pub blueprint_type: BlueprintType,
}

impl IdentifiableEvent for DownloadBlueprint {
    fn unlocalized_name() -> &'static str {
        "cosmos:download_blueprint"
    }
}

impl NettyEvent for DownloadBlueprint {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
pub struct DownloadBlueprintResponse {
    pub blueprint_id: Uuid,
    pub blueprint_type: BlueprintType,
    pub data: Vec<u8>,
}

impl IdentifiableEvent for DownloadBlueprintResponse {
    fn unlocalized_name() -> &'static str {
        "cosmos:download_blueprint_response"
    }
}

impl NettyEvent for DownloadBlueprintResponse {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
pub struct UploadBlueprint {
    pub data: BlueprintData,
    pub name: String,
    /// The slot the player has a blueprint they want to set
    pub slot: u32,
}

impl IdentifiableEvent for UploadBlueprint {
    fn unlocalized_name() -> &'static str {
        "cosmos:upload_blueprint"
    }
}

impl NettyEvent for UploadBlueprint {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<BlueprintItemData>()
        .add_netty_event::<DownloadBlueprintResponse>()
        .add_netty_event::<UploadBlueprint>()
        .add_netty_event::<DownloadBlueprint>();

    sync_component::<BlueprintItemData>(app);
}
