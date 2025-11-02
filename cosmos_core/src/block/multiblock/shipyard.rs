//! The shipyard multiblock logic

use crate::{
    block::{data::BlockData, multiblock::rectangle::RectangleMultiblockBounds},
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
        sync_component,
    },
    prelude::{BlockCoordinate, StructureBlock},
    structure::chunk::BlockInfo,
};
use bevy::{ecs::component::HookContext, platform::collections::HashMap, prelude::*};
use serde::{Deserialize, Serialize};

#[derive(Debug, Component, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone)]
/// A place used to assemble ships
pub struct Shipyard {
    controller: BlockCoordinate,
    bounds: RectangleMultiblockBounds,
}

impl Shipyard {
    /// Creates a new shipyard based on these conditions
    pub fn new(bounds: RectangleMultiblockBounds, controller: BlockCoordinate) -> Self {
        Self { bounds, controller }
    }

    /// Checks if this block coordinate is within the bounds of this shipyard (including the frame)
    pub fn coordinate_within(&self, coord: BlockCoordinate) -> bool {
        coord.within(self.bounds.negative_coords, self.bounds.positive_coords) || coord == self.controller
    }

    /// Returns the coordinate of this shipyard
    pub fn controller(&self) -> BlockCoordinate {
        self.controller
    }

    /// Returns the bounds of this shipyard (including frame)
    pub fn bounds(&self) -> RectangleMultiblockBounds {
        self.bounds
    }
}

impl IdentifiableComponent for Shipyard {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:shipyard"
    }
}

impl SyncableComponent for Shipyard {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Debug, Component, Reflect)]
/// Contains a list of all [`Shipyard`]s this structure has
pub struct Shipyards(Vec<Entity>);

impl Shipyards {
    /// Iterates over all the [`Shipyard`]s this structure has
    pub fn iter(&self) -> impl Iterator<Item = Entity> {
        self.0.iter().copied()
    }
}

#[derive(Debug, Reflect, Serialize, Deserialize, Clone)]
/// A shipyard is creating a blueprint
pub struct ShipyardDoingBlueprint {
    /// The blocks remaining to be placed
    pub blocks_todo: Vec<(BlockCoordinate, u16, BlockInfo)>,
    /// The total blocks of that type left to place (block id, amount left)
    pub total_blocks_count: HashMap<u16, u32>,
    /// The structure we are creating
    pub creating: Entity,
}

#[derive(Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone)]
/// The version of [`ShipyardDoingBlueprint`] that the client needs to know about
///
/// This is automatically updated when [`ShipyardDoingBlueprint`] changes
pub struct ClientFriendlyShipyardDoingBlueprint {
    /// The remaining blocks we stil need to place
    pub remaining_blocks: HashMap<u16, u32>,
    /// The entity we are creating
    pub creating: Entity,
}

#[derive(Debug, Reflect, Component, Serialize, Deserialize)]
/// Represents the state a shipyard is in
pub enum ShipyardState {
    /// The shipyard is currently paused - storing data for `ShipyardState::Building` but not
    /// doing any of the building.
    Paused(ShipyardDoingBlueprint),
    /// The shipyard is currently trying to build a ship
    Building(ShipyardDoingBlueprint),
    /// The shipyard is currently removing the blocks of whatever ship is inside of its bounds
    Deconstructing(Entity),
}

impl ShipyardState {
    /// Returns this state as the version that should be sent to the client(s) that care
    pub fn as_client_friendly(&self) -> ClientFriendlyShipyardState {
        match self {
            Self::Paused(p) => ClientFriendlyShipyardState::Paused(ClientFriendlyShipyardDoingBlueprint {
                remaining_blocks: p.total_blocks_count.clone(),
                creating: p.creating,
            }),
            Self::Building(p) => ClientFriendlyShipyardState::Building(ClientFriendlyShipyardDoingBlueprint {
                remaining_blocks: p.total_blocks_count.clone(),
                creating: p.creating,
            }),
            Self::Deconstructing(p) => ClientFriendlyShipyardState::Deconstructing(*p),
        }
    }
}

#[derive(Debug, Reflect, Component, Serialize, Deserialize, PartialEq, Eq, Clone)]
/// A client-friendly version of the [`ShipyardState`]. This is sent to the client instead of
/// [`ShipyardState`].
pub enum ClientFriendlyShipyardState {
    /// See [`ShipyardState::Paused`]
    Paused(ClientFriendlyShipyardDoingBlueprint),
    /// See [`ShipyardState::Building`]
    Building(ClientFriendlyShipyardDoingBlueprint),
    /// See [`ShipyardState::Deconstructing`]
    Deconstructing(Entity),
}

impl IdentifiableComponent for ShipyardState {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:shipyard_state"
    }
}

impl IdentifiableComponent for ClientFriendlyShipyardState {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:client_shipyard_state"
    }
}

impl SyncableComponent for ClientFriendlyShipyardState {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        match self {
            ClientFriendlyShipyardState::Paused(d) => {
                let creating = mapping.client_from_server(&d.creating)?;
                Some(Self::Paused(ClientFriendlyShipyardDoingBlueprint {
                    creating,
                    remaining_blocks: d.remaining_blocks,
                }))
            }
            ClientFriendlyShipyardState::Building(d) => {
                let creating = mapping.client_from_server(&d.creating)?;
                Some(Self::Building(ClientFriendlyShipyardDoingBlueprint {
                    creating,
                    remaining_blocks: d.remaining_blocks,
                }))
            }
            ClientFriendlyShipyardState::Deconstructing(e) => {
                let entity = mapping.client_from_server(&e)?;
                Some(Self::Deconstructing(entity))
            }
        }
    }
}

#[derive(Debug, Message, Serialize, Deserialize, Clone, Copy)]
/// Client->Server to request setting a shipyards state.
pub enum ClientSetShipyardState {
    /// Sets the state to paused (if building - otherwise does nothing)
    Pause {
        /// The block that has the `cosmos:shipyard_controller`
        controller: StructureBlock,
    },
    /// Sets the state to stopped
    Stop {
        /// The block that has the `cosmos:shipyard_controller`
        controller: StructureBlock,
    },
    /// Sets state back to building (if paused - otherwise does nothing)
    Unpause {
        /// The block that has the `cosmos:shipyard_controller`
        controller: StructureBlock,
    },
    /// Sets the state to deconstructing (if not currently in any state - otherwise does nothing)
    Deconstruct {
        /// The block that has the `cosmos:shipyard_controller`
        controller: StructureBlock,
    },
}

impl ClientSetShipyardState {
    /// Returns the block the `cosmos:shipyard_controller` is at.
    pub fn controller(&self) -> StructureBlock {
        match *self {
            Self::Stop { controller } => controller,
            Self::Pause { controller } => controller,
            Self::Unpause { controller } => controller,
            Self::Deconstruct { controller } => controller,
        }
    }
}

impl IdentifiableMessage for ClientSetShipyardState {
    fn unlocalized_name() -> &'static str {
        "cosmos:set_shipyard_state"
    }
}

impl NettyMessage for ClientSetShipyardState {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.controller().map_to_server(mapping).ok().map(|controller| match self {
            Self::Stop { controller: _ } => Self::Stop { controller },
            Self::Pause { controller: _ } => Self::Pause { controller },
            Self::Unpause { controller: _ } => Self::Unpause { controller },
            Self::Deconstruct { controller: _ } => Self::Deconstruct { controller },
        })
    }
}

#[derive(Message, Debug, Serialize, Deserialize, Clone, Copy)]
/// Server->client
///
/// Triggers the client to display a shipyard's UI
pub struct ShowShipyardUi {
    /// The shipyard controller block
    pub shipyard_block: StructureBlock,
}
impl IdentifiableMessage for ShowShipyardUi {
    fn unlocalized_name() -> &'static str {
        "cosmos:show_shipyard_ui"
    }
}

impl NettyMessage for ShowShipyardUi {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.shipyard_block
            .map_to_client(mapping)
            .map(|shipyard_block| Self { shipyard_block })
            .ok()
    }
}

#[derive(Message, Debug, Serialize, Deserialize, Clone, Copy)]
/// Client->Server
///
/// Requests the server to set the shipyard's blueprint based on the given item in the player's
/// inventory (should be a `cosmos:blueprint`)
pub struct SetShipyardBlueprint {
    /// The shipyard controller's block coordinate
    pub shipyard_block: StructureBlock,
    /// The slot in the player's inventory the blueprint is at
    pub blueprint_slot: u32,
}

impl IdentifiableMessage for SetShipyardBlueprint {
    fn unlocalized_name() -> &'static str {
        "cosmos:set_shipyard_blueprint"
    }
}

impl NettyMessage for SetShipyardBlueprint {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_client_to_server(self, mapping: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.shipyard_block
            .map_to_server(mapping)
            .map(|shipyard_block| Self {
                shipyard_block,
                blueprint_slot: self.blueprint_slot,
            })
            .ok()
    }
}

fn register_shipyard_component_hooks(world: &mut World) {
    world
        .register_component_hooks::<Shipyard>()
        .on_add(|mut world, HookContext { entity, .. }| {
            let Some(block_data) = world.get::<BlockData>(entity) else {
                error!("Shipyard missing block data!");
                return;
            };
            let structure = block_data.identifier.block.structure();
            if let Some(mut shipyards) = world.get_mut::<Shipyards>(structure) {
                shipyards.0.push(entity);
            } else {
                world.commands().entity(structure).insert(Shipyards(vec![entity]));
            }
        })
        .on_remove(|mut world, HookContext { entity, .. }| {
            let Some(block_data) = world.get::<BlockData>(entity) else {
                error!("Shipyard missing block data!");
                return;
            };
            let structure = block_data.identifier.block.structure();
            if let Some(mut shipyards) = world.get_mut::<Shipyards>(structure)
                && let Some((idx, _)) = shipyards.0.iter().enumerate().find(|x| *x.1 == entity)
            {
                shipyards.0.swap_remove(idx);
            }
        });
}

pub(super) fn register(app: &mut App) {
    sync_component::<ClientFriendlyShipyardState>(app);
    sync_component::<Shipyard>(app);

    app.register_type::<Shipyard>()
        .register_type::<Shipyards>()
        .register_type::<ShipyardState>()
        .add_systems(Startup, register_shipyard_component_hooks)
        .add_netty_event::<ClientSetShipyardState>()
        .add_netty_event::<SetShipyardBlueprint>()
        .add_netty_event::<ShowShipyardUi>();
}
