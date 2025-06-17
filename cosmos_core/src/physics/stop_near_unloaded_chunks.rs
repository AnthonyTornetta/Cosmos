//! Freezes entities that are near unloaded chunks so they don't fly into unloaded areas.

use bevy::{
    app::FixedUpdate,
    prelude::{App, Commands, Entity, GlobalTransform, IntoScheduleConfigs, Or, PostUpdate, Query, With},
};
use bevy_rapier3d::prelude::Collider;

use crate::{
    ecs::sets::FixedUpdateSet,
    entities::player::Player,
    physics::location::SECTOR_DIMENSIONS,
    prelude::Ship,
    structure::{
        ChunkState, Structure,
        coordinates::{UnboundChunkCoordinate, UnboundCoordinateType},
        structure_iterator::ChunkIteratorResult,
    },
};

use super::{
    disable_rigid_body::{DisableRigidBody, DisableRigidBodySet},
    location::Location,
};

/// At some point this may have to be based on the size of the entity. For now though, this is fine.
const FREEZE_RADIUS: UnboundCoordinateType = 1;

const REASON: &str = "cosmos:stop_near_unloaded_chunks";

fn stop_near_unloaded_chunks(
    mut query: Query<(Entity, &Location, Option<&mut DisableRigidBody>), Or<(With<Player>, With<Ship>)>>,
    structures: Query<(Entity, &Structure, &Location, &GlobalTransform)>,
    has_collider: Query<(), With<Collider>>,
    mut commands: Commands,
) {
    for (ent, loc, mut disable_rb) in query.iter_mut() {
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

            let near_unloaded_chunk = match structure {
                Structure::Full(f) => !f.is_loaded(),
                Structure::Dynamic(_) => {
                    let relative_coords = g_trans.to_scale_rotation_translation().1.inverse().mul_vec3(abs_coords);

                    let ub_coords = structure.relative_coords_to_local_coords(relative_coords.x, relative_coords.y, relative_coords.z);

                    let ub_chunk_coords = UnboundChunkCoordinate::for_unbound_block_coordinate(ub_coords);

                    structure
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
                        })
                }
            };

            if near_unloaded_chunk {
                if let Some(disable_rb) = disable_rb.as_mut() {
                    disable_rb.add_reason(REASON);
                } else {
                    let mut disable_rb = DisableRigidBody::default();
                    disable_rb.add_reason(REASON);
                    commands.entity(ent).insert(disable_rb);
                }
                is_fixed = true;
                break;
            }
        }

        if !is_fixed && let Some(disable_rb) = disable_rb.as_mut() {
            disable_rb.remove_reason(REASON);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        stop_near_unloaded_chunks
            .before(DisableRigidBodySet::DisableRigidBodies)
            .in_set(FixedUpdateSet::PrePhysics),
    );
}
