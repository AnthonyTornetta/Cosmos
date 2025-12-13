//! Represents the logic behind the reactor multiblock system

use std::time::Duration;

use bevy::{
    prelude::{App, Component, Deref, DerefMut, Message},
    reflect::Reflect,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::Block,
    item::Item,
    netty::sync::{
        IdentifiableComponent, SyncableComponent,
        events::netty_event::{IdentifiableMessage, NettyMessage, SyncedMessageImpl},
        registry::sync_registry,
        sync_component,
    },
    prelude::StructureBlock,
    registry::{Registry, create_registry, identifiable::Identifiable},
    structure::coordinates::BlockCoordinate,
};

#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq)]
/// The inclusive bounds of a reactor, including its casing
pub struct ReactorBounds {
    /// Inclusive negative-most coordinates of a reactor (include casing)
    pub negative_coords: BlockCoordinate,
    /// Inclusive positive-most coordinates of a reactor (include casing)
    pub positive_coords: BlockCoordinate,
}

impl ReactorBounds {
    /// Computes the volume (in blocks) of this reactor including its casing.
    pub fn volume(&self) -> u32 {
        let diff = self.positive_coords - self.negative_coords;
        ((diff.x + 1) * (diff.y + 1) * (diff.z + 1)) as u32
    }
}

#[derive(Clone, Copy, Debug, Reflect, Serialize, Deserialize, PartialEq, Component)]
/// Represents a constructed reactor
pub struct Reactor {
    /// Represents the reactor_controller block
    pub controller: BlockCoordinate,
    /// Represents the power per second this reactor will generate, given 100% efficient fuel.
    /// Note that the fuel efficiency can effect the actual output of the reactor.
    pub power_per_second: f32,
    /// The size of this reactor
    pub bounds: ReactorBounds,
    /// How much of the fuel's base consumption is used when power is generated.
    pub fuel_consumption_multiplier: f32,
}

impl IdentifiableComponent for Reactor {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:reactor"
    }
}

impl SyncableComponent for Reactor {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Component, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, Debug, Reflect)]
/// If a reactor controller block has this component, the reactor is active.
///
/// A reactor may be active but have no fuel, in that case it will generate 0 power
pub struct ReactorActive;

impl IdentifiableComponent for ReactorActive {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:reactor_active"
    }
}

impl SyncableComponent for ReactorActive {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Component, Default, Clone, Serialize, Deserialize, PartialEq, Debug, Reflect)]
/// Stores how much of the current fuel has been consumed
pub struct ReactorFuelConsumption {
    /// How many seconds has this fuel been consumed for
    pub secs_spent: f32,
    /// The type of fuel being used
    pub fuel_id: u16,
}

impl IdentifiableComponent for ReactorFuelConsumption {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:reactor_fuel_consumption"
    }
}

impl SyncableComponent for ReactorFuelConsumption {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

impl Reactor {
    /// Creates a new constructed reactor
    pub fn new(controller: BlockCoordinate, power_per_second: f32, fuel_consumption_percentage: f32, bounds: ReactorBounds) -> Self {
        Self {
            bounds,
            controller,
            power_per_second,
            fuel_consumption_multiplier: fuel_consumption_percentage,
        }
    }

    /// Decreases the power-per-second generated in this reactor.
    pub fn decrease_power_per_second(&mut self, amount: f32) {
        self.power_per_second -= amount;
        self.power_per_second = self.power_per_second.max(0.0);
    }

    /// Increases the power-per-second generated in this reactor
    pub fn increase_power_per_second(&mut self, amount: f32) {
        self.power_per_second += amount;
    }

    /// Returns the power this reactor will generate per second
    pub fn power_per_second(&self) -> f32 {
        self.power_per_second
    }

    /// Returns the block where the controller for this reactor is
    pub fn controller_block(&self) -> BlockCoordinate {
        self.controller
    }
}

#[derive(Debug, Component, Default, Reflect, DerefMut, Deref, Serialize, Deserialize, Clone, PartialEq)]
/// Stores the entities of all the reactors in a structure and their controller blocks for quick access
pub struct Reactors(Vec<BlockCoordinate>);

impl Reactors {
    /// Adds a reactor to the structure
    pub fn add_reactor_controller(&mut self, block: BlockCoordinate) {
        self.0.push(block);
    }
}

impl IdentifiableComponent for Reactors {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:reactors"
    }
}

impl SyncableComponent for Reactors {
    fn get_sync_type() -> crate::netty::sync::SyncType {
        crate::netty::sync::SyncType::ServerAuthoritative
    }
}

#[derive(Debug, Clone)]
/// A block that can be used in a reactor to generate power
pub struct ReactorPowerGenerationBlock {
    power_per_second: f32,

    id: u16,
    unlocalized_name: String,
}

impl Identifiable for ReactorPowerGenerationBlock {
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

impl ReactorPowerGenerationBlock {
    /// Creates a link to a block to make it usable within a reactor to create power
    pub fn new(block: &Block, power_per_second: f32) -> Self {
        Self {
            power_per_second,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        }
    }

    /// The power per second this block will produce
    pub fn power_per_second(&self) -> f32 {
        self.power_per_second
    }
}

impl Registry<ReactorPowerGenerationBlock> {
    /// Gets the reactor power generation entry for this block
    pub fn for_block(&self, block: &Block) -> Option<&ReactorPowerGenerationBlock> {
        self.from_id(block.unlocalized_name())
    }
}

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// Send this to the player to cause them to open a reactor
pub struct OpenReactorMessage(pub StructureBlock);

impl IdentifiableMessage for OpenReactorMessage {
    fn unlocalized_name() -> &'static str {
        "cosmos:open_reactor"
    }
}

impl NettyMessage for OpenReactorMessage {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Client
    }

    #[cfg(feature = "client")]
    fn needs_entity_conversion() -> bool {
        true
    }

    #[cfg(feature = "client")]
    fn convert_entities_server_to_client(self, netty: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.0.map_to_client(netty).ok().map(Self)
    }
}

#[derive(Message, Debug, Serialize, Deserialize, Clone)]
/// The client requests to set the state of the reactor
pub struct ClientRequestChangeReactorStatus {
    /// The reactor they're controller toggling
    pub block: StructureBlock,
    /// If they want to activate/deactivate it
    pub active: bool,
}

impl IdentifiableMessage for ClientRequestChangeReactorStatus {
    fn unlocalized_name() -> &'static str {
        "cosmos:change_reactor_status"
    }
}

impl NettyMessage for ClientRequestChangeReactorStatus {
    fn event_receiver() -> crate::netty::sync::events::netty_event::MessageReceiver {
        crate::netty::sync::events::netty_event::MessageReceiver::Server
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
/// A fuel that can be consumed by the reactor
pub struct ReactorFuel {
    id: u16,
    unlocalized_name: String,
    /// How "efficient" this fuel is when generating power. The calculate is `reactor base power
    /// output * multiplier`.
    pub multiplier: f32,
    /// How long this fuel will last for before being used
    pub lasts_for: Duration,
}

impl ReactorFuel {
    /// Creates a new fuel based on this item.
    ///
    /// If you create and register a fuel of the same item type, the later entry will override the
    /// earlier entry.
    pub fn new(item: &Item, multiplier: f32, lasts_for: Duration) -> Self {
        Self {
            id: 0,
            unlocalized_name: item.unlocalized_name().into(),
            multiplier,
            lasts_for,
        }
    }
}

impl Identifiable for ReactorFuel {
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

pub(super) fn register(app: &mut App) {
    create_registry::<ReactorPowerGenerationBlock>(app, "cosmos:power_generation_blocks");
    create_registry::<ReactorFuel>(app, "cosmos:reactor_fuel");
    sync_component::<Reactors>(app);
    sync_component::<Reactor>(app);
    sync_component::<ReactorActive>(app);
    sync_component::<ReactorFuelConsumption>(app);

    sync_registry::<ReactorFuel>(app);

    app.add_netty_event::<OpenReactorMessage>();
    app.add_netty_event::<ClientRequestChangeReactorStatus>();

    app.register_type::<Reactor>()
        .register_type::<Reactors>()
        .register_type::<ReactorFuelConsumption>()
        .register_type::<ReactorActive>();
}
