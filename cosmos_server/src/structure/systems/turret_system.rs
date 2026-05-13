//! Represents all the energy stored on a structure

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
            StructureSystemType, StructureSystems, StructureSystemsSet,
            turret_system::{TurretBlocks, TurretSystem},
        },
    },
};

use crate::persistence::make_persistent::{DefaultPersistentComponent, make_persistent};

use super::sync::register_structure_system;

fn register_turret_blocks(blocks: Res<Registry<Block>>, mut turret_blocks: ResMut<TurretBlocks>) {
    if let Some(block) = blocks.from_id("cosmos:turret_base") {
        turret_blocks.insert(block);
    }
}

fn turret_block_update_system(
    mut event: MessageReader<BlockChangedMessage>,
    turret_blocks: Res<TurretBlocks>,
    blocks: Res<Registry<Block>>,
    mut system_query: Query<&mut TurretSystem>,
    q_systems: Query<&StructureSystems>,
) {
    for ev in event.read() {
        let Ok(systems) = q_systems.get(ev.block.structure()) else {
            continue;
        };

        let Ok(mut system) = systems.query_mut(&mut system_query) else {
            continue;
        };

        if turret_blocks.is_turret(blocks.from_numeric_id(ev.old_block)) {
            system.block_removed(ev.block.coords());
        }

        if turret_blocks.is_turret(blocks.from_numeric_id(ev.new_block)) {
            system.block_added(ev.block.coords());
        }
    }
}

fn turret_structure_loaded_event_processor(
    mut event_reader: MessageReader<StructureLoadedMessage>,
    mut structure_query: Query<(&Structure, &mut StructureSystems)>,
    blocks: Res<Registry<Block>>,
    mut commands: Commands,
    turret_blocks: Res<TurretBlocks>,
    registry: Res<Registry<StructureSystemType>>,
    q_thruster_system: Query<(), With<TurretSystem>>,
) {
    for ev in event_reader.read() {
        if let Ok((structure, mut systems)) = structure_query.get_mut(ev.structure_entity) {
            if systems.query(&q_thruster_system).is_ok() {
                continue;
            }

            let mut system = TurretSystem::default();

            for block in structure.all_blocks_iter(false) {
                if turret_blocks.is_turret(structure.block_at(block, &blocks)) {
                    system.block_added(block);
                }
            }

            systems.add_system(&mut commands, system, &registry);
        }
    }
}

impl DefaultPersistentComponent for TurretSystem {}

pub(super) fn register(app: &mut App) {
    make_persistent::<TurretSystem>(app);

    app.insert_resource(TurretBlocks::default())
        .add_systems(OnEnter(GameState::PostLoading), register_turret_blocks)
        .add_systems(
            FixedUpdate,
            (
                turret_structure_loaded_event_processor
                    .in_set(StructureSystemsSet::InitSystems)
                    .ambiguous_with(StructureSystemsSet::InitSystems),
                turret_block_update_system
                    .in_set(BlockMessagesSet::ProcessMessages)
                    .in_set(StructureSystemsSet::UpdateSystemsBlocks),
            )
                .run_if(in_state(GameState::Playing)),
        )
        .register_type::<TurretSystem>();

    register_structure_system::<TurretSystem>(app, false, "cosmos:turret_base");
}
