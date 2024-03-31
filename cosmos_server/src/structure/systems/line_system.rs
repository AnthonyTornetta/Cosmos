use bevy::{
    app::{App, Update},
    ecs::{
        event::EventReader,
        schedule::{common_conditions::in_state, IntoSystemConfigs, OnEnter},
        system::{Commands, Query, Res, ResMut},
    },
    render::color::Color,
};
use cosmos_core::{
    block::{Block, BlockFace, BlockRotation},
    events::block_events::BlockChangedEvent,
    registry::Registry,
    structure::{
        coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        events::StructureLoadedEvent,
        loading::StructureLoadingSet,
        structure_block::StructureBlock,
        systems::{
            line_system::{Line, LineBlocks, LineColorBlock, LineColorProperty, LineProperty, LinePropertyCalculator, LineSystem},
            StructureSystemType, Systems,
        },
        Structure,
    },
};

use crate::state::GameState;

use super::BlockStructureSystem;

fn block_update_system<T: LineProperty, S: LinePropertyCalculator<T>>(
    mut event: EventReader<BlockChangedEvent>,
    laser_cannon_blocks: Res<LineBlocks<T>>,
    color_blocks: Res<Registry<LineColorBlock>>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut LineSystem<T, S>>,
    systems_query: Query<&Systems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                let old_block = blocks.from_numeric_id(ev.old_block);
                let new_block = blocks.from_numeric_id(ev.new_block);

                if laser_cannon_blocks.get(old_block).is_some() {
                    system.remove_block(&ev.block);
                }

                if let Some(property) = laser_cannon_blocks.get(new_block) {
                    system.add_block(&ev.block, ev.new_block_rotation, property);
                }

                let mut recalc = false;
                if color_blocks.from_block(old_block).is_some() {
                    system.colors.retain(|x| x.0 != ev.block.coords());
                    recalc = true;
                }

                if let Some(color_property) = color_blocks.from_block(new_block) {
                    system.colors.push((ev.block.coords(), color_property.properties));
                    recalc = true;
                }
                if recalc {
                    recalculate_colors(&mut system, Some(ev.block.coords()));
                }
            }
        }
    }
}

fn structure_loaded_event<T: LineProperty, S: LinePropertyCalculator<T>>(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    color_blocks: Res<Registry<LineColorBlock>>,
    mut commands: Commands,
    line_blocks: Res<LineBlocks<T>>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = LineSystem::<T, S>::default();

            let mut color_found = false;

            for structure_block in structure.all_blocks_iter(false) {
                let block = structure_block.block(structure, &blocks);
                let block_rotation = structure.block_rotation(structure_block.coords());
                if let Some(prop) = line_blocks.get(block) {
                    system.add_block(&structure_block, block_rotation, prop);
                }
                if let Some(color_property) = color_blocks.from_block(block) {
                    color_found = true;
                    system.colors.push((structure_block.coords(), color_property.properties));
                }
            }

            if color_found {
                recalculate_colors(&mut system, None);
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

fn add_colors(mut colors: ResMut<Registry<LineColorBlock>>, blocks: Res<Registry<Block>>) {
    if let Some(block) = blocks.from_id("cosmos:glass_white") {
        colors.insert(block, Color::WHITE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_blue") {
        colors.insert(block, Color::BLUE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_blue") {
        colors.insert(block, Color::hex("2658FE").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_brown") {
        colors.insert(block, Color::hex("943D00").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_green") {
        colors.insert(block, Color::GREEN.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_green") {
        colors.insert(block, Color::DARK_GREEN.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_orange") {
        colors.insert(block, Color::ORANGE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_orange") {
        colors.insert(block, Color::hex("CCA120").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_pink") {
        colors.insert(block, Color::PINK.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_pink") {
        colors.insert(block, Color::hex("CC0170").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_purple") {
        colors.insert(block, Color::PURPLE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_purple") {
        colors.insert(block, Color::hex("AB1EB6").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_red") {
        colors.insert(block, Color::RED.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_red") {
        colors.insert(block, Color::hex("AB1EB6").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_yellow") {
        colors.insert(block, Color::YELLOW.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_yellow") {
        colors.insert(block, Color::hex("CCA120").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_mint") {
        colors.insert(block, Color::hex("28FF9E").unwrap().into());
    }
}

impl<T: LineProperty, S: LinePropertyCalculator<T>> BlockStructureSystem<T> for LineSystem<T, S> {
    fn add_block(&mut self, block: &StructureBlock, block_rotation: BlockRotation, prop: &T) {
        let block_direction = block_rotation.which_face_is(BlockFace::Front);

        let mut found_line = None;
        // If a structure has two lines like this: (XXXXX XXXXXX) and an X is placed
        // in that space, then those two lines need to be linked toegether into one cannon.
        //
        // If this variable is ever Some index, then the found_line has to be linked with
        // the line at this index.
        let mut link_to = None;

        for (i, line) in self.lines.iter_mut().filter(|x| x.direction == block_direction).enumerate() {
            let d = block_direction.direction_coordinates();

            let start: UnboundBlockCoordinate = line.start.coords().into();

            let block: UnboundBlockCoordinate = block.coords().into();

            // Block is before start
            if start.x - d.x == block.x && start.y - d.y == block.y && start.z - d.z == block.z {
                if found_line.is_some() {
                    link_to = Some(i);
                    break;
                } else {
                    // This should always be >= 0 because a block cannot placed at negative coordinates
                    line.start.x = (start.x - d.x) as CoordinateType;
                    line.start.y = (start.y - d.y) as CoordinateType;
                    line.start.z = (start.z - d.z) as CoordinateType;
                    line.len += 1;
                    line.properties.insert(0, *prop);
                    line.property = S::calculate_property(&line.properties);

                    found_line = Some(i);
                }
            }
            // Block is after end
            else if start.x + d.x * (line.len as UnboundCoordinateType) == block.x
                && start.y + d.y * (line.len as UnboundCoordinateType) == block.y
                && start.z + d.z * (line.len as UnboundCoordinateType) == block.z
            {
                if found_line.is_some() {
                    link_to = Some(i);
                    break;
                } else {
                    line.len += 1;
                    line.properties.push(*prop);
                    line.property = S::calculate_property(&line.properties);

                    found_line = Some(i);
                }
            }
        }

        if let Some(l1_i) = found_line {
            if let Some(l2_i) = link_to {
                let [l1, l2] = self.lines.get_many_mut([l1_i, l2_i]).expect("From and to should never be the same");

                // Must use the one before the other in the line so the properties line up
                if match l1.direction {
                    BlockFace::Front => l1.start.z > l2.start.z,
                    BlockFace::Back => l1.start.z < l2.start.z,
                    BlockFace::Right => l1.start.x > l2.start.x,
                    BlockFace::Left => l1.start.x < l2.start.x,
                    BlockFace::Top => l1.start.y > l2.start.y,
                    BlockFace::Bottom => l1.start.y < l2.start.y,
                } {
                    std::mem::swap(l1, l2);
                }

                l1.len += l2.len;

                l1.properties.append(&mut l2.properties);
                l1.property = S::calculate_property(&l1.properties);

                self.lines.swap_remove(l2_i);
            }
            return;
        }

        // If gotten here, no suitable line was found

        let color = calculate_color_for_line(self, block, block_direction);

        let properties = vec![*prop];
        let property = S::calculate_property(&properties);

        self.lines.push(Line {
            start: *block,
            direction: block_direction,
            len: 1,
            properties,
            property,
            color,
        });
    }

    fn remove_block(&mut self, sb: &StructureBlock) {
        for (i, line) in self.lines.iter_mut().enumerate() {
            if line.start == *sb {
                let (dx, dy, dz) = line.direction.direction();

                line.start.x = (line.start.x as i32 + dx) as CoordinateType;
                line.start.y = (line.start.y as i32 + dy) as CoordinateType;
                line.start.z = (line.start.z as i32 + dz) as CoordinateType;
                line.len -= 1;

                if line.len == 0 {
                    self.lines.swap_remove(i);
                    return;
                }
            } else if line.end() == *sb {
                line.len -= 1;

                if line.len == 0 {
                    self.lines.swap_remove(i);
                    return;
                }
            } else if line.within(sb) {
                let l1_len = match line.direction {
                    BlockFace::Front => sb.z - line.start.z,
                    BlockFace::Back => line.start.z - sb.z,
                    BlockFace::Right => sb.x - line.start.x,
                    BlockFace::Left => line.start.x - sb.x,
                    BlockFace::Top => sb.y - line.start.y,
                    BlockFace::Bottom => line.start.y - sb.y,
                };

                let l2_len = line.len as CoordinateType - l1_len - 1;

                let mut l1_props = Vec::with_capacity(l1_len as usize);
                let mut l2_props = Vec::with_capacity(l2_len as usize);

                for prop in line.properties.iter().take(l1_len as usize) {
                    l1_props.push(*prop);
                }

                for prop in line.properties.iter().skip(l1_len as usize + 1) {
                    l2_props.push(*prop);
                }

                let l1_property = S::calculate_property(&l1_props);

                // we are within a line, so split it into two seperate ones
                let l1 = Line {
                    start: line.start,
                    direction: line.direction,
                    len: l1_len,
                    properties: l1_props,
                    property: l1_property,
                    color: line.color,
                };

                let (dx, dy, dz) = line.direction.direction();

                let dist = l1_len as i32 + 1;

                let l2_property = S::calculate_property(&l2_props);
                let l2 = Line {
                    start: StructureBlock::new(BlockCoordinate::new(
                        (line.start.x as i32 + dx * dist) as CoordinateType,
                        (line.start.y as i32 + dy * dist) as CoordinateType,
                        (line.start.z as i32 + dz * dist) as CoordinateType,
                    )),
                    direction: line.direction,
                    len: line.len - l1_len - 1,
                    properties: l2_props,
                    property: l2_property,
                    color: line.color,
                };

                self.lines[i] = l1;
                self.lines.push(l2);

                return;
            }
        }
    }
}

fn is_in_line_with(testing_block: &StructureBlock, direction: BlockFace, line_coord: &BlockCoordinate) -> bool {
    match direction {
        BlockFace::Front => line_coord.x == testing_block.x && line_coord.y == testing_block.y && line_coord.z >= testing_block.z,
        BlockFace::Back => line_coord.x == testing_block.x && line_coord.y == testing_block.y && line_coord.z <= testing_block.z,
        BlockFace::Top => line_coord.x == testing_block.x && line_coord.y >= testing_block.y && line_coord.z == testing_block.z,
        BlockFace::Bottom => line_coord.x == testing_block.x && line_coord.y <= testing_block.y && line_coord.z == testing_block.z,
        BlockFace::Right => line_coord.x >= testing_block.x && line_coord.y == testing_block.y && line_coord.z == testing_block.z,
        BlockFace::Left => line_coord.x <= testing_block.x && line_coord.y == testing_block.y && line_coord.z == testing_block.z,
    }
}

fn calculate_color_for_line<T: LineProperty, S: LinePropertyCalculator<T>>(
    line_system: &LineSystem<T, S>,
    block: &StructureBlock,
    direction: BlockFace,
) -> Color {
    let colors = line_system
        .colors
        .iter()
        .filter(|x| is_in_line_with(block, direction, &x.0))
        .map(|x| x.1)
        .collect::<Vec<LineColorProperty>>();

    let len = colors.len();
    let averaged_color = colors
        .into_iter()
        .map(|x| x.color)
        .reduce(|x, y| Color::rgb(x.r() + y.r(), x.g() + y.g(), x.b() + y.b()))
        .unwrap_or(Color::WHITE);

    if len != 0 {
        Color::rgb(
            averaged_color.r() / len as f32,
            averaged_color.g() / len as f32,
            averaged_color.b() / len as f32,
        )
    } else {
        averaged_color
    }
}

fn recalculate_colors<T: LineProperty, S: LinePropertyCalculator<T>>(
    line_system: &mut LineSystem<T, S>,
    changed_coordinate: Option<BlockCoordinate>,
) {
    // Gets around borrow checker being a total buzzkill
    let mut lines = std::mem::take(&mut line_system.lines);

    for line in lines.iter_mut().filter(|line| {
        changed_coordinate
            .map(|changed_coordinate| is_in_line_with(&line.start, line.direction, &changed_coordinate))
            .unwrap_or(false)
    }) {
        line.color = calculate_color_for_line(line_system, &line.start, line.direction);
    }

    line_system.lines = lines;
}

/// Adds all the functions a line system needs to operate
pub fn add_line_system<T: LineProperty, S: LinePropertyCalculator<T>>(app: &mut App) {
    app.add_systems(
        Update,
        (
            structure_loaded_event::<T, S>.in_set(StructureLoadingSet::StructureLoaded),
            block_update_system::<T, S>,
        )
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<LineBlocks<T>>();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), add_colors);
}
