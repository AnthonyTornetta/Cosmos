use bevy::{ecs::schedule::StateData, prelude::*, utils::HashMap};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use iyes_loopless::prelude::*;

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        events::ChunkSetEvent, systems::energy_storage_system::EnergyStorageSystem, Structure,
    },
};

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
    mut commands: Commands,
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    energy_generation_blocks: Res<EnergyGenerationBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut EnergyGenerationSystem>,
    structure_query: Query<&Structure>,
) {
    for ev in event.iter() {
        let sys = system_query.get_mut(ev.structure_entity);

        if let Ok(mut system) = sys {
            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                system.block_removed(prop);
            }

            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(prop);
            }
        } else {
            let mut system = EnergyGenerationSystem::default();

            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                system.block_removed(prop);
            }

            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(prop);
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }

    // ChunkSetEvents should not overwrite existing blocks, so no need to check for that
    for ev in chunk_set_event.iter() {
        let structure = structure_query.get(ev.structure_entity).unwrap();

        if let Ok(mut system) = system_query.get_mut(ev.structure_entity) {
            for block in ev.iter_blocks(structure) {
                if let Some(prop) = energy_generation_blocks.get(&block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }
        } else {
            let mut system = EnergyGenerationSystem::default();

            for block in ev.iter_blocks(structure) {
                if let Some(prop) = energy_generation_blocks.get(&block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }
}

fn update_energy(
    mut query: Query<(&EnergyGenerationSystem, &mut EnergyStorageSystem)>,
    time: Res<Time>,
) {
    for (sys, mut storage) in query.iter_mut() {
        storage.increase_energy(sys.generation_rate * time.delta_seconds());
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
        .register_inspectable::<EnergyGenerationSystem>();
}
