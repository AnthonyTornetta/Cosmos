//! Freezes entities that are near unloaded chunks so they don't fly into unloaded areas.

use bevy::{
    app::FixedUpdate,
    prelude::{App, Commands, Entity, GlobalTransform, IntoScheduleConfigs, Or, Query, With},
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        physics::player_world::PlayerWorld,
        structure::{
            block_storage::BlockStorer,
            chunk::Chunk,
            coordinates::{ChunkBlockCoordinate, ChunkCoordinate},
            dynamic_structure::DynamicStructure,
        },
    };
    use bevy::prelude::*;
    use bevy_rapier3d::{
        plugin::{DefaultRapierContext, RapierPhysicsPlugin, TimestepMode},
        prelude::{ExternalImpulse, RigidBody, Velocity},
    };

    fn loading_scene() -> (App, Entity, Entity) {
        let mut app = App::new();
        app.add_plugins((MinimalPlugins, TransformPlugin))
            .insert_resource(TimestepMode::Fixed {
                dt: 1.0 / 60.0,
                substeps: 2,
            })
            .add_plugins(RapierPhysicsPlugin::<()>::default().in_fixed_schedule());
        // Use the game's scheduling constraints, including its physics boundaries.
        crate::ecs::register(&mut app);
        register(&mut app);
        super::super::disable_rigid_body::register(&mut app);
        super::super::location::register(&mut app);
        app.finish();
        app.cleanup();
        app.update();
        let mut planet = DynamicStructure::new(1);
        planet.mark_chunk_being_loaded(ChunkCoordinate::new(0, 0, 0));
        let structure = app
            .world_mut()
            .spawn((Structure::Dynamic(planet), Location::ZERO, Transform::IDENTITY))
            .id();
        let player = app
            .world_mut()
            .spawn((
                Player::new("Loading test".into(), 1),
                Location::ZERO,
                Transform::IDENTITY,
                RigidBody::Dynamic,
                Collider::ball(0.5),
                Velocity::linear(Vec3::NEG_Y * 20.0),
                ExternalImpulse::default(),
            ))
            .id();
        let context = app
            .world_mut()
            .query_filtered::<Entity, With<DefaultRapierContext>>()
            .single(app.world())
            .unwrap();
        app.world_mut().entity_mut(context).insert((PlayerWorld { player }, Location::ZERO));
        (app, structure, player)
    }

    #[test]
    fn loading_freeze_holds_under_gravity_until_chunks_are_ready() {
        let (mut app, structure, player) = loading_scene();
        app.world_mut().run_schedule(FixedUpdate);
        let frozen_location = *app.world().get::<Location>(player).unwrap();
        for _ in 0..120 {
            app.world_mut().get_mut::<ExternalImpulse>(player).unwrap().impulse = Vec3::NEG_Y * 0.1;
            app.world_mut().run_schedule(FixedUpdate);
            assert!(app.world().get::<DisableRigidBody>(player).unwrap().contains_reason(REASON));
            assert_eq!(*app.world().get::<Location>(player).unwrap(), frozen_location);
        }
        let mut structure = app.world_mut().get_mut::<Structure>(structure).unwrap();
        let Structure::Dynamic(dynamic) = &mut *structure else {
            unreachable!()
        };
        dynamic.set_to_empty_chunk(ChunkCoordinate::new(0, 0, 0));
        for _ in 0..2 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert!(!app.world().get::<DisableRigidBody>(player).unwrap().contains_reason(REASON));
        assert!(app.world().get::<Location>(player).unwrap().local.y < frozen_location.local.y);
    }

    #[test]
    fn loading_freeze_waits_for_the_terrain_collider() {
        let (mut app, structure_entity, player) = loading_scene();
        let coordinates = ChunkCoordinate::new(0, 0, 0);
        let chunk_entity = app.world_mut().spawn(Transform::from_xyz(0.0, -10.0, 0.0)).id();
        {
            let mut structure = app.world_mut().get_mut::<Structure>(structure_entity).unwrap();
            let Structure::Dynamic(dynamic) = &mut *structure else {
                unreachable!()
            };
            let mut chunk = Chunk::new(coordinates);
            chunk.set_block_at_from_id(ChunkBlockCoordinate::new(0, 0, 0).unwrap(), 1, Default::default());
            dynamic.set_chunk(chunk);
            dynamic.set_chunk_entity(coordinates, chunk_entity);
            assert_eq!(dynamic.get_chunk_state(coordinates), ChunkState::Loaded);
        }
        app.world_mut().run_schedule(FixedUpdate);
        let frozen_location = *app.world().get::<Location>(player).unwrap();
        for _ in 0..3 {
            app.world_mut().run_schedule(FixedUpdate);
            assert!(app.world().get::<DisableRigidBody>(player).unwrap().contains_reason(REASON));
            assert_eq!(*app.world().get::<Location>(player).unwrap(), frozen_location);
        }
        app.world_mut().entity_mut(chunk_entity).insert(Collider::cuboid(1.0, 1.0, 1.0));
        for _ in 0..2 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert!(!app.world().get::<DisableRigidBody>(player).unwrap().contains_reason(REASON));
        assert!(app.world().get::<Location>(player).unwrap().local.y < frozen_location.local.y);
    }

    #[test]
    fn loading_freeze_preserves_other_disable_reasons() {
        let (mut app, structure_entity, player) = loading_scene();
        app.world_mut()
            .entity_mut(player)
            .insert(DisableRigidBody::new_with_reason("test:other"));
        app.world_mut().run_schedule(FixedUpdate);
        let frozen_location = *app.world().get::<Location>(player).unwrap();
        {
            let mut structure = app.world_mut().get_mut::<Structure>(structure_entity).unwrap();
            let Structure::Dynamic(dynamic) = &mut *structure else {
                unreachable!()
            };
            dynamic.set_to_empty_chunk(ChunkCoordinate::new(0, 0, 0));
        }
        app.world_mut().run_schedule(FixedUpdate);
        let disable = app.world().get::<DisableRigidBody>(player).unwrap();
        assert!(!disable.contains_reason(REASON));
        assert!(disable.contains_reason("test:other"));
        assert_eq!(*app.world().get::<Location>(player).unwrap(), frozen_location);

        app.world_mut()
            .get_mut::<DisableRigidBody>(player)
            .unwrap()
            .remove_reason("test:other");
        for _ in 0..2 {
            app.world_mut().run_schedule(FixedUpdate);
        }
        assert!(app.world().get::<Location>(player).unwrap().local.y < frozen_location.local.y);
    }
}
