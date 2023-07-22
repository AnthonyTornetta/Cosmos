//! Freezes entities that are near unloaded chunks so they don't fly into unloaded areas.

use bevy::prelude::{App, Commands, Entity, GlobalTransform, Query, Update, With, Without};
use bevy_rapier3d::prelude::{Collider, RigidBodyDisabled};

use crate::{
    physics::location::SECTOR_DIMENSIONS,
    structure::{
        asteroid::Asteroid,
        coordinates::{UnboundChunkCoordinate, UnboundCoordinateType},
        planet::Planet,
        structure_iterator::ChunkIteratorResult,
        ChunkState, Structure,
    },
};

use super::location::Location;

/// At some point this may have to be based on the size of the entity. For now though, this is fine.
const FREEZE_RADIUS: UnboundCoordinateType = 1;

fn stop_near_unloaded_chunks(
    query: Query<(Entity, &Location), (Without<Asteroid>, Without<Planet>)>,
    structures: Query<(Entity, &Structure, &Location, &GlobalTransform)>,
    has_collider: Query<(), With<Collider>>,
    mut commands: Commands,
) {
    for (ent, loc) in query.iter() {
        let mut is_fixed = false;
        for (structure_ent, structure, structure_loc, g_trans) in structures.iter() {
            if structure_ent == ent {
                continue;
            }

            let unrotated_relative_loc = *loc - *structure_loc;
            let abs_coords = unrotated_relative_loc.absolute_coords_f32();

            if abs_coords.length_squared() > SECTOR_DIMENSIONS * SECTOR_DIMENSIONS {
                continue;
            }

            let relative_coords = g_trans.to_scale_rotation_translation().1.inverse().mul_vec3(abs_coords);

            let ub_coords = structure.relative_coords_to_local_coords(relative_coords.x, relative_coords.y, relative_coords.z);

            let ub_chunk_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ub_coords);

            // let (cx, cy, cz) = (
            //     bx as UnboundCoordinateType / CHUNK_DIMENSIONS as UnboundCoordinateType,
            //     by as UnboundCoordinateType / CHUNK_DIMENSIONS as UnboundCoordinateType,
            //     bz as UnboundCoordinateType / CHUNK_DIMENSIONS as UnboundCoordinateType,
            // );

            let near_unloaded_chunk = structure
                .chunk_iter(
                    UnboundChunkCoordinate::new(
                        ub_chunk_coords.x - FREEZE_RADIUS,
                        ub_chunk_coords.y - FREEZE_RADIUS,
                        ub_chunk_coords.z - FREEZE_RADIUS,
                    ),
                    UnboundChunkCoordinate::new(
                        ub_chunk_coords.x + FREEZE_RADIUS,
                        ub_chunk_coords.y + FREEZE_RADIUS,
                        ub_chunk_coords.z + FREEZE_RADIUS,
                    ),
                    true,
                )
                .any(|x| match x {
                    ChunkIteratorResult::EmptyChunk { position } => structure.get_chunk_state(position) != ChunkState::Loaded,
                    ChunkIteratorResult::FilledChunk { position, chunk: _ } => {
                        if let Some(chunk_entity) = structure.chunk_entity(position) {
                            !has_collider.contains(chunk_entity)
                        } else {
                            true
                        }
                    }
                });

            if near_unloaded_chunk {
                commands.entity(ent).insert(RigidBodyDisabled);
                is_fixed = true;
                break;
            }
        }

        if !is_fixed {
            commands.entity(ent).remove::<RigidBodyDisabled>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, stop_near_unloaded_chunks);
}
