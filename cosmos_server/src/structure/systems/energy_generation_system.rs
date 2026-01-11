//! Represents all the energy generation in a structure

use bevy::prelude::*;

use cosmos_core::{
    block::{Block, block_events::BlockMessagesSet},
    events::block_events::BlockChangedMessage,
    registry::Registry,
    state::GameState,
    structure::{
        Structure,
        events::StructureLoadedMessage,
        systems::{
            StructureSystem, StructureSystemType, StructureSystems, StructureSystemsSet,
            energy_generation_system::{EnergyGenerationBlocks, EnergyGenerationProperty, EnergyGenerationSystem},
            energy_storage_system::EnergyStorageSystem,
        },
    },
};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

use super::sync::register_structure_system;

fn register_energy_blocks(blocks: Res<Registry<Block>>, mut generation: ResMut<EnergyGenerationBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:passive_generator") {
        generation.insert(block, EnergyGenerationProperty { generation_rate: 100.0 });
    }

    if let Some(block) = blocks.from_id("cosmos:ship_core") {
        generation.insert(block, EnergyGenerationProperty { generation_rate: 100.0 })
    }
}

fn block_update_system(
    mut event: MessageReader<BlockChangedMessage>,
    energy_generation_blocks: Res<EnergyGenerationBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut EnergyGenerationSystem>,
    systems_query: Query<&StructureSystems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.block.structure())
            && let Ok(mut system) = systems.query_mut(&mut system_query)
        {
            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                system.block_removed(prop);
            }

            if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                system.block_added(prop);
            }
        }
    }
}

fn update_energy(
    sys_query: Query<&StructureSystems>,
    e_gen_query: Query<(&EnergyGenerationSystem, &StructureSystem)>,
    mut e_storage_query: Query<&mut EnergyStorageSystem>,
    time: Res<Time>,
) {
    for (g, system) in e_gen_query.iter() {
        if let Ok(systems) = sys_query.get(system.structure_entity())
            && let Ok(mut storage) = systems.query_mut(&mut e_storage_query)
        {
            // Prevent unneeded change detection
            if !storage.is_full() {
                storage.increase_energy(g.energy_generation_rate() * time.delta_secs());
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: MessageReader<StructureLoadedMessage>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    energy_generation_blocks: Res<EnergyGenerationBlocks>,
    registry: Res<Registry<StructureSystemType>>,
    q_energy_gen_system: Query<(), With<EnergyGenerationSystem>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            if systems.query(&q_energy_gen_system).is_ok() {
                continue;
            }

            let mut system = EnergyGenerationSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = energy_generation_blocks.get(structure.block_at(block, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

impl DefaultPersistentComponent for EnergyGenerationSystem {}

pub(super) fn register(app: &mut App) {
    make_persistent::<EnergyGenerationSystem>(app);

    app.insert_resource(EnergyGenerationBlocks::default())
        .add_systems(OnEnter(GameState::PostLoading), register_energy_blocks)
        .add_systems(
            FixedUpdate,
            (
                structure_loaded_event
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                (
                    block_update_system.in_set(BlockMessagesSet::ProcessMessages),
                    update_energy.in_set(StructureSystemsSet::UpdateSystemsBlocks),
                )
                    .run_if(in_state(GameState::Playing))
                    .chain(),
            ),
        )
        .register_type::<EnergyGenerationSystem>();

    register_structure_system::<EnergyGenerationSystem>(app, false, "cosmos:passive_generator");
}
