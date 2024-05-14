//! Syncing components across the network between the client & server.
//!
//! See [`sync_component`]

use bevy::{
    app::App,
    ecs::{component::Component, entity::Entity, event::Event, schedule::SystemSet, system::ResMut},
};
use bevy_renet::renet::ClientId;
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
    /// *Server Authoritative Note:* Removed components will NOT be synced if the entity is despawned.
    RemovedComponent {
        component_id: u16,
        entity_identifier: ComponentEntityIdentifier,
    },
}

#[cfg(feature = "client")]
pub mod mapping;

#[cfg(feature = "server")]
pub mod server_entity_syncing;

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
    /// Both the server and client will sync each other on changes.
    BothAuthoritative(ClientAuthority),
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

/// Indicates that a component can be synchronized either from `Server -> Client` or `Client -> Server`.
///
/// Not that `clone()` is only called if the client is sending something to the server ([`SyncType::ClientAuthoritative`]) AND
/// [`SyncableComponent::needs_entity_conversion`] returns true.
///
/// Make sure to call [`sync_component`] for your component type if you want it synced.
///
/// Not that just because a component is syncable, doesn't mean it will be synced. The client must first be aware
/// of the entity before it will sync it.  This is most commonly done via the [`super::server_unreliable_messages::ServerUnreliableMessages::BulkBodies`] networking request.
/// Note that this requires the following components to sync the entity:
/// `Location`, `Transform`, `Velocity`, and `LoadingDistance`. Additionally, the player must be within the `LoadingDistance`.
pub trait SyncableComponent: Component + Serialize + DeserializeOwned + Clone + std::fmt::Debug + PartialEq {
    /// Returns how this component should be synced
    ///
    /// Either from `server -> client` or `client -> server`.
    fn get_sync_type() -> SyncType;
    /// Returns an unlocalized name that should be unique to this component.
    ///
    /// A good practice is to use `mod_id:component_name` format. For example, `cosmos:missile_focused`
    fn get_component_unlocalized_name() -> &'static str;

    /// Returns true if this is a valid instance of this component, false if this should be ignored
    fn validate(&self) -> bool {
        true
    }

    /// The [`SyncableComponent::convert_entities_client_to_server`] function requires cloning this struct,
    /// so to avoid clones on structs without any entities this method can be used.
    ///
    /// This only has to be implemented if this is sent from client to server.
    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        false
    }

    #[cfg(feature = "client")]
    /// Converts server-side entities to their client-side equivalent.
    ///
    /// Return None if this fails.
    fn convert_entities_server_to_client(self, _mapping: &self::mapping::NetworkMapping) -> Option<Self> {
        Some(self)
    }

    #[cfg(feature = "client")]
    /// Converts client-side entities to their server-side equivalent.
    ///
    /// Return None if this fails.
    fn convert_entities_client_to_server(&self, _mapping: &self::mapping::NetworkMapping) -> Option<Self> {
        Some(self.clone())
    }
}

#[derive(Event, Debug)]
struct GotComponentToSyncEvent {
    #[allow(dead_code)] // on client this is unused
    client_id: ClientId,
    component_id: u16,
    entity: Entity,
    /// The entity authority should be checked against - not the entity being targetted.
    #[allow(dead_code)] // on client this is unused
    authority_entity: Entity,
    raw_data: Vec<u8>,
}

#[derive(Event, Debug)]
struct GotComponentToRemoveEvent {
    #[allow(dead_code)] // on client this is unused
    client_id: ClientId,
    component_id: u16,
    entity: Entity,
    /// The entity authority should be checked against - not the entity being targetted.
    #[allow(dead_code)] // on client this is unused
    authority_entity: Entity,
}

fn register_component<T: SyncableComponent>(mut registry: ResMut<Registry<SyncedComponentId>>) {
    registry.register(SyncedComponentId {
        unlocalized_name: T::get_component_unlocalized_name().to_owned(),
        id: 0,
    });
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Reads component data from incoming data and upates component data locally.
pub enum ComponentSyncingSet {
    /// Process any data needed before components are synced here
    PreComponentSyncing,
    /// Reads component data from incoming data and upates component data locally.
    DoComponentSyncing,
    /// Process any data after components were synced here
    PostComponentSyncing,
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

    app.add_event::<GotComponentToSyncEvent>().add_event::<GotComponentToRemoveEvent>();

    #[cfg(feature = "client")]
    {
        client_syncing::setup_client(app);
    }

    #[cfg(feature = "server")]
    {
        server_syncing::setup_server(app);
    }
}
