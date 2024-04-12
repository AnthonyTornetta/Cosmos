//! Syncing components across the network between the client & server.
//!
//! See [`sync_component`]

use bevy::{
    app::App,
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader},
        system::{Commands, Res, ResMut},
    },
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};

use crate::{
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::systems::StructureSystemId,
};

#[derive(Debug, Serialize, Deserialize)]
enum ComponentReplicationMessage {
    ComponentReplication {
        component_id: u16,
        entity_identifier: ComponentEntityIdentifier,
        raw_data: Vec<u8>,
    },
}

#[cfg(feature = "client")]
pub mod mapping;

#[cfg(feature = "client")]
mod client_syncing;
#[cfg(feature = "server")]
mod server_syncing;

#[derive(Clone, Serialize, Deserialize, Debug)]
/// Internally used but public because I'm bad
///
/// Don't mess w/ this
///
/// Links the numeric Id of a component to its unlocalized name
pub struct SyncedComponentId {
    id: u16,
    unlocalized_name: String,
}

impl Identifiable for SyncedComponentId {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
/// The type of syncing that should be used for this component
pub enum SyncType {
    /// Server will sync changes to the clients, and the client will not sync this
    /// with the server.
    ServerAuthoritative,
    /// Client will sync this with the server, and the server will not sync this
    /// with the client.
    ClientAuthoritative(ClientAuthority),
    // Both the server and client will sync each other on changes.
    // This is commented out because I don't have anything to use this on, and logic
    // will have to be added to prevent the server + client from repeatedly detecting
    // changes.
    // BothAuthoritative,
}

/// Clients can rarely (if ever) sync components that belong to anything.
///
/// They normally have to have some sort of authority over it, and this enforces that.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum ClientAuthority {
    /// The server will not check if the player has any rights to change this component.
    ///
    /// It will just accept whatever the client gives it
    Anything,
    /// The server will only accept this change if the client is piloting the entity they are changing
    Piloting,
    /// The server will only accept changes to this component if it's on their player
    Themselves,
}

#[derive(Debug, Serialize, Deserialize)]
enum ComponentEntityIdentifier {
    Entity(Entity),
    StructureSystem { structure_entity: Entity, id: StructureSystemId },
}

#[derive(Debug, Clone, Copy, Component)]
/// Represents how to find this entity for the syncing system.
///
/// This should be handled automatically.
pub(crate) enum SyncableEntity {
    StructureSystem,
}

/// Indicates that a component can be synchronized either from `Server -> Client` or `Client -> Server`.
///
/// Used in conjunction with [`sync_component`]
pub trait SyncableComponent: Component + Serialize + DeserializeOwned {
    /// Returns how this component should be synced
    fn get_sync_type() -> SyncType;
    /// Returns an unlocalized name that should be unique to this component.
    ///
    /// A good practice is to use `mod_id:component_name` format. For example, `cosmos:missile_focused`
    fn get_component_unlocalized_name() -> &'static str;
}

#[derive(Event, Debug)]
struct GotComponentToSyncEvent {
    component_id: u16,
    entity: Entity,
    raw_data: Vec<u8>,
}

fn deserialize_component<T: SyncableComponent>(
    components_registry: Res<Registry<SyncedComponentId>>,
    mut ev_reader: EventReader<GotComponentToSyncEvent>,
    mut commands: Commands,
) {
    for ev in ev_reader.read() {
        let synced_id = components_registry
            .try_from_numeric_id(ev.component_id)
            .unwrap_or_else(|| panic!("Missing component with id {}", ev.component_id));

        if T::get_component_unlocalized_name() != synced_id.unlocalized_name {
            continue;
        }

        commands
            .entity(ev.entity)
            .try_insert(bincode::deserialize::<T>(&ev.raw_data).expect("Failed to deserialize component sent from server!"));
    }
}

fn register_component<T: SyncableComponent>(mut registry: ResMut<Registry<SyncedComponentId>>) {
    registry.register(SyncedComponentId {
        unlocalized_name: T::get_component_unlocalized_name().to_owned(),
        id: 0,
    });
}

/// Indicates that a component should be synced across the client and the server.
///
/// Make sure to call this in either the core project or both the client & server projects.
pub fn sync_component<T: SyncableComponent>(_app: &mut App) {
    // LSP thinks `_app` is unused, even though it is, thus the underscore.

    #[cfg(not(feature = "client"))]
    #[cfg(not(feature = "server"))]
    compile_error!("You must enable one of the features. Either client or server.");

    #[cfg(feature = "client")]
    #[cfg(not(feature = "server"))]
    client_syncing::sync_component_client::<T>(_app);

    #[cfg(feature = "server")]
    #[cfg(not(feature = "client"))]
    server_syncing::sync_component_server::<T>(_app);
}

pub(super) fn register(app: &mut App) {
    create_registry::<SyncedComponentId>(app, "cosmos:syncable_components");

    app.add_event::<GotComponentToSyncEvent>();

    #[cfg(feature = "client")]
    client_syncing::setup_client(app);

    #[cfg(feature = "server")]
    server_syncing::setup_server(app);
}
