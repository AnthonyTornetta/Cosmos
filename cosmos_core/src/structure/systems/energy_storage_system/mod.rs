//! Represents all the energy stored on a structure

use bevy::{
    prelude::{in_state, App, Commands, Component, EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut, Resource, States, Update},
    reflect::Reflect,
    utils::HashMap,
};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{events::StructureLoadedEvent, Structure},
};

use super::Systems;

#[derive(Default, Reflect, Clone, Copy)]
/// Every block that can store energy should have this property
pub struct EnergyStorageProperty {
    /// How much energy this block can store
    pub capacity: f32,
}

#[derive(Default, Resource)]
struct EnergyStorageBlocks {
    blocks: HashMap<u16, EnergyStorageProperty>,
}

impl EnergyStorageBlocks {
    pub fn insert(&mut self, block: &Block, storage_property: EnergyStorageProperty) {
        self.blocks.insert(block.id(), storage_property);
    }

    pub fn get(&self, block: &Block) -> Option<&EnergyStorageProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Component, Default, Reflect)]
/// Represents the energy storage of a structure
pub struct EnergyStorageSystem {
    energy: f32,
    capacity: f32,
}

impl EnergyStorageSystem {
    fn block_added(&mut self, prop: &EnergyStorageProperty) {
        self.capacity += prop.capacity;
    }

    fn block_removed(&mut self, prop: &EnergyStorageProperty) {
        self.capacity -= prop.capacity;
    }

    /// Increases the energy stored in this system
    pub fn increase_energy(&mut self, delta: f32) {
        self.energy = self.capacity.min(self.energy + delta);
    }

    /// Decreases the energy stored in this system - does not go below 0.
    ///
    /// Make sure to check using `get_energy` if there is enough to use.
    pub fn decrease_energy(&mut self, delta: f32) {
        self.energy = (self.energy - delta).max(0.0);
    }

    /// Gets the current stored energy of the system
    pub fn get_energy(&self) -> f32 {
        self.energy
    }

    /// Gets the totaly capacity of this system
    pub fn get_capacity(&self) -> f32 {
        self.capacity
    }
}

fn register_energy_blocks(blocks: Res<Registry<Block>>, mut storage: ResMut<EnergyStorageBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:energy_cell") {
        storage.insert(block, EnergyStorageProperty { capacity: 10000.0 });
    }

    if let Some(block) = blocks.from_id("cosmos:ship_core") {
        storage.insert(block, EnergyStorageProperty { capacity: 1000.0 })
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    energy_storage_blocks: Res<EnergyStorageBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut EnergyStorageSystem>,
    systems_query: Query<&Systems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                    system.block_removed(prop);
                }

                if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                    system.block_added(prop);
                }
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    thruster_blocks: Res<EnergyStorageBlocks>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = EnergyStorageSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = thruster_blocks.get(block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system);
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T, playing_state: T) {
    app.insert_resource(EnergyStorageBlocks::default())
        .add_systems(OnEnter(post_loading_state), register_energy_blocks)
        .add_systems(
            Update,
            (structure_loaded_event, block_update_system).run_if(in_state(playing_state)),
        )
        .register_type::<EnergyStorageSystem>();
}
