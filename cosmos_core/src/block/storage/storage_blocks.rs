use bevy::{
    app::{App, Update},
    core::Name,
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        query::{Added, Without},
        system::{Commands, Query, Res},
    },
    hierarchy::BuildChildren,
    log::warn,
    utils::HashMap,
};

use crate::{
    block::Block,
    ecs::NeedsDespawned,
    events::block_events::BlockChangedEvent,
    inventory::Inventory,
    netty::NoSendEntity,
    registry::Registry,
    structure::{
        chunk::ChunkEntity,
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
        Structure,
    },
};

#[derive(Component, Default)]
struct ChunkStorageBlockData {
    data: HashMap<ChunkBlockCoordinate, Entity>,
}

fn on_add_storage(
    mut q_chunk_storage_block_data: Query<&mut ChunkStorageBlockData>,
    q_structure: Query<&Structure>,
    blocks: Res<Registry<Block>>,
    mut evr_block_changed: EventReader<BlockChangedEvent>,
    mut commands: Commands,
) {
    if evr_block_changed.is_empty() {
        return;
    }

    let Some(block) = blocks.from_id("cosmos:storage") else {
        return;
    };

    for ev in evr_block_changed.read() {
        if ev.new_block == ev.old_block {
            continue;
        }

        let Ok(structure) = q_structure.get(ev.structure_entity) else {
            continue;
        };

        if blocks.from_numeric_id(ev.old_block) == block {
            let coords = ev.block.coords();

            let Some(chunk_ent) = structure.chunk_entity(ChunkCoordinate::for_block_coordinate(coords)) else {
                warn!("Missing chunk entity but got block change event? How???");
                continue;
            };

            let Ok(mut storage) = q_chunk_storage_block_data.get_mut(chunk_ent) else {
                warn!("Chunk missing storage data mapping!");
                continue;
            };

            if let Some(ent) = storage.data.remove(&ChunkBlockCoordinate::for_block_coordinate(coords)) {
                commands.entity(ent).insert(NeedsDespawned);
            }
        }

        if blocks.from_numeric_id(ev.new_block) == block {
            let coords = ev.block.coords();

            let Some(chunk_ent) = structure.chunk_entity(ChunkCoordinate::for_block_coordinate(coords)) else {
                warn!("Missing chunk entity but got block change event? How???");
                continue;
            };

            let Ok(mut storage) = q_chunk_storage_block_data.get_mut(chunk_ent) else {
                warn!("Chunk missing storage data mapping!");
                continue;
            };

            let chunk_block_coord = ChunkBlockCoordinate::for_block_coordinate(coords);

            let inventory_ent = commands
                .spawn((
                    Name::new(format!("Inventory for Block @ {chunk_block_coord}")),
                    NoSendEntity,
                    Inventory::new(9 * 5, None),
                ))
                .id();

            commands.entity(chunk_ent).add_child(inventory_ent);
            storage.data.insert(chunk_block_coord, inventory_ent);
        }
    }
}

fn on_add_chunk_ent(query: Query<Entity, (Without<ChunkStorageBlockData>, Added<ChunkEntity>)>, mut commands: Commands) {
    for ent in query.iter() {
        commands.entity(ent).insert(ChunkStorageBlockData::default());
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, (on_add_chunk_ent, on_add_storage));
}
