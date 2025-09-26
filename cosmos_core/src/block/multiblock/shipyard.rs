//! The shipyard multiblock logic

use crate::{
    block::{data::BlockData, multiblock::rectangle::RectangleMultiblockBounds},
    item::usable::blueprint::BlueprintItemData,
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
        sync_component,
    },
    prelude::{BlockCoordinate, FullStructure, Structure, StructureBlock},
    structure::chunk::BlockInfo,
};
use bevy::{
    ecs::component::HookContext,
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
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

#[derive(Debug, Reflect, Serialize, Deserialize)]
pub struct ShipyardDoingBlueprint {
    pub blocks_todo: Vec<(BlockCoordinate, u16, BlockInfo)>,
    pub total_blocks_count: HashMap<u16, u32>,
    pub creating: Entity,
}

#[derive(Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct ClientFriendlyShipyardDoingBlueprint {
    pub need_items: HashMap<u16, u32>,
    pub creating: Entity,
}

#[derive(Debug, Reflect, Component, Serialize, Deserialize)]
pub enum ShipyardState {
    Paused(ShipyardDoingBlueprint),
    Building(ShipyardDoingBlueprint),
    Deconstructing(Entity),
}

impl ShipyardState {
    pub fn as_client_friendly(&self) -> ClientFriendlyShipyardState {
        match self {
            Self::Paused(p) => ClientFriendlyShipyardState::Paused(ClientFriendlyShipyardDoingBlueprint {
                need_items: p.total_blocks_count.clone(),
                creating: p.creating,
            }),
            Self::Building(p) => ClientFriendlyShipyardState::Building(ClientFriendlyShipyardDoingBlueprint {
                need_items: p.total_blocks_count.clone(),
                creating: p.creating,
            }),
            Self::Deconstructing(p) => ClientFriendlyShipyardState::Deconstructing(*p),
        }
    }
}

#[derive(Debug, Reflect, Component, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub enum ClientFriendlyShipyardState {
    Paused(ClientFriendlyShipyardDoingBlueprint),
    Building(ClientFriendlyShipyardDoingBlueprint),
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
}

#[derive(Debug, Event, Serialize, Deserialize, Clone, Copy)]
pub enum ClientSetShipyardState {
    Paused,
    Unpause,
    BuildFromItem { slot: u32 },
    Deconstruct,
}

impl IdentifiableEvent for ClientSetShipyardState {
    fn unlocalized_name() -> &'static str {
        "cosmos:set_shipyard_state"
    }
}

impl NettyEvent for ClientSetShipyardState {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Event, Debug, Serialize, Deserialize, Clone, Copy)]
pub struct ShowShipyardUi {
    pub shipyard_block: StructureBlock,
}

impl IdentifiableEvent for ShowShipyardUi {
    fn unlocalized_name() -> &'static str {
        "cosmos:show_shipyard_ui"
    }
}

impl NettyEvent for ShowShipyardUi {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
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

#[derive(Event, Debug, Serialize, Deserialize, Clone, Copy)]
pub struct SetShipyardBlueprint {
    pub shipyard_block: StructureBlock,
    pub blueprint_slot: u32,
}

impl IdentifiableEvent for SetShipyardBlueprint {
    fn unlocalized_name() -> &'static str {
        "cosmos:set_shipyard_blueprint"
    }
}

impl NettyEvent for SetShipyardBlueprint {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
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
