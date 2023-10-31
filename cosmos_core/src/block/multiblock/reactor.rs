//! Represents the logic behind the reactor multiblock system

use bevy::{
    prelude::{Added, App, Commands, Component, Entity, OnEnter, Query, Res, ResMut, States, Update, Without},
    reflect::Reflect,
};

use crate::{
    block::Block,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{coordinates::BlockCoordinate, structure_block::StructureBlock, Structure},
};

#[derive(Debug, Clone, Copy, Reflect)]
/// The inclusive bounds of a reactor, including its casing
pub struct ReactorBounds {
    /// Inclusive negative-most coordinates of a reactor (include casing)
    pub negative_coords: BlockCoordinate,
    /// Inclusive positive-most coordinates of a reactor (include casing)
    pub positive_coords: BlockCoordinate,
}

#[derive(Component, Clone, Copy, Debug, Reflect)]
/// Represents a constructed reactor
pub struct Reactor {
    controller: StructureBlock,
    power_per_second: f32,
    bounds: ReactorBounds,
}

impl Reactor {
    /// Creates a new constructed reactor
    pub fn new(controller: StructureBlock, power_per_second: f32, bounds: ReactorBounds) -> Self {
        Self {
            bounds,
            controller,
            power_per_second,
        }
    }
}

#[derive(Component, Default, Reflect)]
/// Stores the entities of all the reactors in a structure and their controller blocks for quick access
pub struct Reactors(Vec<(StructureBlock, Entity)>);

impl Reactors {
    /// Adds a reactor to the structure
    pub fn add_reactor(&mut self, reactor_entity: Entity, controller_block: StructureBlock) {
        self.0.push((controller_block, reactor_entity));
    }

    /// Iterates over all the reactors in the structure
    pub fn iter(&self) -> std::slice::Iter<(StructureBlock, Entity)> {
        self.0.iter()
    }
}

#[derive(Debug, Clone)]
/// A block that can be used in a reactor to generate power
pub struct ReactorPowerGenerationBlock {
    power_per_second: f32,

    id: u16,
    unlocalized_name: String,
}

impl Identifiable for ReactorPowerGenerationBlock {
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

impl ReactorPowerGenerationBlock {
    /// Creates a link to a block to make it usable within a reactor to create power
    pub fn new(block: &Block, power_per_second: f32) -> Self {
        Self {
            power_per_second,
            id: 0,
            unlocalized_name: block.unlocalized_name().to_owned(),
        }
    }

    /// The power per second this block will produce
    pub fn power_per_second(&self) -> f32 {
        self.power_per_second
    }
}

fn register_power_blocks(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<ReactorPowerGenerationBlock>>) {
    if let Some(reactor_block) = blocks.from_id("cosmos:reactor_cell") {
        registry.register(ReactorPowerGenerationBlock::new(reactor_block, 1000.0));
    }
}

impl Registry<ReactorPowerGenerationBlock> {
    /// Gets the reactor power generation entry for this block
    pub fn for_block(&self, block: &Block) -> Option<&ReactorPowerGenerationBlock> {
        self.from_id(block.unlocalized_name())
    }
}

fn on_structure_add(mut commands: Commands, query: Query<Entity, (Added<Structure>, Without<Reactors>)>) {
    for ent in query.iter() {
        commands.entity(ent).insert(Reactors::default());
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    create_registry::<ReactorPowerGenerationBlock>(app);

    app.add_systems(OnEnter(post_loading_state), register_power_blocks)
        .add_systems(Update, on_structure_add)
        .register_type::<Reactor>()
        .register_type::<Reactors>();
}
