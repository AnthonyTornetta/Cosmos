use bevy::{ecs::schedule::StateData, prelude::*, utils::HashMap};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use iyes_loopless::prelude::*;

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        chunk::CHUNK_DIMENSIONS, events::ChunkSetEvent,
        systems::energy_storage_system::EnergyStorageSystem, Structure, StructureBlock,
    },
};

struct LaserCannonProperty {
    energy_per_shot: f32,
}

#[derive(Default, Resource)]
struct LaserCannonBlocks {
    blocks: HashMap<u16, LaserCannonProperty>,
}

impl LaserCannonBlocks {
    pub fn insert(&mut self, block: &Block, cannon_property: LaserCannonProperty) {
        self.blocks.insert(block.id(), cannon_property);
    }

    pub fn get(&self, block: &Block) -> Option<&LaserCannonProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Inspectable, Default)]
struct Line {
    start: StructureBlock,
    direction: (usize, usize, usize),
    len: usize,
    energy_per_shot: f32,
}

#[derive(Component, Default, Inspectable)]
struct LaserCannonSystem {
    lines: Vec<Line>,
}

impl LaserCannonSystem {
    fn laser_cannon_removed(&mut self, old_prop: &LaserCannonProperty, sb: &StructureBlock) {}

    fn laser_cannon_added(&mut self, prop: &LaserCannonProperty, sb: &StructureBlock) {}
}

fn register_laser_blocks(blocks: Res<Registry<Block>>, mut cannon: ResMut<LaserCannonBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:laser_cannon") {
        cannon.insert(
            block,
            LaserCannonProperty {
                energy_per_shot: 100.0,
            },
        )
    }
}

fn block_update_system(
    mut commands: Commands,
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    laser_cannon_blocks: Res<LaserCannonBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut LaserCannonSystem>,
    structure_query: Query<&Structure>,
) {
    for ev in event.iter() {
        if let Ok(mut system) = system_query.get_mut(ev.structure_entity) {
            if let Some(property) = laser_cannon_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                system.laser_cannon_removed(property, &ev.block);
            }

            if let Some(property) = laser_cannon_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.laser_cannon_added(property, &ev.block);
            }
        } else {
            let mut system = LaserCannonSystem::default();

            if let Some(property) = laser_cannon_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.laser_cannon_added(property, &ev.block);
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

                        if laser_cannon_blocks.blocks.contains_key(&b) {
                            system.cannon_rate += laser_cannon_blocks
                                .get(blocks.from_numeric_id(b))
                                .unwrap()
                                .cannon_rate;
                        }
                    }
                }
            }
        } else {
            let mut system = LaserCannonSystem::default();

            for z in ev.z * CHUNK_DIMENSIONS..(ev.z + 1) * CHUNK_DIMENSIONS {
                for y in (ev.y * CHUNK_DIMENSIONS)..(ev.y + 1) * CHUNK_DIMENSIONS {
                    for x in ev.x * CHUNK_DIMENSIONS..(ev.x + 1) * CHUNK_DIMENSIONS {
                        let b = structure.block_at(x, y, z);

                        if laser_cannon_blocks.blocks.contains_key(&b) {
                            system.cannon_rate += laser_cannon_blocks
                                .get(blocks.from_numeric_id(b))
                                .unwrap()
                                .cannon_rate;
                        }
                    }
                }
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }
}

fn update_laser(mut query: Query<(&LaserCannonSystem, &mut EnergyStorageSystem)>, time: Res<Time>) {
    for (sys, mut storage) in query.iter_mut() {
        storage.increase_laser(sys.cannon_rate * time.delta_seconds());
    }
}

pub fn register<T: StateData + Clone + Copy>(
    app: &mut App,
    post_loading_state: T,
    playing_state: T,
) {
    app.insert_resource(LaserCannonBlocks::default())
        .add_system_set(SystemSet::on_enter(post_loading_state).with_system(register_laser_blocks))
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_bevy_state(playing_state),
        )
        .add_system_set(SystemSet::on_update(playing_state).with_system(update_laser))
        .register_inspectable::<LaserCannonSystem>();
}
