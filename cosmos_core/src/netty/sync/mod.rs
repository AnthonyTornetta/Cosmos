//! Syncing components across the network between the client & server.
//!
//! See [`sync_component`]

use bevy::{
    app::{App, Startup},
    ecs::{component::Component, entity::Entity, message::Message, schedule::SystemSet},
    prelude::States,
    state::state::FreelyMutableState,
};
use bevy_renet::renet::ClientId;
use registry::{RegistrySyncInit, sync_registry};
use serde::{Deserialize, Serialize, de::DeserializeOwned};

use crate::{
    block::data::BlockDataIdentifier,
    registry::{create_registry, identifiable::Identifiable},
    structure::systems::StructureSystemId,
};

#[derive(Debug, Serialize, Deserialize, Clone)]
/// Data that represents a component and what entity it belongs to
///
/// Make sure to use bincode to serialize and deserialize the [`Self::raw_data`] field.
pub struct ReplicatedComponentData {
    /// How this entity should be identified.
    ///
    /// This is kinda ugly, and we should try not to continue packing more stuff into this.
    pub entity_identifier: ComponentEntityIdentifier,
    /// This is encoded via bincode, not cosmos_encoder.
    pub raw_data: Vec<u8>,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
/// Used by the server and synced to the client, allowing the server to communicate the exact type
/// of component is being synced.
pub enum ComponentId {
    /// The ID given to this component after registration in [`sync_component`].
    Custom(u16),
    /// This is bevy's [`ChildOf`] component.
    ChildOf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
/// A message sent from either the client/server to the other (depending on syncing authority) to
/// sync a component to the other
pub enum ComponentReplicationMessage {
    /// This component should be replicated
    ComponentReplication {
        /// The type of component you are talking about.
        component_id: ComponentId,
        /// A list of all replicated components and their data. The list is done to send multiple
        /// at a time, which is much faster to do than sending each individually
        replicated: Vec<ReplicatedComponentData>,
    },
    /// This component should be removed
    ///
    /// Removed components will NOT be synced if the entity is despawned.
    RemovedComponent {
        /// The type of component you are talking about.
        component_id: ComponentId,
        /// The entity this component was a part of
        entity_identifier: ComponentEntityIdentifier,
    },
}

#[cfg(feature = "client")]
pub mod mapping;

#[cfg(feature = "server")]
pub mod server_entity_syncing;

#[cfg(feature = "client")]
pub mod client_syncing;
#[cfg(feature = "server")]
pub mod server_syncing;

/// Messages that are synced from server->client and client->server.
pub mod events;
/// Syncing of registries from server -> client
pub mod registry;
/// Syncing of resources from server -> client
pub mod resources;

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

impl SyncType {
    /// Returns true if the client dicates the value of this component.
    ///
    /// This will also return true if BOTH the client and server dictate the value of this
    /// component
    pub fn is_client_authoritative(&self) -> bool {
        !matches!(self, Self::ServerAuthoritative)
    }

    /// Returns true if the server dicates the value of this component.
    ///
    /// This will also return true if BOTH the client and server dictate the value of this
    /// component
    pub fn is_server_authoritative(&self) -> bool {
        !matches!(self, Self::ClientAuthoritative(_))
    }
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

#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
/// Used to identify an entity sent from the client/server.
pub enum ComponentEntityIdentifier {
    /// Just a normal entity
    Entity(Entity),
    /// This entity is a structure system
    StructureSystem {
        /// The structure this system is a part of
        structure_entity: Entity,
        /// The system's ID within the structure
        id: StructureSystemId,
    },
    /// This entity represents data for an itemstack
    ItemData {
        /// The inventory the ItemStack is in
        inventory_entity: Entity,
        /// The slot the ItemStack is in
        item_slot: u32,
        /// The server's entity that represents this ItemStack's data
        server_data_entity: Entity,
    },
    /// This entity represents data for a block
    BlockData {
        /// The identifier for the BlockData
        identifier: BlockDataIdentifier,
        /// The server's entity that represents this block's data
        server_data_entity: Entity,
    },
}

/// Used to uniquely identify components
pub trait IdentifiableComponent: Component {
    /// This string should be a unique identifier for the component, using the `modid:name` format.
    ///
    /// For example, `cosmos:missile_focused`.
    fn get_component_unlocalized_name() -> &'static str;
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
pub trait SyncableComponent: Serialize + DeserializeOwned + Clone + std::fmt::Debug + PartialEq + IdentifiableComponent {
    /// Returns how this component should be synced
    ///
    /// Either from `server -> client` or `client -> server`.
    fn get_sync_type() -> SyncType;

    /// Set this to true to see when this component is synced over the network in the console as
    /// `info` prints.
    fn debug() -> bool {
        false
    }

    /// Returns true if this is a valid instance of this component, false if this should be ignored
    fn validate(&self) -> bool {
        true
    }

    /// If this returns true, this will be synced with the server.
    #[cfg(feature = "client")]
    fn should_send_to_server(&self, _mapping: &crate::netty::sync::mapping::NetworkMapping) -> bool {
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

#[derive(Message, Debug)]
/// When a component needs to be synced with this instance of the game, this event will be sent.
pub struct GotComponentToSyncMessage {
    #[allow(dead_code)]
    /// **Server**: The client that is trying to sync this component with you.
    /// **Client**: This is unused and is meaningless.
    pub client_id: ClientId,
    /// The ID of the component that is being synced
    pub component_id: ComponentId,
    /// The entity that should get this component
    pub entity: Entity,
    /// The entity authority should be checked against - not the entity being targetted.
    #[allow(dead_code)]
    /// **Server**: The entity that should be checked for authority before applying these changes.
    /// **Client**: This is unused and is meaningless
    pub authority_entity: Entity,
    /// The raw data of the component (decode using [`bincode::deserialize`])
    pub raw_data: Vec<u8>,
}

#[derive(Message, Debug)]
/// A component should be removed from the specified entity. On the server, the
/// [`Self::authority_entity`] should be checked for authority first in addition to any other
/// checks required.
pub struct GotComponentToRemoveMessage {
    #[allow(dead_code)]
    /// *Server*: The client ID that removed this component.
    /// *Client*: On client this is unused
    pub client_id: ClientId,
    /// The unique identifier for this component, for use with [`Registry<SyncedComponentId>`]
    pub component_id: ComponentId,
    /// The entity that used to have this component
    pub entity: Entity,
    /// The entity authority should be checked against - not the entity being targetted.
    #[allow(dead_code)]
    /// *Server*: The entity that should be checked for authority over this component.
    /// *Client*: On client this is unused
    pub authority_entity: Entity,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum RegisterComponentSet {
    RegisterComponent,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Reads component data from incoming data and upates component data locally.
pub enum ComponentSyncingSet {
    /// Receives component networking requests from the other side
    ReceiveComponents,

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
pub fn sync_component<T: SyncableComponent>(app: &mut App) {
    #[cfg(not(feature = "client"))]
    #[cfg(not(feature = "server"))]
    compile_error!("You must enable one of the features. Either client or server.");

    #[cfg(feature = "client")]
    client_syncing::sync_component_client::<T>(app);

    #[cfg(feature = "server")]
    server_syncing::sync_component_server::<T>(app);
}

pub(super) fn register<T: States + Clone + Copy + FreelyMutableState>(app: &mut App, registry_syncing: RegistrySyncInit<T>) {
    create_registry::<SyncedComponentId>(app, "cosmos:syncable_components");
    sync_registry::<SyncedComponentId>(app);
    registry::register(app, registry_syncing);
    resources::register(app);
    events::register(app);

    app.add_message::<GotComponentToSyncMessage>()
        .add_message::<GotComponentToRemoveMessage>();

    app.configure_sets(Startup, RegisterComponentSet::RegisterComponent);

    #[cfg(feature = "client")]
    {
        client_syncing::setup_client(app);
    }

    #[cfg(feature = "server")]
    {
        server_syncing::setup_server(app);
    }
}
