use bevy::{ecs::schedule::StateData, prelude::*, utils::HashMap};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use iyes_loopless::prelude::*;

use crate::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        events::ChunkSetEvent, systems::energy_storage_system::EnergyStorageSystem, Structure,
        StructureBlock,
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
    direction: (i32, i32, i32),
    len: usize,
    energy_per_shot: f32,
}

#[derive(Component, Default, Inspectable)]
struct LaserCannonSystem {
    lines: Vec<Line>,
}

impl LaserCannonSystem {
    fn block_removed(&mut self, old_prop: &LaserCannonProperty, sb: &StructureBlock) {}

    fn block_added(&mut self, prop: &LaserCannonProperty, block: &StructureBlock) {
        for line in self.lines.iter_mut() {
            let (dx, dy, dz) = line.direction;

            let (sx, sy, sz) = (
                line.start.x as i32,
                line.start.y as i32,
                line.start.z as i32,
            );

            let (bx, by, bz) = (block.x as i32, block.y as i32, block.z as i32);

            // Block is before start
            if sx - dx == bx && sy - dy == by && sz - dz == bz {
                line.start.x -= dx as usize;
                line.start.y -= dy as usize;
                line.start.z -= dz as usize;
                line.len += 1;
                line.energy_per_shot += prop.energy_per_shot;

                return;
            }
            // Block is after end
            else if sx + dx * (line.len as i32 + 1) == bx
                && sy + dy * (line.len as i32 + 1) == by
                && sz + dz * (line.len as i32 + 1) == bz
            {
                line.len += 1;
                line.energy_per_shot += prop.energy_per_shot;

                return;
            }
        }

        // If gotten here, no suitable line was found

        self.lines.push(Line {
            start: *block,
            direction: (0, 0, 1), // Always assume +z direction (for now) - eventually account for rotation?
            len: 1,
            energy_per_shot: prop.energy_per_shot,
        });
    }
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
                system.block_removed(property, &ev.block);
            }

            if let Some(property) = laser_cannon_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(property, &ev.block);
            }
        } else {
            let mut system = LaserCannonSystem::default();

            if let Some(property) = laser_cannon_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(property, &ev.block);
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }

    // ChunkSetEvents should not overwrite existing blocks, so no need to check for that
    for ev in chunk_set_event.iter() {
        let structure = structure_query.get(ev.structure_entity).unwrap();

        if let Ok(mut system) = system_query.get_mut(ev.structure_entity) {
            for block in ev.iter_blocks(structure) {
                if let Some(prop) = laser_cannon_blocks.get(block.block(structure, &blocks)) {
                    system.block_added(prop, &block);
                }
            }
        } else {
            let mut system = LaserCannonSystem::default();

            for block in ev.iter_blocks(structure) {
                if let Some(prop) = laser_cannon_blocks.get(&block.block(structure, &blocks)) {
                    system.block_added(prop, &block);
                }
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }
}

// fn update_laser(mut query: Query<(&LaserCannonSystem, &mut EnergyStorageSystem)>, time: Res<Time>) {
// }

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
        // .add_system_set(SystemSet::on_update(playing_state).with_system(update_laser))
        .register_inspectable::<LaserCannonSystem>();
}
//
