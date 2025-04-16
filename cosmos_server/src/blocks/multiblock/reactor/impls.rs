use std::{cell::RefCell, ops::DerefMut, rc::Rc, time::Duration};

use bevy::prelude::*;
use cosmos_core::{
    block::{
        Block,
        block_events::{BlockEventsSet, BlockInteractEvent},
        data::BlockData,
        multiblock::reactor::{
            ClientRequestChangeReactorStatus, OpenReactorEvent, Reactor, ReactorActive, ReactorFuel, ReactorFuelConsumption,
            ReactorPowerGenerationBlock, Reactors,
        },
    },
    entities::player::Player,
    events::block_events::{BlockChangedEvent, BlockDataSystemParams},
    inventory::Inventory,
    item::Item,
    netty::{
        sync::events::server_event::{NettyEventReceived, NettyEventWriter},
        system_sets::NetworkingSystemsSet,
    },
    prelude::{Structure, StructureLoadingSet, StructureSystems},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::systems::{StructureSystemsSet, energy_storage_system::EnergyStorageSystem},
};

use crate::{
    blocks::data::utils::add_default_block_data_for_block,
    persistence::make_persistent::{DefaultPersistentComponent, make_persistent},
};

impl DefaultPersistentComponent for Reactors {}
impl DefaultPersistentComponent for Reactor {}
impl DefaultPersistentComponent for ReactorFuelConsumption {}
impl DefaultPersistentComponent for ReactorActive {}

fn handle_block_event(
    mut interact_events: EventReader<BlockInteractEvent>,
    s_query: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    q_player: Query<&Player>,
    mut nevw: NettyEventWriter<OpenReactorEvent>,
) {
    for ev in interact_events.read() {
        let Some(s_block) = ev.block else {
            continue;
        };

        let Ok(player) = q_player.get(ev.interactor) else {
            continue;
        };

        let Ok(structure) = s_query.get(s_block.structure()) else {
            continue;
        };

        let Some(block) = blocks.from_id("cosmos:reactor_controller") else {
            continue;
        };

        let block_id = s_block.block_id(structure);

        if block_id == block.id() {
            nevw.send(OpenReactorEvent(s_block), player.client_id());
        }
    }
}

fn generate_power(
    reactors: Query<(&Reactors, Entity)>,
    mut q_structure: Query<(&mut Structure, &StructureSystems)>,
    mut energy_storage_system_query: Query<&mut EnergyStorageSystem>,
    mut q_reactor: Query<(&Reactor, Option<&mut ReactorFuelConsumption>, &mut Inventory), With<ReactorActive>>,
    time: Res<Time>,
    bds_params: BlockDataSystemParams,
    fuels: Res<Registry<ReactorFuel>>,
    items: Res<Registry<Item>>,
    q_has_fuel_cons: Query<(), With<ReactorFuelConsumption>>,
    mut q_block_data: Query<&mut BlockData>,
    mut commands: Commands,
) {
    let bds_params = Rc::new(RefCell::new(bds_params));

    for (reactors, structure_entity) in reactors.iter() {
        let Ok((mut structure, systems)) = q_structure.get_mut(structure_entity) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut energy_storage_system_query) else {
            continue;
        };

        for &c in reactors.iter() {
            let Some(mut reactor) = structure.query_block_data_mut(c, &mut q_reactor, bds_params.clone()) else {
                continue;
            };

            let (reactor, fuel_consumption, inventory) = reactor.deref_mut();

            let mut delta = time.delta_secs() * reactor.fuel_consumption_multiplier;
            if let Some(fuel_consumption) = fuel_consumption {
                fuel_consumption.secs_spent += delta;
                let fuel = fuels.from_numeric_id(fuel_consumption.fuel_id);

                let over = fuel_consumption.secs_spent - fuel.lasts_for.as_secs_f32();

                if over >= 0.0 {
                    if let Some(fuel) = get_fuel_if_available(&fuels, &items, inventory, &mut commands) {
                        fuel_consumption.secs_spent = over;
                        fuel_consumption.fuel_id = fuel.id();
                    } else {
                        delta -= over;
                        structure.remove_block_data::<ReactorFuelConsumption>(
                            c,
                            &mut bds_params.borrow_mut(),
                            &mut q_block_data,
                            &q_has_fuel_cons,
                        );
                    }
                }
            } else {
                let Some(fuel) = get_fuel_if_available(&fuels, &items, inventory, &mut commands) else {
                    continue;
                };

                structure.insert_block_data(
                    c,
                    ReactorFuelConsumption {
                        fuel_id: fuel.id(),
                        secs_spent: delta,
                    },
                    &mut bds_params.borrow_mut(),
                    &mut q_block_data,
                    &q_has_fuel_cons,
                );
            }

            system.increase_energy(reactor.power_per_second() * delta);
        }
    }
}

fn get_fuel_if_available<'a>(
    fuels: &'a Registry<ReactorFuel>,
    items: &Registry<Item>,
    inventory: &mut Mut<'_, Inventory>,
    commands: &mut Commands,
) -> Option<&'a ReactorFuel> {
    let is = inventory.itemstack_at(0)?;
    let item = items.from_numeric_id(is.item_id());
    let fuel = fuels.from_id(item.unlocalized_name())?;
    inventory.take_and_remove_item(item, 1, commands);

    Some(fuel)
}

fn add_reactor_to_structure(mut commands: Commands, query: Query<Entity, (Added<Structure>, Without<Reactors>)>) {
    for ent in query.iter() {
        commands.entity(ent).insert(Reactors::default());
    }
}

fn on_modify_reactor(
    mut reactors_query: Query<&mut Reactors>,
    mut block_change_event: EventReader<BlockChangedEvent>,
    blocks: Res<Registry<Block>>,
    reactor_cells: Res<Registry<ReactorPowerGenerationBlock>>,
    mut q_reactor: Query<&mut Reactor>,
    mut q_structure: Query<&mut Structure>,
    bds_params: BlockDataSystemParams,
    q_has_reactor: Query<(), With<Reactor>>,
    mut q_block_data: Query<&mut BlockData>,
) {
    let bds_params = Rc::new(RefCell::new(bds_params));
    for ev in block_change_event.read() {
        let Ok(mut reactors) = reactors_query.get_mut(ev.block.structure()) else {
            continue;
        };

        let Ok(mut structure) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };

        // Stores stuff so borrow checker is happy
        let mut to_remove = vec![];

        reactors.retain(|&reactor_controller| {
            let Some(mut reactor) = structure.query_block_data_mut(reactor_controller, &mut q_reactor, bds_params.clone()) else {
                // This can happen if the controller is destroyed.
                return false;
            };

            let (neg, pos) = (reactor.bounds.negative_coords, reactor.bounds.positive_coords);

            let block = ev.block.coords();

            let within_x = neg.x <= block.x && pos.x >= block.x;
            let within_y = neg.y <= block.y && pos.y >= block.y;
            let within_z = neg.z <= block.z && pos.z >= block.z;

            if (neg.x == block.x || pos.x == block.x) && (within_y && within_z)
                || (neg.y == block.y || pos.y == block.y) && (within_x && within_z)
                || (neg.z == block.z || pos.z == block.z) && (within_x && within_y)
            {
                // They changed the casing of the reactor - kill it
                to_remove.push(reactor_controller);
                false
            } else {
                if within_x && within_y && within_z {
                    // The innards of the reactor were changed, add/remove any needed power per second
                    if let Some(reactor_cell) = reactor_cells.for_block(blocks.from_numeric_id(ev.old_block)) {
                        reactor.decrease_power_per_second(reactor_cell.power_per_second());
                        reactor.fuel_consumption_multiplier -= 1.0;
                    }

                    if let Some(reactor_cell) = reactor_cells.for_block(blocks.from_numeric_id(ev.new_block)) {
                        reactor.increase_power_per_second(reactor_cell.power_per_second());
                        reactor.fuel_consumption_multiplier += 1.0;
                    }
                }

                true
            }
        });

        for controller_block in to_remove {
            structure.remove_block_data::<Reactor>(controller_block, &mut bds_params.borrow_mut(), &mut q_block_data, &q_has_reactor);
        }
    }
}

fn process_activate_reactor(
    mut nevr: EventReader<NettyEventReceived<ClientRequestChangeReactorStatus>>,
    mut q_structure: Query<&mut Structure>,
    mut q_block_data: Query<&mut BlockData>,
    mut bds_params: BlockDataSystemParams,
    q_is_active: Query<(), With<ReactorActive>>,
    q_reactor: Query<(), With<Reactor>>,
) {
    for ev in nevr.read() {
        // TODO: Verify has access to reactor
        let Ok(mut structure) = q_structure.get_mut(ev.block.structure()) else {
            continue;
        };

        if structure.query_block_data(ev.block.coords(), &q_reactor).is_none() {
            continue;
        }

        if ev.active {
            structure.insert_block_data(ev.block.coords(), ReactorActive, &mut bds_params, &mut q_block_data, &q_is_active);
        } else {
            structure.remove_block_data::<ReactorActive>(ev.block.coords(), &mut bds_params, &mut q_block_data, &q_is_active);
        }
    }
}

fn register_reactor_fuel(mut reg: ResMut<Registry<ReactorFuel>>, items: Res<Registry<Item>>) {
    if let Some(uranium_fuel_cell) = items.from_id("cosmos:uranium_fuel_cell") {
        reg.register(ReactorFuel::new(uranium_fuel_cell, 1.0, Duration::from_mins(20)));
    }
}

fn register_power_blocks(blocks: Res<Registry<Block>>, mut registry: ResMut<Registry<ReactorPowerGenerationBlock>>) {
    if let Some(reactor_block) = blocks.from_id("cosmos:reactor_cell") {
        registry.register(ReactorPowerGenerationBlock::new(reactor_block, 5000.0));
    }
}

pub(super) fn register(app: &mut App) {
    add_default_block_data_for_block(app, |e, _| Inventory::new("Reactor", 1, None, e), "cosmos:reactor_controller");
    make_persistent::<Reactors>(app);
    make_persistent::<Reactor>(app);
    make_persistent::<ReactorFuelConsumption>(app);
    make_persistent::<ReactorActive>(app);

    app.add_systems(OnEnter(GameState::PostLoading), (register_power_blocks, register_reactor_fuel));

    app.add_systems(
        Update,
        handle_block_event
            .in_set(NetworkingSystemsSet::Between)
            .in_set(BlockEventsSet::ProcessEvents)
            .run_if(in_state(GameState::Playing)),
    );

    app.add_systems(
        Update,
        (
            add_reactor_to_structure.in_set(StructureLoadingSet::AddStructureComponents),
            process_activate_reactor.in_set(NetworkingSystemsSet::Between),
            (on_modify_reactor.in_set(BlockEventsSet::ProcessEvents), generate_power)
                .in_set(StructureSystemsSet::UpdateSystemsBlocks)
                .in_set(NetworkingSystemsSet::Between)
                .chain(),
        )
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    );
}
