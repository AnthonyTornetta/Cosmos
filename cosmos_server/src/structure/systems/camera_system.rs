//! Represents all the energy stored on a structure

use bevy::prelude::{in_state, App, Commands, EventReader, IntoSystemConfigs, OnEnter, Query, Res, ResMut, Update};

use cosmos_core::{
    block::Block,
    events::block_events::BlockChangedEvent,
    registry::Registry,
    structure::{
        events::StructureLoadedEvent,
        loading::StructureLoadingSet,
        systems::{
            camera_system::{CameraBlocks, CameraSystem},
            StructureSystemType, Systems,
        },
        Structure,
    },
};

use crate::state::GameState;

use super::sync::register_structure_system;

fn register_camera_blocks(blocks: Res<Registry<Block>>, mut storage: ResMut<CameraBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:camera") {
        storage.insert(block);
    }

    // if let Some(block) = blocks.from_id("cosmos:ship_core") {
    //     storage.insert(block);
    // }
}

fn camera_block_update_system(
    mut event: EventReader<BlockChangedEvent>,
    camera_blocks: Res<CameraBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut CameraSystem>,
    systems_query: Query<&Systems>,
) {
    for ev in event.read() {
        let Ok(systems) = systems_query.get(ev.structure_entity) else {
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
    mut structure_query: Query<(&Structure, &mut Systems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    camera_blocks: Res<CameraBlocks>,
    registry: Res<Registry<StructureSystemType>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            let mut system = CameraSystem::default();

            for block in structure.all_blocks_iter(false) {
                if camera_blocks.is_camera(block.block(structure, &blocks)) {
                    system.block_added(block.coords());
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
                camera_structure_loaded_event_processor.in_set(StructureLoadingSet::StructureLoaded),
                camera_block_update_system,
            )
                .run_if(in_state(GameState::Playing)),
        )
        .register_type::<CameraSystem>();

    register_structure_system::<CameraSystem>(app);
}
