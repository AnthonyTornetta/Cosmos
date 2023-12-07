//! Represents all the laser cannons on this structure

use std::{
    mem::swap,
    ops::{Add, AddAssign, SubAssign},
};

use bevy::{prelude::*, reflect::Reflect, utils::HashMap};

use crate::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{
        coordinates::{BlockCoordinate, CoordinateType},
        events::StructureLoadedEvent,
        loading::StructureLoadingSet,
        Structure, StructureBlock,
    },
};

use super::Systems;

#[derive(Default, Reflect, Clone, Copy)]
/// Every block that is a laser cannon should have this property
pub struct LaserCannonProperty {
    /// How much energy is consumed per shot
    pub energy_per_shot: f32,
}

#[derive(Default, Reflect, Clone, Copy)]
/// Every block that will change the color of laser cannons should have this property
pub struct LaserCannonColorProperty {
    /// The color this will change the laser to
    pub color: Color,
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

#[derive(Clone)]
/// The wrapper that ties a block to its alser cannon color properties
pub struct LaserCannonColorBlock {
    id: u16,
    unlocalized_name: String,

    /// The color properties of this block
    pub properties: LaserCannonColorProperty,
}

impl From<Color> for LaserCannonColorProperty {
    fn from(color: Color) -> Self {
        Self { color }
    }
}

impl Identifiable for LaserCannonColorBlock {
    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }
}

impl LaserCannonColorBlock {
    /// Creates a new laser cannon color block entry
    ///
    /// You can also use the `insert` method in the `Registry<LaserCannonColorBlock>` if that is easier.
    pub fn new(block: &Block, properties: LaserCannonColorProperty) -> Self {
        Self {
            properties,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        }
    }
}

impl Registry<LaserCannonColorBlock> {
    /// Gets the corrusponding properties if there is an entry for this block
    pub fn from_block(&self, block: &Block) -> Option<&LaserCannonColorBlock> {
        self.from_id(block.unlocalized_name())
    }

    /// Inserts a block with the specified properties
    pub fn insert(&mut self, block: &Block, properties: LaserCannonColorProperty) {
        self.register(LaserCannonColorBlock::new(block, properties));
    }
}

impl LaserCannonBlocks {
    pub fn insert(&mut self, block: &Block, cannon_property: LaserCannonProperty) {
        self.blocks.insert(block.id(), cannon_property);
    }

    pub fn get(&self, block: &Block) -> Option<&LaserCannonProperty> {
        self.blocks.get(&block.id())
    }
}

#[derive(Reflect)]
/// Represents a line of laser cannons.
///
/// All laser cannons in this line are facing the same direction.
pub struct Line {
    /// The block at the start
    pub start: StructureBlock,
    /// The direction this line is facing
    pub direction: BlockFace,
    /// How many blocks this line has
    pub len: CoordinateType,
    /// The property of all the blocks in this line
    pub property: LaserCannonProperty,
    /// The color of the laser
    pub color: Color,

    /// All the properties of the laser cannons in this line
    properties: Vec<LaserCannonProperty>,
}

impl Line {
    #[inline]
    /// Returns the ending structure block
    pub fn end(&self) -> StructureBlock {
        let (dx, dy, dz) = self.direction.direction();
        let delta = self.len as i32 - 1;

        StructureBlock::new(BlockCoordinate::new(
            (self.start.x as i32 + delta * dx) as CoordinateType,
            (self.start.y as i32 + delta * dy) as CoordinateType,
            (self.start.z as i32 + delta * dz) as CoordinateType,
        ))
    }

    /// Returns true if a structure block is within this line
    pub fn within(&self, sb: &StructureBlock) -> bool {
        match self.direction {
            BlockFace::Front => sb.x == self.start.x && sb.y == self.start.y && (sb.z >= self.start.z && sb.z < self.start.z + self.len),
            BlockFace::Back => sb.x == self.start.x && sb.y == self.start.y && (sb.z <= self.start.z && sb.z > self.start.z - self.len),
            BlockFace::Right => sb.z == self.start.z && sb.y == self.start.y && (sb.x >= self.start.x && sb.x < self.start.x + self.len),
            BlockFace::Left => sb.z == self.start.z && sb.y == self.start.y && (sb.x <= self.start.x && sb.x > self.start.x - self.len),
            BlockFace::Top => sb.x == self.start.x && sb.z == self.start.z && (sb.y >= self.start.y && sb.y < self.start.y + self.len),
            BlockFace::Bottom => sb.x == self.start.x && sb.z == self.start.z && (sb.y <= self.start.y && sb.y > self.start.y - self.len),
        }
    }
}

#[derive(Component, Default, Reflect)]
/// Represents all the laser cannons that are within this structure
pub struct LaserCannonSystem {
    /// All the lins that there are
    pub lines: Vec<Line>,
    /// Any color changers that are placed on this structure
    pub colors: Vec<(BlockCoordinate, LaserCannonColorProperty)>,
    /// The time since this system was last fired.
    pub last_shot_time: f32,
}

fn is_in_line_with(block: &StructureBlock, direction: BlockFace, coord: &BlockCoordinate) -> bool {
    match direction {
        BlockFace::Front => coord.x == block.x && coord.y == block.y && coord.z <= block.z,
        BlockFace::Back => coord.x == block.x && coord.y == block.y && coord.z >= block.z,
        _ => todo!(),
    }
}

impl LaserCannonSystem {
    fn calculate_color_for_line(&self, block: &StructureBlock, direction: BlockFace) -> Color {
        let colors = self
            .colors
            .iter()
            .filter(|x| is_in_line_with(block, direction, &x.0))
            .map(|x| x.1)
            .collect::<Vec<LaserCannonColorProperty>>();

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

    fn recalculate_colors(&mut self, changed_coordinate: Option<BlockCoordinate>) {
        // Gets around borrow checker being a total buzzkill
        let mut lines = std::mem::take(&mut self.lines);

        for line in lines.iter_mut().filter(|line| {
            changed_coordinate
                .map(|changed_coordinate| is_in_line_with(&line.start, line.direction, &changed_coordinate))
                .unwrap_or(false)
        }) {
            line.color = self.calculate_color_for_line(&line.start, line.direction);
        }

        self.lines = lines;
    }

    fn block_removed(&mut self, sb: &StructureBlock) {
        for (i, line) in self.lines.iter_mut().enumerate() {
            if line.start == *sb {
                let (dx, dy, dz) = line.direction.direction();

                line.start.x = (line.start.x as i32 + dx) as CoordinateType;
                line.start.y = (line.start.y as i32 + dy) as CoordinateType;
                line.start.z = (line.start.z as i32 + dz) as CoordinateType;
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

                let mut l1_total_prop = LaserCannonProperty::default();
                let mut l2_total_prop = LaserCannonProperty::default();

                let mut l1_props = Vec::with_capacity(l1_len as usize);
                let mut l2_props = Vec::with_capacity(l2_len as usize);

                for prop in line.properties.iter().take(l1_len as usize) {
                    l1_total_prop.energy_per_shot += prop.energy_per_shot;
                    l1_props.push(*prop);
                }

                for prop in line.properties.iter().skip(l1_len as usize + 1) {
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
                    color: line.color,
                };

                let (dx, dy, dz) = line.direction.direction();

                let dist = l1_len as i32 + 1;

                let l2 = Line {
                    start: StructureBlock::new(BlockCoordinate::new(
                        (line.start.x as i32 + dx * dist) as CoordinateType,
                        (line.start.y as i32 + dy * dist) as CoordinateType,
                        (line.start.z as i32 + dz * dist) as CoordinateType,
                    )),
                    direction: line.direction,
                    len: line.len - l1_len - 1,
                    property: l2_total_prop,
                    properties: l2_props,
                    color: line.color,
                };

                self.lines[i] = l1;
                self.lines.push(l2);

                return;
            }
        }
    }

    fn block_added(&mut self, prop: &LaserCannonProperty, block: &StructureBlock) {
        // Always assume +z direction (for now)
        let block_direction = BlockFace::Front; // eventually take this as argument

        let mut found_line = None;
        let mut link_to = None;

        for (i, line) in self.lines.iter_mut().filter(|x| x.direction == block_direction).enumerate() {
            let (dx, dy, dz) = line.direction.direction();

            let (sx, sy, sz) = (line.start.x as i32, line.start.y as i32, line.start.z as i32);

            let (bx, by, bz) = (block.x as i32, block.y as i32, block.z as i32);

            // Block is before start
            if sx - dx == bx && sy - dy == by && sz - dz == bz {
                if found_line.is_some() {
                    link_to = Some(i);
                    break;
                } else {
                    line.start.x -= dx as CoordinateType;
                    line.start.y -= dy as CoordinateType;
                    line.start.z -= dz as CoordinateType;
                    line.len += 1;
                    line.property += *prop;
                    line.properties.insert(0, *prop);

                    found_line = Some(i);
                }
            }
            // Block is after end
            else if sx + dx * (line.len as i32) == bx && sy + dy * (line.len as i32) == by && sz + dz * (line.len as i32) == bz {
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
                    swap(l1, l2);
                }

                l1.len += l2.len;
                l1.property += l2.property;

                l1.properties.append(&mut l2.properties);

                self.lines.swap_remove(l2_i);
            }
            return;
        }

        // If gotten here, no suitable line was found

        let color = self.calculate_color_for_line(block, block_direction);

        self.lines.push(Line {
            start: *block,
            direction: block_direction,
            len: 1,
            property: *prop,
            properties: vec![*prop],
            color,
        });
    }
}

fn register_laser_blocks(
    blocks: Res<Registry<Block>>,
    mut cannon: ResMut<LaserCannonBlocks>,
    mut colors: ResMut<Registry<LaserCannonColorBlock>>,
) {
    if let Some(block) = blocks.from_id("cosmos:laser_cannon") {
        cannon.insert(block, LaserCannonProperty { energy_per_shot: 100.0 })
    }

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

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    laser_cannon_blocks: Res<LaserCannonBlocks>,
    color_blocks: Res<Registry<LaserCannonColorBlock>>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut LaserCannonSystem>,
    systems_query: Query<&Systems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                let old_block = blocks.from_numeric_id(ev.old_block);
                let new_block = blocks.from_numeric_id(ev.new_block);

                if laser_cannon_blocks.get(old_block).is_some() {
                    system.block_removed(&ev.block);
                }

                if let Some(property) = laser_cannon_blocks.get(new_block) {
                    system.block_added(property, &ev.block);
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
                    system.recalculate_colors(Some(ev.block.coords()));
                }
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    color_blocks: Res<Registry<LaserCannonColorBlock>>,
    mut commands: Commands,
    laser_cannon_blocks: Res<LaserCannonBlocks>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = LaserCannonSystem::default();

            let mut color_found = false;

            for structure_block in structure.all_blocks_iter(false) {
                let block = structure_block.block(structure, &blocks);
                if let Some(prop) = laser_cannon_blocks.get(block) {
                    system.block_added(prop, &structure_block);
                }
                if let Some(color_property) = color_blocks.from_block(block) {
                    color_found = true;
                    system.colors.push((structure_block.coords(), color_property.properties));
                }
            }

            if color_found {
                system.recalculate_colors(None);
            }

            systems.add_system(&mut commands, system);
        }
    }
}

pub(super) fn register<T: States + Clone + Copy>(app: &mut App, post_loading_state: T, playing_state: T) {
    create_registry::<LaserCannonColorBlock>(app);

    app.insert_resource(LaserCannonBlocks::default())
        .add_systems(OnEnter(post_loading_state), register_laser_blocks)
        .add_systems(
            Update,
            (
                structure_loaded_event.in_set(StructureLoadingSet::StructureLoaded),
                block_update_system,
            )
                .run_if(in_state(playing_state)),
        )
        .register_type::<LaserCannonSystem>();
}
