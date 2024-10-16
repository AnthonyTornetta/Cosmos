//! Represents all the energy stored on a structure

use bevy::prelude::{in_state, App, Commands, EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut, Update};

use cosmos_core::{
    block::{block_events::BlockEventsSet, Block},
    events::block_events::BlockChangedEvent,
    registry::Registry,
    state::GameState,
    structure::{
        events::StructureLoadedEvent,
        systems::{
            energy_storage_system::{EnergyStorageBlocks, EnergyStorageProperty, EnergyStorageSystem},
            StructureSystemType, StructureSystems, StructureSystemsSet,
        },
        Structure,
    },
};

use super::sync::register_structure_system;

fn register_energy_blocks(blocks: Res<Registry<Block>>, mut storage: ResMut<EnergyStorageBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:energy_cell") {
        storage.insert(block, EnergyStorageProperty { capacity: 10000.0 });
    }

    if let Some(block) = blocks.from_id("cosmos:ship_core") {
        storage.insert(block, EnergyStorageProperty { capacity: 1000.0 })
    }
}

fn block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    energy_storage_blocks: Res<EnergyStorageBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut EnergyStorageSystem>,
    systems_query: Query<&StructureSystems>,
) {
    for ev in event.read() {
        if let Ok(systems) = systems_query.get(ev.structure_entity) {
            if let Ok(mut system) = systems.query_mut(&mut system_query) {
                if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.old_block)) {
                    system.block_removed(prop);
                }

                if let Some(prop) = energy_storage_blocks.get(blocks.from_numeric_id(ev.new_block)) {
                    system.block_added(prop);
                }
            }
        }
    }
}

fn structure_loaded_event(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    energy_storage_blocks: Res<EnergyStorageBlocks>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = EnergyStorageSystem::default();

            for block in structure.all_blocks_iter(false) {
                if let Some(prop) = energy_storage_blocks.get(block.block(structure, &blocks)) {
                    system.block_added(prop);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(EnergyStorageBlocks::default())
        .add_systems(OnEnter(GameState::PostLoading), register_energy_blocks)
        .add_systems(
            Update,
            (
                structure_loaded_event
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                block_update_system
                    .in_set(BlockEventsSet::ProcessEvents)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
            )
                .run_if(in_state(GameState::Playing)),
        )
        .register_type::<EnergyStorageSystem>();

    register_structure_system::<EnergyStorageSystem>(app, false, "cosmos:energy_cell");
}
