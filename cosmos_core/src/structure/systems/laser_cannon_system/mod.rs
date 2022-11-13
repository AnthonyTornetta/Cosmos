use bevy::{
    ecs::schedule::StateData,
    prelude::{
        App, Commands, Component, CoreStage, EventReader, Query, Res, ResMut, Resource, SystemSet,
    },
    utils::HashMap,
};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use iyes_loopless::prelude::*;

use crate::{
    block::{blocks::Blocks, Block, BlockFace},
    events::block_events::BlockChangedEvent,
    structure::{chunk::CHUNK_DIMENSIONS, events::ChunkSetEvent, structure::Structure},
};

pub struct LaserCannonProperty {
    pub strength: f32,
    pub energy_consupmtion: f32,
}

#[derive(Default, Resource)]
struct LaserCannonBlocks {
    blocks: HashMap<u16, LaserCannonProperty>,
}

impl LaserCannonBlocks {
    pub fn insert(&mut self, block: &Block, thruster: LaserCannonProperty) {
        self.blocks.insert(block.id(), thruster);
    }

    pub fn get(&self, block: &Block) -> Option<&LaserCannonProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Inspectable, Default)]
struct CannonLocation {
    start: (usize, usize, usize),
    facing: BlockFace,
    length: usize,
}

#[derive(Component, Default, Inspectable)]
pub struct LaserCannonSystem {
    cannon_locations: Vec<CannonLocation>,
    energy_consumption: f32,
}

fn register_thruster_blocks(blocks: Res<Blocks>, mut storage: ResMut<LaserCannonBlocks>) {
    if let Some(block) = blocks.block_from_id("cosmos:laser_cannon") {
        storage.insert(
            block,
            LaserCannonProperty {
                strength: 2.0,
                energy_consupmtion: 100.0,
            },
        );
    }
}

fn calculate_cannon_removal(
    property: &LaserCannonProperty,
    x: usize,
    y: usize,
    z: usize,
    system: &mut LaserCannonSystem,
) {
    system.energy_consumption -= property.energy_consupmtion;

    for i in 0..system.cannon_locations.len() {
        let location = &mut system.cannon_locations[i];
        match location.facing {
            BlockFace::Front => {
                if x == location.start.0 && y == location.start.1 {
                    if z == location.start.2 {
                        location.start.2 += 1;
                        location.length -= 1;

                        if location.length == 0 {
                            system.cannon_locations.swap_remove(i);
                        }
                    } else if z == location.start.2 + location.length - 1 {
                        location.length -= 1;
                    } else {
                        let row_one = CannonLocation {
                            start: location.start,
                            facing: location.facing,
                            length: z - location.start.2,
                        };

                        let row_two = CannonLocation {
                            start: (location.start.0, location.start.1, z),
                            facing: location.facing,
                            length: location.length - (z - location.start.2),
                        };

                        system.cannon_locations.swap_remove(i);

                        system.cannon_locations.push(row_one);

                        if row_two.length != 0 {
                            system.cannon_locations.push(row_two);
                        }
                    }

                    return;
                }
            }
            BlockFace::Back => {
                if x == location.start.0 && y == location.start.1 {
                    println!(
                        "Z: {}, start: {}, length: {}, start - length: {}",
                        z,
                        location.start.2,
                        location.length,
                        location.start.2 - location.length
                    );

                    if z == location.start.2 {
                        location.start.2 -= 1;
                        location.length -= 1;

                        if location.length == 0 {
                            system.cannon_locations.swap_remove(i);
                        }

                        return;
                    } else if z == location.start.2 - (location.length - 1) {
                        location.length -= 1;

                        return;
                    } else if z < location.start.2 && z > (location.start.2 - (location.length - 1))
                    {
                        let row_one = CannonLocation {
                            start: location.start,
                            facing: location.facing,
                            length: location.start.2 - z,
                        };

                        let row_two = CannonLocation {
                            start: (location.start.0, location.start.1, z - 1),
                            facing: location.facing,
                            length: location.length - (location.start.2 - z) - 1,
                        };

                        system.cannon_locations.swap_remove(i);

                        system.cannon_locations.push(row_one);
                        system.cannon_locations.push(row_two);

                        return;
                    }
                }
            }
            BlockFace::Right => {
                if y == location.start.1 && z == location.start.2 {
                    if location.start.0 != 0 && x == location.start.0 - 1 {
                        location.start.0 -= 1;
                    }
                    location.length += 1;

                    return;
                }
            }
            BlockFace::Left => {
                if y == location.start.1 && z == location.start.2 {
                    if x == location.start.0 + 1 {
                        location.start.0 += 1;
                    }
                    location.length += 1;

                    return;
                }
            }
            BlockFace::Top => {
                if x == location.start.0 && z == location.start.2 {
                    if location.start.1 != 0 && y == location.start.1 - 1 {
                        location.start.1 -= 1;
                    }
                    location.length += 1;

                    return;
                }
            }
            BlockFace::Bottom => {
                if x == location.start.0 && z == location.start.2 {
                    if y == location.start.1 + 1 {
                        location.start.1 += 1;
                    }
                    location.length += 1;

                    return;
                }
            }
        }
    }
}

fn calculate_cannon_addition(
    property: &LaserCannonProperty,
    x: usize,
    y: usize,
    z: usize,
    system: &mut LaserCannonSystem,
) {
    system.energy_consumption += property.energy_consupmtion;

    for i in 0..system.cannon_locations.len() {
        let (start_x, start_y, start_z) = system.cannon_locations[i].start;
        let mut length = system.cannon_locations[i].length;
        let facing = system.cannon_locations[i].facing;

        match facing {
            BlockFace::Front => {
                if x == start_x && y == start_y {
                    if start_z != 0 && z == start_z - 1 {
                        system.cannon_locations[i].start.2 -= 1;
                    }
                    system.cannon_locations[i].length += 1;

                    for j in 0..system.cannon_locations.len() {
                        let loc = &system.cannon_locations[j];
                        if loc.start.0 == start_x
                            && loc.start.1 == start_y
                            && loc.start.2 == start_z + length
                        {
                            system.cannon_locations[i].length += loc.length;
                            system.cannon_locations.swap_remove(j);
                            break;
                        }
                    }
                    return;
                }
            }
            BlockFace::Back => {
                if x == start_x && y == start_y {
                    if z == start_z + 1 {
                        system.cannon_locations[i].start.2 += 1;
                        system.cannon_locations[i].length += 1;
                        length += 1;

                        for j in 0..system.cannon_locations.len() {
                            if j == i {
                                continue;
                            }

                            let loc = &system.cannon_locations[j];
                            if loc.start.0 == start_x
                                && loc.start.1 == start_y
                                && loc.start.2 + loc.length == z + 1
                            {
                                system.cannon_locations[j].length += length;
                                system.cannon_locations.swap_remove(i);
                                break;
                            }
                        }
                        return;
                    } else if z == start_z - length {
                        system.cannon_locations[i].length += 1;
                        length += 1;

                        for j in 0..system.cannon_locations.len() {
                            if j == i {
                                continue;
                            }

                            let loc = &system.cannon_locations[j];
                            if loc.start.0 == start_x
                                && loc.start.1 == start_y
                                && loc.start.2 == start_z - length
                            {
                                system.cannon_locations[i].length += length;
                                system.cannon_locations.swap_remove(j);
                                break;
                            }
                        }
                        return;
                    }
                }
            }
            _ => {} // BlockFace::Right => {
                    //     if y == location.start.1 && z == location.start.2 {
                    //         if location.start.0 != 0 && x == location.start.0 - 1 {
                    //             location.start.0 -= 1;
                    //         }
                    //         location.length += 1;

                    //         return;
                    //     }
                    // }
                    // BlockFace::Left => {
                    //     if y == location.start.1 && z == location.start.2 {
                    //         if x == location.start.0 + 1 {
                    //             location.start.0 += 1;
                    //         }
                    //         location.length += 1;

                    //         return;
                    //     }
                    // }
                    // BlockFace::Top => {
                    //     if x == location.start.0 && z == location.start.2 {
                    //         if location.start.1 != 0 && y == location.start.1 - 1 {
                    //             location.start.1 -= 1;
                    //         }
                    //         location.length += 1;

                    //         return;
                    //     }
                    // }
                    // BlockFace::Bottom => {
                    //     if x == location.start.0 && z == location.start.2 {
                    //         if y == location.start.1 + 1 {
                    //             location.start.1 += 1;
                    //         }
                    //         location.length += 1;

                    //         return;
                    //     }
                    // }
        }
    }

    // If we got here, no existing laser cannon rows could be found

    system.cannon_locations.push(CannonLocation {
        start: (x, y, z),
        length: 1,
        facing: BlockFace::Back,
    });
}

fn block_update_system(
    mut commands: Commands,
    mut event: EventReader<BlockChangedEvent>,
    mut chunk_set_event: EventReader<ChunkSetEvent>,
    laser_cannon_blocks: Res<LaserCannonBlocks>,
    blocks: Res<Blocks>,
    mut system_query: Query<&mut LaserCannonSystem>,
    structure_query: Query<&Structure>,
) {
    for ev in event.iter() {
        if let Ok(mut system) = system_query.get_mut(ev.structure_entity) {
            if let Some(property) =
                laser_cannon_blocks.get(blocks.block_from_numeric_id(ev.old_block))
            {
                calculate_cannon_removal(
                    &property,
                    ev.block.x(),
                    ev.block.y(),
                    ev.block.z(),
                    &mut system,
                );
            }

            if let Some(property) =
                laser_cannon_blocks.get(blocks.block_from_numeric_id(ev.new_block))
            {
                calculate_cannon_addition(
                    &property,
                    ev.block.x(),
                    ev.block.y(),
                    ev.block.z(),
                    &mut system,
                );
            }
        } else {
            let mut system = LaserCannonSystem::default();

            if let Some(property) =
                laser_cannon_blocks.get(blocks.block_from_numeric_id(ev.old_block))
            {
                calculate_cannon_removal(
                    &property,
                    ev.block.x(),
                    ev.block.y(),
                    ev.block.z(),
                    &mut system,
                );
            }

            if let Some(property) =
                laser_cannon_blocks.get(blocks.block_from_numeric_id(ev.new_block))
            {
                calculate_cannon_addition(
                    &property,
                    ev.block.x(),
                    ev.block.y(),
                    ev.block.z(),
                    &mut system,
                );
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }

    // ChunkSetEvents should not overwrite existing blocks, so no need to check for that
    for ev in chunk_set_event.iter() {
        let structure = structure_query.get(ev.structure_entity).unwrap();

        if let Ok(mut system) = system_query.get_mut(ev.structure_entity) {
            for z in ev.z * CHUNK_DIMENSIONS..(ev.z + 1) * CHUNK_DIMENSIONS {
                for y in (ev.y * CHUNK_DIMENSIONS)..(ev.y + 1) * CHUNK_DIMENSIONS {
                    for x in ev.x * CHUNK_DIMENSIONS..(ev.x + 1) * CHUNK_DIMENSIONS {
                        let b = structure.block_at(x, y, z);

                        if laser_cannon_blocks.blocks.contains_key(&b) {
                            let property = laser_cannon_blocks
                                .get(blocks.block_from_numeric_id(b))
                                .unwrap();

                            calculate_cannon_addition(&property, x, y, z, &mut system);
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
                            let property = laser_cannon_blocks
                                .get(blocks.block_from_numeric_id(b))
                                .unwrap();

                            calculate_cannon_addition(&property, x, y, z, &mut system);
                        }
                    }
                }
            }

            commands.entity(ev.structure_entity).insert(system);
        }
    }
}

fn update_system() {}

pub fn register<T: StateData + Clone>(app: &mut App, post_loading_state: T, playing_state: T) {
    app.insert_resource(LaserCannonBlocks::default())
        .add_system_set(
            SystemSet::on_enter(post_loading_state).with_system(register_thruster_blocks),
        )
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_bevy_state(playing_state.clone()),
        )
        .add_system_set(SystemSet::on_update(playing_state).with_system(update_system))
        .register_inspectable::<LaserCannonSystem>();
}
