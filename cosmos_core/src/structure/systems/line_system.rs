//! Shared functionality between systems that are created in a line

use std::{marker::PhantomData, mem::swap};

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

/// Calculates the total property from a line of properties
pub trait LinePropertyCalculator<T: LineProperty>: 'static + Send + Sync {
    /// Calculates the total property from a line of properties
    fn calculate_property(properties: &[T]) -> T;
}

/// Property each block adds to the line
pub trait LineProperty: 'static + Send + Sync + Clone + Copy {}

#[derive(Resource)]
/// The blocks that will effect this line
pub struct LineBlocks<T: LineProperty> {
    blocks: HashMap<u16, T>,
}

impl<T: LineProperty> Default for LineBlocks<T> {
    fn default() -> Self {
        Self {
            blocks: Default::default(),
        }
    }
}

impl<T: LineProperty> LineBlocks<T> {
    /// Registers a block with this property
    pub fn insert(&mut self, block: &Block, cannon_property: T) {
        self.blocks.insert(block.id(), cannon_property);
    }

    /// Gets the property for this specific block is there is one registered
    pub fn get(&self, block: &Block) -> Option<&T> {
        self.blocks.get(&block.id())
    }
}

#[derive(Default, Reflect, Clone, Copy)]
/// Every block that will change the color of laser cannons should have this property
pub struct LaserCannonColorProperty {
    /// The color this mining beam will be
    pub color: Color,
}

#[derive(Clone)]
/// The wrapper that ties a block to its alser cannon color properties
pub struct LineColorBlock {
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

impl Identifiable for LineColorBlock {
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

impl LineColorBlock {
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

impl Registry<LineColorBlock> {
    /// Gets the corrusponding properties if there is an entry for this block
    pub fn from_block(&self, block: &Block) -> Option<&LineColorBlock> {
        self.from_id(block.unlocalized_name())
    }

    /// Inserts a block with the specified properties
    pub fn insert(&mut self, block: &Block, properties: LaserCannonColorProperty) {
        self.register(LineColorBlock::new(block, properties));
    }
}

#[derive(Reflect)]
/// Represents a line of laser cannons.
///
/// All laser cannons in this line are facing the same direction.
pub struct Line<T: LineProperty> {
    /// The block at the start
    pub start: StructureBlock,
    /// The direction this line is facing
    pub direction: BlockFace,
    /// How many blocks this line has
    pub len: CoordinateType,
    /// The color of the laser
    pub color: Color,
    /// The combined property of all the blocks in this line
    pub property: T,

    /// All the properties of the laser cannons in this line
    properties: Vec<T>,
}

impl<T: LineProperty> Line<T> {
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

#[derive(Component)]
/// Represents all the laser cannons that are within this structure
pub struct LineSystem<T: LineProperty, S: LinePropertyCalculator<T>> {
    /// All the lins that there are
    pub lines: Vec<Line<T>>,
    /// Any color changers that are placed on this structure
    pub colors: Vec<(BlockCoordinate, LaserCannonColorProperty)>,
    _phantom: PhantomData<S>,
}

impl<T: LineProperty, S: LinePropertyCalculator<T>> Default for LineSystem<T, S> {
    fn default() -> Self {
        Self {
            lines: Default::default(),
            colors: Default::default(),
            _phantom: Default::default(),
        }
    }
}

fn is_in_line_with(block: &StructureBlock, direction: BlockFace, coord: &BlockCoordinate) -> bool {
    match direction {
        BlockFace::Front => coord.x == block.x && coord.y == block.y && coord.z <= block.z,
        BlockFace::Back => coord.x == block.x && coord.y == block.y && coord.z >= block.z,
        _ => todo!(),
    }
}

impl<T: LineProperty, S: LinePropertyCalculator<T>> LineSystem<T, S> {
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

    fn block_added(&mut self, prop: &T, block: &StructureBlock) {
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

                l1.properties.append(&mut l2.properties);

                self.lines.swap_remove(l2_i);
            }
            return;
        }

        // If gotten here, no suitable line was found

        let color = self.calculate_color_for_line(block, block_direction);

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

fn structure_loaded_event<T: LineProperty, S: LinePropertyCalculator<T>>(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    color_blocks: Res<Registry<LineColorBlock>>,
    mut commands: Commands,
    laser_cannon_blocks: Res<LineBlocks<T>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = LineSystem::<T, S>::default();

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

/// Adds all the functions a line system needs to operate
pub fn add_line_system<K: States, T: LineProperty, S: LinePropertyCalculator<T>>(app: &mut App, playing_state: K) {
    app.add_systems(
        Update,
        (
            structure_loaded_event::<T, S>.in_set(StructureLoadingSet::StructureLoaded),
            block_update_system::<T, S>,
        )
            .run_if(in_state(playing_state)),
    )
    .init_resource::<LineBlocks<T>>();
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    create_registry::<LineColorBlock>(app);

    app.add_systems(OnEnter(post_loading_state), add_colors);
}
