use bevy::{ecs::schedule::StateData, prelude::*, utils::HashMap};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use iyes_loopless::prelude::*;

use crate::{
    block::{blocks::Blocks, Block},
    events::block_events::BlockChangedEvent,
    structure::{
        chunk::CHUNK_DIMENSIONS, events::ChunkSetEvent, structure::Structure,
        systems::energy_storage_system::EnergyStorageSystem,
    },
};

struct EnergyGenerationProperty {
    generation_rate: f32,
}

#[derive(Default)]
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

fn register_energy_blocks(blocks: Res<Blocks>, mut generation: ResMut<EnergyGenerationBlocks>) {
    if let Some(block) = blocks.block_from_id("cosmos:reactor_block") {
        generation.insert(
            block,
            EnergyGenerationProperty {
                generation_rate: 1000.0,
            },
        );
    }

    if let Some(block) = blocks.block_from_id("cosmos:ship_core") {
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
    blocks: Res<Blocks>,
    mut system_query: Query<&mut EnergyGenerationSystem>,
    structure_query: Query<&Structure>,
) {
    for ev in event.iter() {
        let sys = system_query.get_mut(ev.structure_entity);

        if let Ok(mut system) = sys {
            if let Some(es) =
                energy_generation_blocks.get(blocks.block_from_numeric_id(ev.old_block))
            {
                system.generation_rate -= es.generation_rate;
            }

            if let Some(es) =
                energy_generation_blocks.get(blocks.block_from_numeric_id(ev.new_block))
            {
                system.generation_rate += es.generation_rate;
            }
        } else {
            let mut system = EnergyGenerationSystem::default();

            if let Some(es) =
                energy_generation_blocks.get(blocks.block_from_numeric_id(ev.old_block))
            {
                system.generation_rate -= es.generation_rate;
            }

            if let Some(es) =
                energy_generation_blocks.get(blocks.block_from_numeric_id(ev.new_block))
            {
                system.generation_rate += es.generation_rate;
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }

    // ChunkSetEvents should not overwrite existing blocks, so no need to check for that
    for ev in chunk_set_event.iter() {
        let sys = system_query.get_mut(ev.structure_entity);
        let structure = structure_query.get(ev.structure_entity).unwrap();

        if let Ok(mut system) = sys {
            for z in ev.z * CHUNK_DIMENSIONS..(ev.z + 1) * CHUNK_DIMENSIONS {
                for y in (ev.y * CHUNK_DIMENSIONS)..(ev.y + 1) * CHUNK_DIMENSIONS {
                    for x in ev.x * CHUNK_DIMENSIONS..(ev.x + 1) * CHUNK_DIMENSIONS {
                        let b = structure.block_at(x, y, z);

                        if energy_generation_blocks.blocks.contains_key(&b) {
                            system.generation_rate += energy_generation_blocks
                                .get(blocks.block_from_numeric_id(b))
                                .unwrap()
                                .generation_rate;
                        }
                    }
                }
            }
        } else {
            let mut system = EnergyGenerationSystem::default();

            for z in ev.z * CHUNK_DIMENSIONS..(ev.z + 1) * CHUNK_DIMENSIONS {
                for y in (ev.y * CHUNK_DIMENSIONS)..(ev.y + 1) * CHUNK_DIMENSIONS {
                    for x in ev.x * CHUNK_DIMENSIONS..(ev.x + 1) * CHUNK_DIMENSIONS {
                        let b = structure.block_at(x, y, z);

                        if energy_generation_blocks.blocks.contains_key(&b) {
                            system.generation_rate += energy_generation_blocks
                                .get(blocks.block_from_numeric_id(b))
                                .unwrap()
                                .generation_rate;
                        }
                    }
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

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_state: T) {
    app.insert_resource(EnergyGenerationBlocks::default())
        .add_system_set(SystemSet::on_enter(post_loading_state).with_system(register_energy_blocks))
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_bevy_state(playing_state.clone()),
        )
        .add_system_set(SystemSet::on_update(playing_state).with_system(update_energy))
        .register_inspectable::<EnergyGenerationSystem>();
}
