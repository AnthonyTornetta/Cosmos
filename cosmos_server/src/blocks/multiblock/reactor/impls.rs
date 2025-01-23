use bevy::prelude::*;
use cosmos_core::{
    block::{
        block_events::{BlockEventsSet, BlockInteractEvent},
        multiblock::reactor::{OpenReactorEvent, ReactorPowerGenerationBlock, Reactors},
        Block,
    },
    entities::player::Player,
    events::block_events::BlockChangedEvent,
    inventory::Inventory,
    netty::{sync::events::server_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    prelude::{Structure, StructureLoadingSet, StructureSystems},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::systems::{energy_storage_system::EnergyStorageSystem, StructureSystemsSet},
};

use crate::{
    blocks::data::utils::add_default_block_data_for_block,
    persistence::make_persistent::{make_persistent, DefaultPersistentComponent},
};

impl DefaultPersistentComponent for Reactors {}

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
            nevw.send(OpenReactorEvent(s_block), player.id());
        }
    }
}

fn generate_power(
    reactors: Query<(&Reactors, Entity)>,
    structure: Query<&StructureSystems>,
    mut energy_storage_system_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (reactors, structure_entity) in reactors.iter() {
        let Ok(systems) = structure.get(structure_entity) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut energy_storage_system_query) else {
            continue;
        };

        for reactor in reactors.iter() {
            system.increase_energy(reactor.power_per_second() * time.delta_secs());
        }
    }
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
) {
    for ev in block_change_event.read() {
        let Ok(mut reactors) = reactors_query.get_mut(ev.block.structure()) else {
            continue;
        };

        reactors.retain_mut(|reactor| {
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

pub(super) fn register(app: &mut App) {
    add_default_block_data_for_block(app, |e, _| Inventory::new("Reactor", 1, None, e), "cosmos:reactor_controller");
    make_persistent::<Reactors>(app);

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
