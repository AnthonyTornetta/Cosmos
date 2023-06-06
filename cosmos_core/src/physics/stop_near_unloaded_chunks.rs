//! Freezes entities that are near unloaded chunks so they don't fly into unloaded areas.

use bevy::prelude::{App, Commands, Component, Entity, GlobalTransform, Query, Without};
use bevy_rapier3d::prelude::RigidBody;

use crate::{
    physics::location::SECTOR_DIMENSIONS,
    structure::{
        asteroid::Asteroid, chunk::CHUNK_DIMENSIONS, planet::Planet,
        structure_iterator::ChunkIteratorResult, ChunkState, Structure,
    },
};

use super::location::Location;

/// At some point this may have to be based on the size of the entity. For now though, this is fine.
const FREEZE_RADIUS: i32 = 1;

/// TODO: THIS WILL NOT ALWAYS WORK
///
/// This has an issue where if some other system sets the rigidbody to static while this
/// component is attached (ie a player pilots a ship while that player is frozen) then
/// the body will be set back to dynamic once everything is loaded. This will cause in the
/// given example the player to have a dynamic body while piloting a ship (not good).
///
/// Come up with a better solution at some point please.
#[derive(Component)]
struct NeedsSetBack;

fn stop_near_unloaded_chunks(
    mut query: Query<
        (Entity, &Location, &mut RigidBody, Option<&NeedsSetBack>),
        (Without<Asteroid>, Without<Planet>),
    >,
    structures: Query<(Entity, &Structure, &Location, &GlobalTransform)>,
    mut commands: Commands,
) {
    for (ent, loc, mut rb, needs_set_back) in query.iter_mut() {
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

            let relative_coords = g_trans
                .to_scale_rotation_translation()
                .1
                .inverse()
                .mul_vec3(abs_coords);

            let (bx, by, bz) = structure.relative_coords_to_local_coords(
                relative_coords.x,
                relative_coords.y,
                relative_coords.z,
            );

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
                    ChunkIteratorResult::EmptyChunk {
                        position: (cx, cy, cz),
                    } => structure.get_chunk_state(cx, cy, cz) != ChunkState::Loaded,
                    _ => false,
                });

            if near_unloaded_chunk {
                is_fixed = true;
                *rb = RigidBody::Fixed;
                commands.entity(ent).insert(NeedsSetBack);
                break;
            }
        }

        if !is_fixed && needs_set_back.is_some() {
            *rb = RigidBody::Dynamic;
            commands.entity(ent).remove::<NeedsSetBack>();
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(stop_near_unloaded_chunks);
}
