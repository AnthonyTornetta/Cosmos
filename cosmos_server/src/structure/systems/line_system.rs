use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    block::{Block, block_direction::BlockDirection, block_events::BlockEventsSet, block_face::BlockFace, block_rotation::BlockRotation},
    events::{block_events::BlockChangedEvent, structure::structure_event::StructureEventIterator},
    prelude::StructureSystem,
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        coordinates::{BlockCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        events::StructureLoadedEvent,
        systems::{
            StructureSystemImpl, StructureSystemOrdering, StructureSystemType, StructureSystems, StructureSystemsSet,
            line_system::{Line, LineBlocks, LineColorBlock, LineColorProperty, LineProperty, LinePropertyCalculator, LineSystem},
        },
    },
    utils::ecs::MutOrMutRef,
};
use serde::{Serialize, de::DeserializeOwned};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

use super::BlockStructureSystem;

fn block_update_system<T: LineProperty, S: LinePropertyCalculator<T>>(
    mut event: EventReader<BlockChangedEvent>,
    laser_cannon_blocks: Res<LineBlocks<T>>,
    color_blocks: Res<Registry<LineColorBlock>>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut LineSystem<T, S>>,
    mut systems_query: Query<(&mut StructureSystems, &mut StructureSystemOrdering)>,
    mut commands: Commands,
    q_system: Query<&StructureSystem, With<LineSystem<T, S>>>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for (structure, events) in event.read().group_by_structure() {
        let Ok((mut systems, mut sys_ordering)) = systems_query.get_mut(structure) else {
            continue;
        };

        let mut new_system_if_needed = LineSystem::<T, S>::default();

        let mut system = systems
            .query_mut(&mut system_query)
            .map(MutOrMutRef::from)
            .unwrap_or(MutOrMutRef::from(&mut new_system_if_needed));

        for ev in events {
            let old_block = blocks.from_numeric_id(ev.old_block);
            let new_block = blocks.from_numeric_id(ev.new_block);

            if laser_cannon_blocks.get(old_block).is_some() {
                system.remove_block(ev.block.coords());
            }

            if let Some(property) = laser_cannon_blocks.get(new_block) {
                system.add_block(ev.block.coords(), ev.new_block_rotation(), property);
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

        match system {
            MutOrMutRef::Mut(existing_system) => {
                if existing_system.is_empty() {
                    let system = *systems.query(&q_system).expect("This should always exist on a StructureSystem");
                    systems.remove_system(&mut commands, &system, &registry, &mut sys_ordering);
                }
            }
            MutOrMutRef::Ref(new_system) => {
                if !new_system.is_empty() {
                    let (id, _) = systems.add_system(&mut commands, std::mem::take(&mut new_system_if_needed), &registry);
                    if let Some(system_type) = registry.from_id(LineSystem::<T, S>::unlocalized_name())
                        && system_type.is_activatable()
                    {
                        sys_ordering.add_to_next_available(id);
                    }
                }
            }
        }
    }
}

fn structure_loaded_event<T: LineProperty, S: LinePropertyCalculator<T>>(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    color_blocks: Res<Registry<LineColorBlock>>,
    mut commands: Commands,
    line_blocks: Res<LineBlocks<T>>,
    registry: Res<Registry<StructureSystemType>>,
    q_line_system: Query<(), With<LineSystem<T, S>>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            if systems.query(&q_line_system).is_ok() {
                // This system already exists - skip
                info!("System already exsists - skip!");
                continue;
            }

            let mut system = LineSystem::<T, S>::default();

            let mut color_found = false;

            for coords in structure.all_blocks_iter(false) {
                let block = structure.block_at(coords, &blocks);
                let block_rotation = structure.block_rotation(coords);
                if let Some(prop) = line_blocks.get(block) {
                    system.add_block(coords, block_rotation, prop);
                }
                if let Some(color_property) = color_blocks.from_block(block) {
                    color_found = true;
                    system.colors.push((coords, color_property.properties));
                }
            }

            if color_found {
                recalculate_colors(&mut system, None);
            }

            if !system.is_empty() {
                systems.add_system(&mut commands, system, &registry);
            }
        }
    }
}

fn add_colors(mut colors: ResMut<Registry<LineColorBlock>>, blocks: Res<Registry<Block>>) {
    if let Some(block) = blocks.from_id("cosmos:glass_white") {
        colors.insert(block, css::WHITE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_blue") {
        colors.insert(block, css::BLUE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_blue") {
        colors.insert(block, Srgba::hex("2658FE").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_brown") {
        colors.insert(block, Srgba::hex("943D00").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_green") {
        colors.insert(block, css::GREEN.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_green") {
        colors.insert(block, css::DARK_GREEN.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_orange") {
        colors.insert(block, css::ORANGE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_orange") {
        colors.insert(block, Srgba::hex("CCA120").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_pink") {
        colors.insert(block, css::PINK.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_pink") {
        colors.insert(block, Srgba::hex("CC0170").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_purple") {
        colors.insert(block, css::PURPLE.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_purple") {
        colors.insert(block, Srgba::hex("AB1EB6").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_red") {
        colors.insert(block, css::RED.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_red") {
        colors.insert(block, Srgba::hex("AB1EB6").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_yellow") {
        colors.insert(block, css::YELLOW.into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_dark_yellow") {
        colors.insert(block, Srgba::hex("CCA120").unwrap().into());
    }
    if let Some(block) = blocks.from_id("cosmos:glass_mint") {
        colors.insert(block, Srgba::hex("28FF9E").unwrap().into());
    }
}

impl<T: LineProperty, S: LinePropertyCalculator<T>> BlockStructureSystem<T> for LineSystem<T, S> {
    fn add_block(&mut self, block: BlockCoordinate, block_rotation: BlockRotation, prop: &T) {
        let block_direction = block_rotation.direction_of(BlockFace::Front);

        let mut found_line = None;
        // If a structure has two lines like this: (XXXXX XXXXXX) and an X is placed
        // in that space, then those two lines need to be linked toegether into one cannon.
        //
        // If this variable is ever Some index, then the found_line has to be linked with
        // the line at this index.
        let mut link_to = None;

        for (i, line) in self.lines.iter_mut().filter(|x| x.direction == block_direction).enumerate() {
            let delta = block_direction.to_coordinates();

            let start: UnboundBlockCoordinate = line.start.into();

            let block: UnboundBlockCoordinate = block.into();

            // Block is before start
            if start.x - delta.x == block.x && start.y - delta.y == block.y && start.z - delta.z == block.z {
                if found_line.is_some() {
                    link_to = Some(i);
                    break;
                } else {
                    // This should always be >= 0 because a block cannot placed at negative coordinates
                    line.start.x = (start.x - delta.x) as CoordinateType;
                    line.start.y = (start.y - delta.y) as CoordinateType;
                    line.start.z = (start.z - delta.z) as CoordinateType;
                    line.len += 1;
                    line.properties.insert(0, *prop);
                    line.property = S::calculate_property(&line.properties);

                    found_line = Some(i);
                }
            }
            // Block is after end
            else if start.x + delta.x * (line.len as UnboundCoordinateType) == block.x
                && start.y + delta.y * (line.len as UnboundCoordinateType) == block.y
                && start.z + delta.z * (line.len as UnboundCoordinateType) == block.z
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
                let [l1, l2] = self
                    .lines
                    .get_disjoint_mut([l1_i, l2_i])
                    .expect("From and to should never be the same");

                // Must use the one before the other in the line so the properties line up.
                if match l1.direction {
                    BlockDirection::PosX => l1.start.x > l2.start.x,
                    BlockDirection::NegX => l1.start.x < l2.start.x,
                    BlockDirection::PosY => l1.start.y > l2.start.y,
                    BlockDirection::NegY => l1.start.y < l2.start.y,
                    BlockDirection::PosZ => l1.start.z > l2.start.z,
                    BlockDirection::NegZ => l1.start.z < l2.start.z,
                } {
                    std::mem::swap(l1, l2);
                }

                l1.len += l2.len;
                l1.power += l2.power;
                l1.active_blocks.append(&mut l2.active_blocks);

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
            start: block,
            direction: block_direction,
            len: 1,
            properties,
            property,
            color,
            active_blocks: vec![],
            power: 0.0,
        });
    }

    fn remove_block(&mut self, sb: BlockCoordinate) {
        for (i, line) in self.lines.iter_mut().enumerate() {
            line.mark_block_inactive(sb);

            if line.start == sb {
                let (dx, dy, dz) = line.direction.to_i32_tuple();
                line.properties.remove(0);
                line.property = S::calculate_property(&line.properties);
                line.start.x = (line.start.x as i32 + dx) as CoordinateType;
                line.start.y = (line.start.y as i32 + dy) as CoordinateType;
                line.start.z = (line.start.z as i32 + dz) as CoordinateType;
                line.len -= 1;

                if line.len == 0 {
                    self.lines.swap_remove(i);
                }
                return;
            } else if line.end() == sb {
                line.properties.pop();
                line.property = S::calculate_property(&line.properties);
                line.len -= 1;
                if line.len == 0 {
                    self.lines.swap_remove(i);
                }
                return;
            } else if line.within(&sb) {
                let l1_len = match line.direction {
                    BlockDirection::PosX => sb.x - line.start.x,
                    BlockDirection::NegX => line.start.x - sb.x,
                    BlockDirection::PosY => sb.y - line.start.y,
                    BlockDirection::NegY => line.start.y - sb.y,
                    BlockDirection::PosZ => sb.z - line.start.z,
                    BlockDirection::NegZ => line.start.z - sb.z,
                };

                let l2_len = line.len as CoordinateType - l1_len - 1;

                let mut l1_props = Vec::with_capacity(l1_len as usize);
                let mut l2_props = Vec::with_capacity(l2_len as usize);

                let percent_power_l1 = l1_len as f32 / line.len as f32;
                let percent_power_l2 = l2_len as f32 / line.len as f32;

                for prop in line.properties.iter().take(l1_len as usize) {
                    l1_props.push(*prop);
                }

                for prop in line.properties.iter().skip(l1_len as usize + 1) {
                    l2_props.push(*prop);
                }

                let l1_property = S::calculate_property(&l1_props);

                // we are within a line, so split it into two seperate ones
                let mut l1 = Line {
                    start: line.start,
                    direction: line.direction,
                    len: l1_len,
                    properties: l1_props,
                    property: l1_property,
                    color: line.color,
                    power: percent_power_l1 * line.power,
                    active_blocks: vec![],
                };

                l1.active_blocks = line
                    .active_blocks
                    .iter()
                    .filter(|x| l1.within(x))
                    .copied()
                    .collect::<Vec<BlockCoordinate>>();

                let (dx, dy, dz) = line.direction.to_i32_tuple();

                let dist = l1_len as i32 + 1;

                let l2_property = S::calculate_property(&l2_props);
                let mut l2 = Line {
                    start: BlockCoordinate::new(
                        (line.start.x as i32 + dx * dist) as CoordinateType,
                        (line.start.y as i32 + dy * dist) as CoordinateType,
                        (line.start.z as i32 + dz * dist) as CoordinateType,
                    ),
                    direction: line.direction,
                    len: l2_len,
                    properties: l2_props,
                    property: l2_property,
                    color: line.color,
                    power: percent_power_l2 * line.power,
                    active_blocks: vec![], // this will probably have to be calculated later.
                };

                l2.active_blocks = line
                    .active_blocks
                    .iter()
                    .filter(|x| l2.within(x))
                    .copied()
                    .collect::<Vec<BlockCoordinate>>();

                self.lines[i] = l1;
                self.lines.push(l2);

                return;
            }
        }
    }
}

fn is_in_line_with(testing_block: BlockCoordinate, direction: BlockDirection, line_coord: BlockCoordinate) -> bool {
    match direction {
        BlockDirection::PosX => line_coord.x >= testing_block.x && line_coord.y == testing_block.y && line_coord.z == testing_block.z,
        BlockDirection::NegX => line_coord.x <= testing_block.x && line_coord.y == testing_block.y && line_coord.z == testing_block.z,
        BlockDirection::PosY => line_coord.x == testing_block.x && line_coord.y >= testing_block.y && line_coord.z == testing_block.z,
        BlockDirection::NegY => line_coord.x == testing_block.x && line_coord.y <= testing_block.y && line_coord.z == testing_block.z,
        BlockDirection::PosZ => line_coord.x == testing_block.x && line_coord.y == testing_block.y && line_coord.z >= testing_block.z,
        BlockDirection::NegZ => line_coord.x == testing_block.x && line_coord.y == testing_block.y && line_coord.z <= testing_block.z,
    }
}

fn calculate_color_for_line<T: LineProperty, S: LinePropertyCalculator<T>>(
    line_system: &LineSystem<T, S>,
    block: BlockCoordinate,
    direction: BlockDirection,
) -> Option<Color> {
    let colors = line_system
        .colors
        .iter()
        .filter(|x| is_in_line_with(block, direction, x.0))
        .map(|x| x.1)
        .collect::<Vec<LineColorProperty>>();

    if !colors.is_empty() {
        let len = colors.len();

        let averaged_color = colors
            .into_iter()
            .map(|x| Srgba::from(x.color))
            .reduce(|x, y| Srgba {
                red: x.red + y.red,
                green: x.green + y.green,
                blue: x.blue + y.blue,
                alpha: 1.0,
            })
            .unwrap_or(css::WHITE);

        Some(
            Srgba {
                red: averaged_color.red / len as f32,
                green: averaged_color.green / len as f32,
                blue: averaged_color.blue / len as f32,
                alpha: 1.0,
            }
            .into(),
        )
    } else {
        None
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
            .map(|changed_coordinate| is_in_line_with(line.start, line.direction, changed_coordinate))
            .unwrap_or(false)
    }) {
        line.color = calculate_color_for_line(line_system, line.start, line.direction);
    }

    line_system.lines = lines;
}

impl<T: LineProperty + DeserializeOwned + Serialize, S: LinePropertyCalculator<T>> DefaultPersistentComponent for LineSystem<T, S> {}

/// Adds all the functions a line system needs to operate
pub fn add_line_system<T: LineProperty + DeserializeOwned + Serialize, S: LinePropertyCalculator<T>>(app: &mut App) {
    make_persistent::<LineSystem<T, S>>(app);

    app.add_systems(
        Update,
        (
            structure_loaded_event::<T, S>
                .in_set(StructureSystemsSet::InitSystems)
                .ambiguous_with(StructureSystemsSet::InitSystems),
            block_update_system::<T, S>
                .in_set(BlockEventsSet::ProcessEvents)
                .in_set(StructureSystemsSet::UpdateSystemsBlocks),
        )
            .run_if(in_state(GameState::Playing)),
    )
    .init_resource::<LineBlocks<T>>();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::PostLoading), add_colors);
}
