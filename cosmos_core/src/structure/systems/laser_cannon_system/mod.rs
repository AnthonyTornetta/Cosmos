use std::{
    mem::swap,
    ops::{Add, AddAssign, SubAssign},
};

use bevy::{ecs::schedule::StateData, prelude::*, utils::HashMap};
use bevy_inspector_egui::{Inspectable, RegisterInspectable};
use iyes_loopless::prelude::*;

use crate::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    registry::{identifiable::Identifiable, Registry},
    structure::{events::StructureLoadedEvent, Structure, StructureBlock},
};

use super::Systems;

#[derive(Default, Inspectable, Clone, Copy)]
pub struct LaserCannonProperty {
    pub energy_per_shot: f32,
}

impl SubAssign for LaserCannonProperty {
    fn sub_assign(&mut self, rhs: Self) {
        self.energy_per_shot -= rhs.energy_per_shot;
    }
}

impl AddAssign for LaserCannonProperty {
    fn add_assign(&mut self, rhs: Self) {
        self.energy_per_shot += rhs.energy_per_shot;
    }
}

impl Add for LaserCannonProperty {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self {
            energy_per_shot: self.energy_per_shot + rhs.energy_per_shot,
        }
    }
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
pub struct Line {
    pub start: StructureBlock,
    pub direction: BlockFace,
    pub len: usize,
    pub property: LaserCannonProperty,
    properties: Vec<LaserCannonProperty>,
}

impl Line {
    #[inline]
    pub fn end(&self) -> StructureBlock {
        let (dx, dy, dz) = self.direction.direction();
        let delta = self.len as i32 - 1;
        StructureBlock {
            x: (self.start.x as i32 + delta * dx) as usize,
            y: (self.start.y as i32 + delta * dy) as usize,
            z: (self.start.z as i32 + delta * dz) as usize,
        }
    }

    pub fn within(&self, sb: &StructureBlock) -> bool {
        match self.direction {
            BlockFace::Front => {
                sb.x == self.start.x
                    && sb.y == self.start.y
                    && (sb.z >= self.start.z && sb.z < self.start.z + self.len)
            }
            BlockFace::Back => {
                sb.x == self.start.x
                    && sb.y == self.start.y
                    && (sb.z <= self.start.z && sb.z > self.start.z - self.len)
            }
            BlockFace::Right => {
                sb.z == self.start.z
                    && sb.y == self.start.y
                    && (sb.x >= self.start.x && sb.x < self.start.x + self.len)
            }
            BlockFace::Left => {
                sb.z == self.start.z
                    && sb.y == self.start.y
                    && (sb.x <= self.start.x && sb.x > self.start.x - self.len)
            }
            BlockFace::Top => {
                sb.x == self.start.x
                    && sb.z == self.start.z
                    && (sb.y >= self.start.y && sb.y < self.start.y + self.len)
            }
            BlockFace::Bottom => {
                sb.x == self.start.x
                    && sb.z == self.start.z
                    && (sb.y <= self.start.y && sb.y > self.start.y - self.len)
            }
        }
    }
}

#[derive(Component, Default, Inspectable)]
pub struct LaserCannonSystem {
    pub lines: Vec<Line>,
    pub last_shot_time: f32,
}

impl LaserCannonSystem {
    fn block_removed(&mut self, sb: &StructureBlock) {
        for (i, line) in self.lines.iter_mut().enumerate() {
            if line.start == *sb {
                let (dx, dy, dz) = line.direction.direction();

                line.start.x = (line.start.x as i32 + dx) as usize;
                line.start.y = (line.start.y as i32 + dy) as usize;
                line.start.z = (line.start.z as i32 + dz) as usize;
                line.len -= 1;

                line.property -= line.properties.remove(0);

                if line.len == 0 {
                    self.lines.swap_remove(i);
                    return;
                }
            } else if line.end() == *sb {
                line.len -= 1;

                line.property -= line.properties.pop().expect("At least one");

                if line.len == 0 {
                    self.lines.swap_remove(i);
                    return;
                }
            } else {
                if line.within(sb) {
                    let l1_len = match line.direction {
                        BlockFace::Front => sb.z - line.start.z,
                        BlockFace::Back => line.start.z - sb.z,
                        BlockFace::Right => sb.x - line.start.x,
                        BlockFace::Left => line.start.x - sb.x,
                        BlockFace::Top => sb.y - line.start.y,
                        BlockFace::Bottom => line.start.y - sb.y,
                    };

                    let l2_len = line.len - l1_len - 1;

                    let mut l1_total_prop = LaserCannonProperty::default();
                    let mut l2_total_prop = LaserCannonProperty::default();

                    let mut l1_props = Vec::with_capacity(l1_len);
                    let mut l2_props = Vec::with_capacity(l2_len);

                    for prop in line.properties.iter().take(l1_len) {
                        l1_total_prop.energy_per_shot += prop.energy_per_shot;
                        l1_props.push(*prop);
                    }

                    for prop in line.properties.iter().skip(l1_len + 1) {
                        l2_total_prop.energy_per_shot += prop.energy_per_shot;
                        l2_props.push(*prop);
                    }

                    // we are within a line, so split it into two seperate ones
                    let l1 = Line {
                        start: line.start,
                        direction: line.direction,
                        len: l1_len,
                        property: l1_total_prop,
                        properties: l1_props,
                    };

                    let (dx, dy, dz) = line.direction.direction();

                    let dist = l1_len as i32 + 1;

                    let l2 = Line {
                        start: StructureBlock {
                            x: (line.start.x as i32 + dx * dist) as usize,
                            y: (line.start.y as i32 + dy * dist) as usize,
                            z: (line.start.z as i32 + dz * dist) as usize,
                        },
                        direction: line.direction,
                        len: line.len - l1_len - 1,
                        property: l2_total_prop,
                        properties: l2_props,
                    };

                    self.lines[i] = l1;
                    self.lines.push(l2);

                    return;
                }
            }
        }
    }

    fn block_added(&mut self, prop: &LaserCannonProperty, block: &StructureBlock) {
        // Always assume +z direction (for now)
        let block_direction = BlockFace::Front; // eventually take this as argument

        let mut found_line = None;
        let mut link_to = None;

        for (i, line) in self
            .lines
            .iter_mut()
            .filter(|x| x.direction == block_direction)
            .enumerate()
        {
            let (dx, dy, dz) = line.direction.direction();

            let (sx, sy, sz) = (
                line.start.x as i32,
                line.start.y as i32,
                line.start.z as i32,
            );

            let (bx, by, bz) = (block.x as i32, block.y as i32, block.z as i32);

            // println!(
            //     "Checking ({}, {}, {}) -> ({}, {}, {}) for ({}, {}, {})",
            //     sx,
            //     sy,
            //     sz,
            //     sx + line.len as i32 * dx,
            //     sy + line.len as i32 * dy,
            //     sz + line.len as i32 * dz,
            //     bx,
            //     by,
            //     bz
            // );

            // Block is before start
            if sx - dx == bx && sy - dy == by && sz - dz == bz {
                if found_line.is_some() {
                    link_to = Some(i);
                    break;
                } else {
                    line.start.x -= dx as usize;
                    line.start.y -= dy as usize;
                    line.start.z -= dz as usize;
                    line.len += 1;
                    line.property += *prop;
                    line.properties.insert(0, *prop);

                    found_line = Some(i);
                }
            }
            // Block is after end
            else if sx + dx * (line.len as i32) == bx
                && sy + dy * (line.len as i32) == by
                && sz + dz * (line.len as i32) == bz
            {
                if found_line.is_some() {
                    link_to = Some(i);
                    break;
                } else {
                    line.len += 1;
                    line.property += *prop;
                    line.properties.push(*prop);

                    found_line = Some(i);
                }
            }
        }

        if let Some(l1_i) = found_line {
            if let Some(l2_i) = link_to {
                let [mut l1, l2] = self
                    .lines
                    .get_many_mut([l1_i, l2_i])
                    .expect("From and to should never be the same");

                // Must use the one before the other in the line so the properties line up
                if match l1.direction {
                    BlockFace::Front => l1.start.z > l2.start.z,
                    BlockFace::Back => l1.start.z < l2.start.z,
                    BlockFace::Right => l1.start.x > l2.start.x,
                    BlockFace::Left => l1.start.x < l2.start.x,
                    BlockFace::Top => l1.start.y > l2.start.y,
                    BlockFace::Bottom => l1.start.y < l2.start.y,
                } {
                    swap(l1, l2);
                }

                l1.len = l1.len + l2.len;
                l1.property += l2.property;

                l1.properties.append(&mut l2.properties);

                self.lines.swap_remove(l2_i);
            }
            return;
        }

        // If gotten here, no suitable line was found

        self.lines.push(Line {
            start: *block,
            direction: block_direction,
            len: 1,
            property: *prop,
            properties: vec![*prop],
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
    mut event: EventReader<BlockChangedEvent>,
    laser_cannon_blocks: Res<LaserCannonBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut LaserCannonSystem>,
    systems_query: Query<&Systems>,
) {
    for ev in event.iter() {
        if let Ok(mut system) = systems_query
            .get(ev.structure_entity)
            .expect("Structure should have Systems component")
            .query_mut(&mut system_query)
        {
            if laser_cannon_blocks
                .get(blocks.from_numeric_id(ev.old_block))
                .is_some()
            {
                system.block_removed(&ev.block);
            }

            if let Some(property) = laser_cannon_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(property, &ev.block);
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    laser_cannon_blocks: Res<LaserCannonBlocks>,
) {
    for ev in event_reader.iter() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = LaserCannonSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = laser_cannon_blocks.get(&block.block(structure, &blocks)) {
                    system.block_added(prop, &block);
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
    app.insert_resource(LaserCannonBlocks::default())
        .add_system_set(SystemSet::on_enter(post_loading_state).with_system(register_laser_blocks))
        .add_system_to_stage(
            CoreStage::PostUpdate,
            block_update_system.run_in_bevy_state(playing_state),
        )
        .add_system_set(SystemSet::on_update(playing_state).with_system(structure_loaded_event))
        // .add_system_set(SystemSet::on_update(playing_state).with_system(update_laser))
        .register_inspectable::<LaserCannonSystem>();
}
//
