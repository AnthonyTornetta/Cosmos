//! Represents the logic behind the reactor multiblock system

use bevy::{
    prelude::{
        in_state, Added, App, Commands, Component, Deref, DerefMut, Entity, EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut,
        States, Update, Without,
    },
    reflect::Reflect,
    time::Time,
};
use serde::{Deserialize, Serialize};

use crate::{
    block::{block_events::BlockEventsSet, Block},
    events::block_events::BlockChangedEvent,
    netty::{
        sync::{sync_component, IdentifiableComponent, SyncableComponent},
        system_sets::NetworkingSystemsSet,
    },
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{
        coordinates::BlockCoordinate,
        loading::StructureLoadingSet,
        structure_block::StructureBlock,
        systems::{energy_storage_system::EnergyStorageSystem, StructureSystems, StructureSystemsSet},
        Structure,
    },
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
    controller: StructureBlock,
    power_per_second: f32,
    bounds: ReactorBounds,
}

impl Reactor {
    /// Creates a new constructed reactor
    pub fn new(controller: StructureBlock, power_per_second: f32, bounds: ReactorBounds) -> Self {
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

    /// Returns the block where the controller for this reactor is
    pub fn controller_block(&self) -> StructureBlock {
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

fn add_reactor_to_structure(mut commands: Commands, query: Query<Entity, (Added<Structure>, Without<Reactors>)>) {
    for ent in query.iter() {
        commands.entity(ent).insert(Reactors::default());
    }
}

fn on_modify_reactor(
    mut reactors_query: Query<&mut Reactors>,
    mut block_change_event: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    reactor_cells: Res<Registry<ReactorPowerGenerationBlock>>,
) {
    for ev in block_change_event.read() {
        let Ok(mut reactors) = reactors_query.get_mut(ev.block.structure()) else {
            continue;
        };

        reactors.retain_mut(|reactor| {
            let (neg, pos) = (reactor.bounds.negative_coords, reactor.bounds.positive_coords);

            let block = ev.block.coords();

            let within_x = neg.x <= block.x && pos.x >= block.x;
            let within_y = neg.y <= block.y && pos.y >= block.y;
            let within_z = neg.z <= block.z && pos.z >= block.z;

            if (neg.x == block.x || pos.x == block.x) && (within_y && within_z)
                || (neg.y == block.y || pos.y == block.y) && (within_x && within_z)
                || (neg.z == block.z || pos.z == block.z) && (within_x && within_y)
            {
                // They changed the casing of the reactor - kill it
                false
            } else {
                if within_x && within_y && within_z {
                    // The innards of the reactor were changed, add/remove any needed power per second
                    if let Some(reactor_cell) = reactor_cells.for_block(blocks.from_numeric_id(ev.old_block)) {
                        reactor.decrease_power_per_second(reactor_cell.power_per_second());
                    }

                    if let Some(reactor_cell) = reactor_cells.for_block(blocks.from_numeric_id(ev.new_block)) {
                        reactor.increase_power_per_second(reactor_cell.power_per_second());
                    }
                }

                true
            }
        });
    }
}

// TODO: move this to server
fn generate_power(
    reactors: Query<(&Reactors, Entity)>,
    structure: Query<&StructureSystems>,
    mut energy_storage_system_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (reactors, structure_entity) in reactors.iter() {
        let Ok(systems) = structure.get(structure_entity) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut energy_storage_system_query) else {
            continue;
        };

        for reactor in reactors.iter() {
            system.increase_energy(reactor.power_per_second * time.delta_seconds());
        }
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T, playing_state: T) {
    create_registry::<ReactorPowerGenerationBlock>(app, "cosmos:power_generation_blocks");
    sync_component::<Reactors>(app);

    app.add_systems(OnEnter(post_loading_state), register_power_blocks)
        .add_systems(
            Update,
            (
                add_reactor_to_structure.in_set(StructureLoadingSet::AddStructureComponents),
                (on_modify_reactor.in_set(BlockEventsSet::ProcessEvents), generate_power)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks)
                    .in_set(NetworkingSystemsSet::Between)
                    .chain(),
            )
                .chain()
                .in_set(NetworkingSystemsSet::Between)
                .run_if(in_state(playing_state)),
        )
        .register_type::<Reactor>()
        .register_type::<Reactors>();
}
