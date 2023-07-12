//! Freezes entities that are near unloaded chunks so they don't fly into unloaded areas.

use bevy::prelude::{App, Commands, Entity, GlobalTransform, Query, Without};
use bevy_rapier3d::prelude::RigidBodyDisabled;

use crate::{
    physics::location::SECTOR_DIMENSIONS,
    structure::{
        asteroid::Asteroid, chunk::CHUNK_DIMENSIONS, planet::Planet, structure_iterator::ChunkIteratorResult, ChunkState, Structure,
    },
};

use super::location::Location;

/// At some point this may have to be based on the size of the entity. For now though, this is fine.
const FREEZE_RADIUS: i32 = 1;

fn stop_near_unloaded_chunks(
    query: Query<(Entity, &Location), (Without<Asteroid>, Without<Planet>)>,
    structures: Query<(Entity, &Structure, &Location, &GlobalTransform)>,
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

            let (bx, by, bz) = structure.relative_coords_to_local_coords(relative_coords.x, relative_coords.y, relative_coords.z);

            let (cx, cy, cz) = (
                bx / CHUNK_DIMENSIONS as i32,
                by / CHUNK_DIMENSIONS as i32,
                bz / CHUNK_DIMENSIONS as i32,
            );

            let near_unloaded_chunk = structure
                .chunk_iter(
                    (cx - FREEZE_RADIUS, cy - FREEZE_RADIUS, cz - FREEZE_RADIUS),
                    (cx + FREEZE_RADIUS, cy + FREEZE_RADIUS, cz + FREEZE_RADIUS),
                    true,
                )
                .any(|x| match x {
                    ChunkIteratorResult::EmptyChunk { position: (cx, cy, cz) } => {
                        structure.get_chunk_state(cx, cy, cz) != ChunkState::Loaded
                    }
                    _ => false,
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
    app.add_system(stop_near_unloaded_chunks);
}
