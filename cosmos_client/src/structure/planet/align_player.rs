//! Aligns a player to the planet

use bevy::prelude::*;
use cosmos_core::{
    block::block_face::BlockFace,
    ecs::sets::FixedUpdateSet,
    netty::client::LocalPlayer,
    physics::{
        gravity_system::GravityEmitter,
        location::{Location, LocationPhysicsSet},
    },
    structure::{planet::Planet, ship::pilot::Pilot},
};

pub(crate) fn align_player(
    mut player: Query<(Entity, &Location, &mut Transform), (With<LocalPlayer>, Without<ChildOf>, Without<Planet>)>,
    planets: Query<(Entity, &Location, &GravityEmitter, &Transform), With<Planet>>,
    mut commands: Commands,
) {
    let Ok((entity, location, mut transform)) = player.single_mut() else {
        return;
    };
    let mut best_planet = None;
    let mut best_dist = f32::INFINITY;

    for (ent, loc, ge, planet_transform) in planets.iter() {
        let dist = loc.distance_sqrd(location);
        if dist < best_dist {
            best_dist = dist;
            best_planet = Some((ent, loc, ge, planet_transform));
        }
    }

    if let Some((planet_ent, loc, ge, planet_transform)) = best_planet {
        let relative_position = loc.relative_coords_to(location);
        let planet_rotation = planet_transform.rotation;
        let relative_position = planet_rotation.inverse() * relative_position;

        let dist = relative_position.abs().max_element();

        if dist <= ge.radius {
            let face = Planet::planet_face_relative(relative_position);
            let axis = match face {
                BlockFace::Back | BlockFace::Front => AlignmentAxis::Z,
                BlockFace::Left | BlockFace::Right => AlignmentAxis::X,
                BlockFace::Top | BlockFace::Bottom => AlignmentAxis::Y,
            };
            commands.entity(entity).insert(PlayerAlignment {
                axis,
                aligned_to: planet_ent,
            });

            // Planet transforms are unparented. Use their current rotation, since global
            // transforms may not have propagated yet during this fixed update.
            let target_up = planet_rotation * face.direction().as_vec3();
            // Only correct the up direction so camera-driven yaw is preserved.
            let correction = Quat::from_rotation_arc(*transform.up(), target_up);
            let rotation = transform.rotation;
            transform.rotation = rotation.lerp(correction * rotation, 0.3);
        } else {
            commands.entity(entity).remove::<PlayerAlignment>();
        }
    } else {
        commands.entity(entity).remove::<PlayerAlignment>();
    }
}

fn align_on_ship(query: Query<(Entity, &Pilot), With<LocalPlayer>>, mut commands: Commands) {
    if let Ok((ent, pilot)) = query.single() {
        commands.entity(ent).insert(PlayerAlignment {
            aligned_to: pilot.entity,
            axis: AlignmentAxis::Y,
        });
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Reflect)]
/// Represents an X/Y/Z axis
///
/// Used for orientation on a planet
pub enum AlignmentAxis {
    /// X axis
    X,
    #[default]
    /// Y axis
    Y,
    /// Z axis
    Z,
}

#[derive(Debug, Component, Clone, Copy, PartialEq, Eq, Reflect)]
/// Used to represent the player's orientation on a structure
pub struct PlayerAlignment {
    /// The entity this player is aligned to
    pub aligned_to: Entity,
    /// The axis RELATIVE to the `aligned_to`'s rotation
    pub axis: AlignmentAxis,
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (align_player, align_on_ship)
            .in_set(FixedUpdateSet::Main)
            .after(super::rotate_around_planet::rotate_client_around_planets)
            .before(LocationPhysicsSet::DoPhysics)
            .chain(),
    )
    .register_type::<PlayerAlignment>();
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmos_core::{prelude::Structure, structure::dynamic_structure::DynamicStructure};

    const FACE_NORMALS: [Vec3; 6] = [Vec3::X, Vec3::NEG_X, Vec3::Y, Vec3::NEG_Y, Vec3::Z, Vec3::NEG_Z];

    fn scene(planet_rotation: Quat, normal: Vec3, player_rotation: Quat) -> (App, Entity, Entity) {
        let mut app = App::new();
        register(&mut app);
        super::super::rotate_around_planet::register(&mut app);
        let planet_location = Location::ZERO + Vec3::new(300.0, -200.0, 100.0);
        let planet = app
            .world_mut()
            .spawn((
                Planet::new(300.0),
                planet_location,
                Structure::Dynamic(DynamicStructure::new(16)),
                GravityEmitter {
                    force_per_kg: 9.8,
                    radius: 1000.0,
                },
                Transform::from_rotation(planet_rotation),
                // Intentionally stale: alignment must use the current fixed-step transform.
                GlobalTransform::IDENTITY,
            ))
            .id();
        let player = app
            .world_mut()
            .spawn((
                LocalPlayer,
                planet_location + planet_rotation * (normal * 50.0),
                Transform::from_rotation(player_rotation),
            ))
            .id();
        (app, planet, player)
    }

    #[test]
    fn planet_alignment_preserves_heading_on_every_rotated_face() {
        for planet_rotation in [Quat::IDENTITY, Quat::from_euler(EulerRot::XYZ, 0.4, -0.7, 1.2)] {
            for normal in FACE_NORMALS {
                for yaw in [-2.3, 0.9] {
                    let rotation = planet_rotation * Quat::from_rotation_arc(Vec3::Y, normal) * Quat::from_rotation_y(yaw);
                    let (mut app, planet, player) = scene(planet_rotation, normal, rotation);
                    for _ in 0..10 {
                        app.world_mut().run_schedule(FixedUpdate);
                    }
                    let transform = app.world().get::<Transform>(player).unwrap();
                    assert!((transform.rotation * Vec3::Y).abs_diff_eq(planet_rotation * normal, 1e-4));
                    assert!((transform.rotation * Vec3::NEG_Z).abs_diff_eq(rotation * Vec3::NEG_Z, 1e-4));
                    assert_eq!(app.world().get::<PlayerAlignment>(player).unwrap().aligned_to, planet);
                }
            }
        }
    }

    #[test]
    fn planet_alignment_converges_to_each_outward_face_normal() {
        let planet_rotation = Quat::from_euler(EulerRot::XYZ, 0.4, -0.7, 1.2);
        for normal in FACE_NORMALS {
            let (mut app, _, player) = scene(planet_rotation, normal, planet_rotation * Quat::from_rotation_y(0.9));
            for _ in 0..60 {
                app.world_mut().run_schedule(FixedUpdate);
            }
            let transform = app.world().get::<Transform>(player).unwrap();
            assert!(
                (transform.rotation * Vec3::Y).abs_diff_eq(planet_rotation * normal, 0.002),
                "face {normal:?}"
            );
            assert!((transform.rotation.length() - 1.0).abs() < 1e-5);
        }
    }

    #[test]
    fn planet_rotation_preserves_heading_and_position_relative_to_each_face() {
        for normal in FACE_NORMALS {
            let mut planet_rotation = Quat::from_euler(EulerRot::XYZ, 0.4, -0.7, 1.2);
            let local_rotation = Quat::from_rotation_arc(Vec3::Y, normal) * Quat::from_rotation_y(0.9);
            let (mut app, planet, player) = scene(planet_rotation, normal, planet_rotation * local_rotation);
            app.world_mut().run_schedule(FixedUpdate);
            for _ in 0..120 {
                planet_rotation = (Quat::from_euler(EulerRot::XYZ, 0.01, -0.02, 0.03) * planet_rotation).normalize();
                app.world_mut().get_mut::<Transform>(planet).unwrap().rotation = planet_rotation;
                app.world_mut().run_schedule(FixedUpdate);

                let transform = app.world().get::<Transform>(player).unwrap();
                let expected_rotation = planet_rotation * local_rotation;
                assert!((transform.rotation * Vec3::Y).abs_diff_eq(expected_rotation * Vec3::Y, 0.002));
                assert!((transform.rotation * Vec3::NEG_Z).abs_diff_eq(expected_rotation * Vec3::NEG_Z, 0.002));
                let location = app.world().get::<Location>(player).unwrap();
                let planet_location = app.world().get::<Location>(planet).unwrap();
                assert!(
                    planet_location
                        .relative_coords_to(location)
                        .abs_diff_eq(planet_rotation * (normal * 50.0), 0.01)
                );
            }
        }
    }
}
