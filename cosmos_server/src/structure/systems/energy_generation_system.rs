//! Represents all the energy generation in a structure

use bevy::prelude::*;

use cosmos_core::{
    block::{block_events::BlockEventsSet, Block},
    events::block_events::BlockChangedEvent,
    netty::system_sets::NetworkingSystemsSet,
    registry::Registry,
    structure::{
        events::StructureLoadedEvent,
        systems::{
            energy_generation_system::{EnergyGenerationBlocks, EnergyGenerationProperty, EnergyGenerationSystem},
            energy_storage_system::EnergyStorageSystem,
            StructureSystem, StructureSystemType, StructureSystems, StructureSystemsSet,
        },
        Structure,
    },
};

use crate::state::GameState;

use super::sync::register_structure_system;

fn register_energy_blocks(blocks: Res<Registry<Block>>, mut generation: ResMut<EnergyGenerationBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:reactor") {
        generation.insert(block, EnergyGenerationProperty { generation_rate: 100.0 });
    }

    if let Some(block) = blocks.from_id("cosmos:ship_core") {
        generation.insert(block, EnergyGenerationProperty { generation_rate: 100.0 })
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    energy_generation_blocks: Res<EnergyGenerationBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut EnergyGenerationSystem>,
    systems_query: Query<&StructureSystems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                    system.block_removed(prop);
                }

                if let Some(prop) = energy_generation_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                    system.block_added(prop);
                }
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
    for (gen, system) in e_gen_query.iter() {
        if let Ok(systems) = sys_query.get(system.structure_entity()) {
            if let Ok(mut storage) = systems.query_mut(&mut e_storage_query) {
                storage.increase_energy(gen.energy_generation_rate() * time.delta_seconds());
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    energy_generation_blocks: Res<EnergyGenerationBlocks>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = EnergyGenerationSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = energy_generation_blocks.get(block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(EnergyGenerationBlocks::default())
        .add_systems(OnEnter(GameState::PostLoading), register_energy_blocks)
        .add_systems(
            Update,
            (
                structure_loaded_event
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                (
                    block_update_system.in_set(BlockEventsSet::ProcessEvents),
                    update_energy.in_set(StructureSystemsSet::UpdateSystems),
                )
                    .run_if(in_state(GameState::Playing))
                    .in_set(NetworkingSystemsSet::Between)
                    .chain(),
            ),
        )
        .register_type::<EnergyGenerationSystem>();

    register_structure_system::<EnergyGenerationSystem>(app, false, "cosmos:reactor");
}
