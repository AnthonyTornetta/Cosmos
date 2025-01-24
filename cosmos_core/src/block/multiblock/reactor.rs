//! Represents the logic behind the reactor multiblock system

use std::time::Duration;

use bevy::{
    prelude::{
        in_state, Added, App, Commands, Component, Deref, DerefMut, Entity, Event, EventReader, IntoSystemConfigs, OnEnter, Query, Res,
        ResMut, States, Update, Without,
    },
    reflect::Reflect,
    time::Time,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{block_events::BlockEventsSet, Block},
    events::block_events::BlockChangedEvent,
    item::Item,
    netty::{
        sync::{
            events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
            registry::sync_registry,
            sync_component, IdentifiableComponent, SyncableComponent,
        },
        system_sets::NetworkingSystemsSet,
    },
    prelude::StructureBlock,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, loading::StructureLoadingSet, systems::StructureSystemsSet},
};

#[derive(Debug, Clone, Copy, Reflect, Serialize, Deserialize, PartialEq, Eq)]
/// The inclusive bounds of a reactor, including its casing
pub struct ReactorBounds {
    /// Inclusive negative-most coordinates of a reactor (include casing)
    pub negative_coords: BlockCoordinate,
    /// Inclusive positive-most coordinates of a reactor (include casing)
    pub positive_coords: BlockCoordinate,
}

#[derive(Clone, Copy, Debug, Reflect, Serialize, Deserialize, PartialEq, Component)]
/// Represents a constructed reactor
pub struct Reactor {
    pub controller: BlockCoordinate,
    pub power_per_second: f32,
    pub bounds: ReactorBounds,
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
pub struct ReactorFuelConsumption {
    pub secs_spent: f32,
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
    pub fn new(controller: BlockCoordinate, power_per_second: f32, bounds: ReactorBounds) -> Self {
        Self {
            bounds,
            controller,
            power_per_second,
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

#[derive(Event, Debug, Serialize, Deserialize)]
pub struct OpenReactorEvent(pub StructureBlock);

impl IdentifiableEvent for OpenReactorEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:open_reactor"
    }

    #[cfg(feature = "client")]
    fn convert_to_client_entity(self, netty: &crate::netty::sync::mapping::NetworkMapping) -> Option<Self> {
        use crate::netty::sync::mapping::Mappable;

        self.0.map_to_client(&netty).ok().map(|x| Self(x))
    }
}

impl NettyEvent for OpenReactorEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Client
    }
}

#[derive(Event, Debug, Serialize, Deserialize)]
pub struct ClientRequestActiveReactorEvent {
    pub block: StructureBlock,
    pub active: bool,
}

impl IdentifiableEvent for ClientRequestActiveReactorEvent {
    fn unlocalized_name() -> &'static str {
        "cosmos:activate_reactor"
    }
}

impl NettyEvent for ClientRequestActiveReactorEvent {
    fn event_receiver() -> crate::netty::sync::events::netty_event::EventReceiver {
        crate::netty::sync::events::netty_event::EventReceiver::Server
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ReactorFuel {
    id: u16,
    unlocalized_name: String,
    pub multiplier: f32,
    pub lasts_for: Duration,
}

impl ReactorFuel {
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

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    create_registry::<ReactorPowerGenerationBlock>(app, "cosmos:power_generation_blocks");
    sync_component::<Reactors>(app);
    sync_component::<Reactor>(app);
    sync_component::<ReactorActive>(app);
    sync_component::<ReactorFuelConsumption>(app);

    sync_registry::<ReactorFuel>(app);

    app.add_netty_event::<OpenReactorEvent>();
    app.add_netty_event::<ClientRequestActiveReactorEvent>();

    app.register_type::<Reactor>()
        .register_type::<Reactors>()
        .register_type::<ReactorFuelConsumption>()
        .register_type::<ReactorActive>();
}
