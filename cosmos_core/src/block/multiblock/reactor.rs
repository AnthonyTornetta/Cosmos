//! Represents the logic behind the reactor multiblock system

use bevy::{
    prelude::{
        in_state, warn, Added, App, Commands, Component, Deref, DerefMut, Entity, EventReader, IntoSystemConfigs, OnEnter, Parent, Query,
        Res, ResMut, States, Update, Without,
    },
    reflect::Reflect,
    time::Time,
};

use crate::{
    block::Block,
    ecs::NeedsDespawned,
    events::block_events::BlockChangedEvent,
    registry::{create_registry, identifiable::Identifiable, Registry},
    structure::{
        coordinates::BlockCoordinate,
        structure_block::StructureBlock,
        systems::{energy_storage_system::EnergyStorageSystem, Systems},
        Structure,
    },
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

    /// Decreases the power-per-second generated in this reactor.
    pub fn decrease_power_per_second(&mut self, amount: f32) {
        self.power_per_second -= amount;
        self.power_per_second = self.power_per_second.max(0.0);
    }

    /// Increases the power-per-second generated in this reactor
    pub fn increase_power_per_second(&mut self, amount: f32) {
        self.power_per_second += amount;
    }
}

#[derive(Component, Default, Reflect, DerefMut, Deref)]
/// Stores the entities of all the reactors in a structure and their controller blocks for quick access
pub struct Reactors(Vec<(StructureBlock, Entity)>);

impl Reactors {
    /// Adds a reactor to the structure
    pub fn add_reactor(&mut self, reactor_entity: Entity, controller_block: StructureBlock) {
        self.0.push((controller_block, reactor_entity));
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

fn on_modify_reactor(
    mut commands: Commands,
    mut reactor_query: Query<&mut Reactor>,
    mut reactors_query: Query<&mut Reactors>,
    mut block_change_event: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    reactor_cells: Res<Registry<ReactorPowerGenerationBlock>>,
) {
    for ev in block_change_event.iter() {
        let Ok(mut reactors) = reactors_query.get_mut(ev.structure_entity) else {
            continue;
        };

        reactors.retain(|&(_, reactor_entity)| {
            let Ok(mut reactor) = reactor_query.get_mut(reactor_entity) else {
                warn!("Missing reactor component but is in the reactors component list.");
                return false;
            };

            let (neg, pos) = (reactor.bounds.negative_coords, reactor.bounds.positive_coords);

            let within_x = neg.x <= ev.block.x && pos.x >= ev.block.x;
            let within_y = neg.y <= ev.block.y && pos.y >= ev.block.y;
            let within_z = neg.z <= ev.block.z && pos.z >= ev.block.z;

            if (neg.x == ev.block.x || pos.x == ev.block.x) && (within_y && within_z)
                || (neg.y == ev.block.y || pos.y == ev.block.y) && (within_x && within_z)
                || (neg.z == ev.block.z || pos.z == ev.block.z) && (within_x && within_y)
            {
                // They changed the casing of the reactor - kill it
                commands.entity(reactor_entity).insert(NeedsDespawned);

                false
            } else {
                if within_x && within_y && within_z {
                    // The innards of the reactor were changed, add/remove any needed power per second
                    if let Some(reactor_cell) = reactor_cells.for_block(blocks.from_numeric_id(ev.old_block)) {
                        reactor.decrease_power_per_second(reactor_cell.power_per_second());
                    }

                    if let Some(reactor_cell) = reactor_cells.for_block(blocks.from_numeric_id(ev.new_block)) {
                        reactor.increase_power_per_second(reactor_cell.power_per_second());
                    }
                }

                true
            }
        });
    }
}

fn generate_power(
    reactors: Query<(&Reactor, &Parent)>,
    structure: Query<&Systems>,
    mut energy_storage_system_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (reactor, parent) in reactors.iter() {
        let Ok(systems) = structure.get(parent.get()) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut energy_storage_system_query) else {
            continue;
        };

        system.increase_energy(reactor.power_per_second * time.delta_seconds());
    }
}

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T, playing_state: T) {
    create_registry::<ReactorPowerGenerationBlock>(app);

    app.add_systems(OnEnter(post_loading_state), register_power_blocks)
        .add_systems(
            Update,
            (on_structure_add, generate_power, on_modify_reactor).run_if(in_state(playing_state)),
        )
        .register_type::<Reactor>()
        .register_type::<Reactors>();
}
