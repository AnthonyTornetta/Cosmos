//! Represents all the energy generation in a structure

use bevy::{prelude::*, utils::HashMap};

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        events::StructureLoadedEvent, systems::energy_storage_system::EnergyStorageSystem,
        Structure,
    },
};

use super::{StructureSystem, Systems};

#[derive(Default, FromReflect, Reflect, Clone, Copy)]
/// Any block that can generate energy will have this property.
pub struct EnergyGenerationProperty {
    /// How much energy is generated
    pub generation_rate: f32,
}

#[derive(Default, Resource)]
/// All the energy generation blocks - register them here.
pub struct EnergyGenerationBlocks {
    blocks: HashMap<u16, EnergyGenerationProperty>,
}

impl EnergyGenerationBlocks {
    /// Inserts a block with a property
    pub fn insert(&mut self, block: &Block, generation_property: EnergyGenerationProperty) {
        self.blocks.insert(block.id(), generation_property);
    }

    /// Inserts a property form that block if it has one
    pub fn get(&self, block: &Block) -> Option<&EnergyGenerationProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Component, Default, Reflect, FromReflect)]
struct EnergyGenerationSystem {
    generation_rate: f32,
}

impl EnergyGenerationSystem {
    pub fn block_added(&mut self, prop: &EnergyGenerationProperty) {
        self.generation_rate += prop.generation_rate;
    }

    pub fn block_removed(&mut self, prop: &EnergyGenerationProperty) {
        self.generation_rate -= prop.generation_rate;
    }
}

fn register_energy_blocks(
    blocks: Res<Registry<Block>>,
    mut generation: ResMut<EnergyGenerationBlocks>,
) {
    if let Some(block) = blocks.from_id("cosmos:reactor") {
        generation.insert(
            block,
            EnergyGenerationProperty {
                generation_rate: 1000.0,
            },
        );
    }

    if let Some(block) = blocks.from_id("cosmos:ship_core") {
        generation.insert(
            block,
            EnergyGenerationProperty {
                generation_rate: 100.0,
            },
        )
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    energy_generation_blocks: Res<EnergyGenerationBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut EnergyGenerationSystem>,
    systems_query: Query<&Systems>,
) {
    for ev in event.iter() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                if let Some(prop) =
                    energy_generation_blocks.get(blocks.from_numeric_id(ev.old_block))
                {
                    system.block_removed(prop);
                }

                if let Some(prop) =
                    energy_generation_blocks.get(blocks.from_numeric_id(ev.new_block))
                {
                    system.block_added(prop);
                }
            }
        }
    }
}

fn update_energy(
    sys_query: Query<&Systems>,
    e_gen_query: Query<(&EnergyGenerationSystem, &StructureSystem)>,
    mut e_storage_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (gen, system) in e_gen_query.iter() {
        if let Ok(systems) = sys_query.get(system.structure_entity) {
            if let Ok(mut storage) = systems.query_mut(&mut e_storage_query) {
                storage.increase_energy(gen.generation_rate * time.delta_seconds());
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    thruster_blocks: Res<EnergyGenerationBlocks>,
) {
    for ev in event_reader.iter() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = EnergyGenerationSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = thruster_blocks.get(block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system);
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_state: T,
) {
    app.insert_resource(EnergyGenerationBlocks::default())
        .add_systems((
            register_energy_blocks.in_schedule(OnEnter(post_loading_state)),
            // block update system used to be in CoreState::PostUpdate
            structure_loaded_event.in_set(OnUpdate(playing_state)),
            block_update_system.in_set(OnUpdate(playing_state)),
            update_energy.in_set(OnUpdate(playing_state)),
        ))
        .register_type::<EnergyGenerationSystem>();
}
