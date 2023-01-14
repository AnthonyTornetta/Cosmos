use bevy::{ecs::schedule::StateData, prelude::*, utils::HashMap};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use iyes_loopless::prelude::*;

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        events::StructureLoadedEvent, systems::energy_storage_system::EnergyStorageSystem,
        Structure,
    },
};

use super::Systems;

struct EnergyGenerationProperty {
    generation_rate: f32,
}

#[derive(Default, Resource)]
struct EnergyGenerationBlocks {
    blocks: HashMap<u16, EnergyGenerationProperty>,
}

impl EnergyGenerationBlocks {
    pub fn insert(&mut self, block: &Block, generation_property: EnergyGenerationProperty) {
        self.blocks.insert(block.id(), generation_property);
    }

    pub fn get(&self, block: &Block) -> Option<&EnergyGenerationProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Component, Default, Inspectable)]
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
    if let Some(block) = blocks.from_id("cosmos:reactor_block") {
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
        if let Ok(mut system) = systems_query
            .get(ev.structure_entity)
            .expect("Structure should have Systems component")
            .query_mut(&mut system_query)
        {
            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                system.block_removed(prop);
            }

            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(prop);
            }
        }
    }
}

fn update_energy(
    sys_query: Query<&Systems>,
    e_gen_query: Query<&EnergyGenerationSystem>,
    mut e_storage_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for systems in sys_query.iter() {
        if let Ok(gen) = systems.query(&e_gen_query) {
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
                if let Some(prop) = thruster_blocks.get(&block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system);
        }
    }
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_state: T,
) {
    app.insert_resource(EnergyGenerationBlocks::default())
        .add_system_set(SystemSet::on_enter(post_loading_state).with_system(register_energy_blocks))
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_bevy_state(playing_state),
        )
        .add_system_set(SystemSet::on_update(playing_state).with_system(update_energy))
        .add_system_set(SystemSet::on_update(playing_state).with_system(structure_loaded_event))
        .register_inspectable::<EnergyGenerationSystem>();
}
