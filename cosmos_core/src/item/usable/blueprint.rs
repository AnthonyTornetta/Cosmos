//! A usable blueprint item

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
        sync_component,
    },
    structure::blueprint::{Blueprint, BlueprintAuthor, BlueprintType},
};

#[derive(Component, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
/// Is present on an item that points to a blueprint on disk
pub struct BlueprintItemData {
    /// The blueprint's unique id
    pub blueprint_id: Uuid,
    /// The type of blueprint this points to
    pub blueprint_type: BlueprintType,
    /// The display name of this blueprint (could be out of date)
    pub name: String,
    /// The author of this blueprint (could be out of date)
    pub author: BlueprintAuthor,
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

impl BlueprintItemData {
    /// Returns the relative path this blueprint would be saved to disk on
    pub fn get_blueprint_path(&self) -> String {
        self.blueprint_type.path_for(&self.blueprint_id.to_string())
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
/// client -> server - Client requests to download a blueprint of this id and type.
///
/// The server will check for validity + authorization
pub struct DownloadBlueprint {
    /// The blueprint's id
    pub blueprint_id: Uuid,
    /// The blueprint's type
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
/// A response to the [`DownloadBlueprint`] that contains the raw data for that blueprint
pub struct DownloadBlueprintResponse {
    /// The blueprint's id (from the [`DownloadBlueprint`] request)
    pub blueprint_id: Uuid,
    /// The blueprint's data
    pub blueprint: Blueprint,
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
/// client -> server - Uploads a blueprint to the server from the client's computer
pub struct UploadBlueprint {
    /// The client's blueprint data
    pub blueprint: Blueprint,
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

#[derive(Event, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
/// Clears the blueprint from this item. Does not delete the blueprint file, only clears the
/// reference to it.
pub struct ClearBlueprint {
    /// The slot to copy from
    pub slot: u32,
}

impl IdentifiableEvent for ClearBlueprint {
    fn unlocalized_name() -> &'static str {
        "cosmos:clear_blueprint"
    }
}

impl NettyEvent for ClearBlueprint {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
/// Copys this blueprint item into an empty slot
pub struct CopyBlueprint {
    /// The slot to copy from
    pub slot: u32,
}

impl IdentifiableEvent for CopyBlueprint {
    fn unlocalized_name() -> &'static str {
        "cosmos:copy_blueprint"
    }
}

impl NettyEvent for CopyBlueprint {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Serialize, Deserialize, Debug, Clone, Reflect, PartialEq, Eq)]
/// Loads the blueprint on top of the player if they are in creative
pub struct RequestLoadBlueprint {
    /// The slot the player has a blueprint they want to load
    pub slot: u32,
}

impl IdentifiableEvent for RequestLoadBlueprint {
    fn unlocalized_name() -> &'static str {
        "cosmos:load_blueprint"
    }
}

impl NettyEvent for RequestLoadBlueprint {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

pub(super) fn register(app: &mut App) {
    app.register_type::<BlueprintItemData>()
        .add_netty_event::<DownloadBlueprintResponse>()
        .add_netty_event::<RequestLoadBlueprint>()
        .add_netty_event::<UploadBlueprint>()
        .add_netty_event::<ClearBlueprint>()
        .add_netty_event::<CopyBlueprint>()
        .add_netty_event::<DownloadBlueprint>();

    sync_component::<BlueprintItemData>(app);
}
