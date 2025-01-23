//! Represents the logic behind the reactor multiblock system

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
    netty::{
        sync::{
            events::netty_event::{IdentifiableEvent, NettyEvent, SyncedEventImpl},
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

#[derive(Clone, Copy, Debug, Reflect, Serialize, Deserialize, PartialEq)]
/// Represents a constructed reactor
pub struct Reactor {
    pub controller: BlockCoordinate,
    pub power_per_second: f32,
    pub bounds: ReactorBounds,
}

#[derive(Component)]
pub struct ReactorActive;

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
pub struct Reactors(Vec<Reactor>);

impl Reactors {
    /// Adds a reactor to the structure
    pub fn add_reactor(&mut self, reactor: Reactor) {
        self.0.push(reactor);
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

fn register_power_blocks(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<ReactorPowerGenerationBlock>>) {
    if let Some(reactor_block) = blocks.from_id("cosmos:reactor_cell") {
        registry.register(ReactorPowerGenerationBlock::new(reactor_block, 1000.0));
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

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    create_registry::<ReactorPowerGenerationBlock>(app, "cosmos:power_generation_blocks");
    sync_component::<Reactors>(app);

    app.add_netty_event::<OpenReactorEvent>();

    app.add_systems(OnEnter(post_loading_state), register_power_blocks)
        .register_type::<Reactor>()
        .register_type::<Reactors>();
}
