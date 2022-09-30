use bevy::{
    ecs::schedule::StateData,
    prelude::{App, Component, CoreStage, EventReader, Query, Res, ResMut, SystemSet},
    utils::HashMap,
};
use iyes_loopless::prelude::*;

use crate::{
    block::{block::Block, blocks::Blocks},
    events::block_events::BlockChangedEvent,
    structure::{chunk::CHUNK_DIMENSIONS, events::ChunkSetEvent, structure::Structure},
};

struct EnergyStorageProperty {
    capacity: f32,
}

#[derive(Default)]
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

#[derive(Component)]
struct EnergyStorageSystem {
    energy: f32,
    capacity: f32,
}

fn register_energy_blocks(blocks: Res<Blocks>, mut storage: ResMut<EnergyStorageBlocks>) {
    if let Some(block) = blocks.block_from_id("cosmos:energy_cell") {
        storage.insert(block, EnergyStorageProperty { capacity: 1000.0 });
    }

    if let Some(block) = blocks.block_from_id("cosmos:ship_core") {
        storage.insert(block, EnergyStorageProperty { capacity: 5000.0 })
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    energy_storage_blocks: Res<EnergyStorageBlocks>,
    blocks: Res<Blocks>,
    mut system_query: Query<&mut EnergyStorageSystem>,
    structure_query: Query<&Structure>,
) {
    for ev in event.iter() {
        let mut system = None;
        if let Some(es) = energy_storage_blocks.get(blocks.block_from_numeric_id(ev.old_block)) {
            system = Some(system_query.get_mut(ev.structure_entity.clone()).unwrap());
            system.as_mut().unwrap().capacity -= es.capacity;
        }

        if let Some(es) = energy_storage_blocks.get(blocks.block_from_numeric_id(ev.new_block)) {
            if system.is_none() {
                system = Some(system_query.get_mut(ev.structure_entity.clone()).unwrap());
            }
            system.as_mut().unwrap().capacity += es.capacity;
        }
    }

    // ChunkSetEvents should not overwrite existing blocks, so no need to check for that
    for ev in chunk_set_event.iter() {
        let mut system = system_query.get_mut(ev.structure_entity).unwrap();
        let structure = structure_query.get(ev.structure_entity).unwrap();

        for z in ev.z * CHUNK_DIMENSIONS..(ev.z + 1) * CHUNK_DIMENSIONS {
            for y in (ev.y * CHUNK_DIMENSIONS)..(ev.y + 1) * CHUNK_DIMENSIONS {
                for x in ev.x * CHUNK_DIMENSIONS..(ev.x + 1) * CHUNK_DIMENSIONS {
                    let b = structure.block_at(x, y, z);
                    system.capacity += energy_storage_blocks
                        .get(blocks.block_from_numeric_id(b))
                        .unwrap()
                        .capacity;
                }
            }
        }
    }
}

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_state: T) {
    app.insert_resource(EnergyStorageBlocks::default())
        .add_system_set(SystemSet::on_enter(post_loading_state).with_system(register_energy_blocks))
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_state(playing_state),
        );
}
