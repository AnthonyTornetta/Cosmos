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
            camera_system::{CameraBlocks, CameraSystem},
            StructureSystemType, StructureSystems, StructureSystemsSet,
        },
        Structure,
    },
};

use super::sync::register_structure_system;

fn register_camera_blocks(blocks: Res<Registry<Block>>, mut camera_blocks: ResMut<CameraBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:camera") {
        camera_blocks.insert(block);
    }
}

fn camera_block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    camera_blocks: Res<CameraBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut CameraSystem>,
    q_systems: Query<&StructureSystems>,
) {
    for ev in event.read() {
        let Ok(systems) = q_systems.get(ev.block.structure()) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut system_query) else {
            continue;
        };

        if camera_blocks.is_camera(blocks.from_numeric_id(ev.old_block)) {
            system.block_removed(ev.block.coords());
        }

        if camera_blocks.is_camera(blocks.from_numeric_id(ev.new_block)) {
            system.block_added(ev.block.coords());
        }
    }
}

fn camera_structure_loaded_event_processor(
    mut event_reader: EventReader<StructureLoadedEvent>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    camera_blocks: Res<CameraBlocks>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = CameraSystem::default();

            for block in structure.all_blocks_iter(false) {
                if camera_blocks.is_camera(structure.block_at(block, &blocks)) {
                    system.block_added(block);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(CameraBlocks::default())
        .add_systems(OnEnter(GameState::PostLoading), register_camera_blocks)
        .add_systems(
            Update,
            (
                camera_structure_loaded_event_processor
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                camera_block_update_system
                    .in_set(BlockEventsSet::ProcessEvents)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
            )
                .run_if(in_state(GameState::Playing)),
        )
        .register_type::<CameraSystem>();

    register_structure_system::<CameraSystem>(app, false, "cosmos:camera");
}
