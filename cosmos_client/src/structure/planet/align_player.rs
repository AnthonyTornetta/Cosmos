//! Aligns a player to the planet

use std::f32::consts::PI;

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

#[derive(Debug, Component)]
struct PreviousOrientation(AlignmentAxis);

fn align_player(
    mut player: Query<
        (
            Entity,
            &Location,
            &mut Transform,
            Option<&PlayerAlignment>,
            Option<&PreviousOrientation>,
        ),
        (With<LocalPlayer>, Without<ChildOf>),
    >,
    planets: Query<(Entity, &Location, &GravityEmitter, &GlobalTransform), With<Planet>>,
    mut commands: Commands,
) {
    let Ok((entity, location, mut transform, alignment, prev_orientation)) = player.single_mut() else {
        return;
    };
    let mut best_planet = None;
    let mut best_dist = f32::INFINITY;

    for (ent, loc, ge, g_trans) in planets.iter() {
        let dist = loc.distance_sqrd(location);
        if dist < best_dist {
            best_dist = dist;
            best_planet = Some((ent, loc, ge, g_trans));
        }
    }

    if let Some((planet_ent, loc, ge, planet_g_trans)) = best_planet {
        let relative_position = loc.relative_coords_to(location);
        let planet_rotation = Quat::from_affine3(&planet_g_trans.affine());
        let relative_position = planet_rotation.inverse() * relative_position;

        let dist = relative_position.abs().max_element();

        if dist <= ge.radius {
            let face = Planet::planet_face_relative(relative_position);
            if let Some(a) = alignment {
                let old_atlas = match face {
                    BlockFace::Back | BlockFace::Front => AlignmentAxis::Z,
                    BlockFace::Left | BlockFace::Right => AlignmentAxis::X,
                    BlockFace::Top | BlockFace::Bottom => AlignmentAxis::Y,
                };

                if old_atlas != a.axis {
                    commands.entity(entity).insert(PreviousOrientation(a.axis));
                }
            }

            let aligned_to = planet_ent;

            transform.rotation = transform.rotation.lerp(
                planet_rotation
                    * match face {
                        BlockFace::Top => {
                            commands.entity(entity).insert(PlayerAlignment {
                                axis: AlignmentAxis::Y,
                                aligned_to,
                            });
                            Quat::IDENTITY
                        }
                        BlockFace::Bottom => {
                            commands.entity(entity).insert(PlayerAlignment {
                                axis: AlignmentAxis::Y,
                                aligned_to,
                            });

                            match prev_orientation {
                                // Fixes the player rotating in a weird direction when coming from
                                // the left/right faces of a planet.
                                Some(PreviousOrientation(AlignmentAxis::X)) => Quat::from_axis_angle(Vec3::Z, PI),
                                _ => Quat::from_axis_angle(Vec3::X, PI),
                            }
                        }
                        BlockFace::Front => {
                            commands.entity(entity).insert(PlayerAlignment {
                                axis: AlignmentAxis::Z,
                                aligned_to,
                            });
                            Quat::from_axis_angle(Vec3::X, -PI / 2.0)
                        }
                        BlockFace::Back => {
                            commands.entity(entity).insert(PlayerAlignment {
                                axis: AlignmentAxis::Z,
                                aligned_to,
                            });
                            Quat::from_axis_angle(Vec3::X, PI / 2.0)
                        }
                        BlockFace::Right => {
                            commands.entity(entity).insert(PlayerAlignment {
                                axis: AlignmentAxis::X,
                                aligned_to,
                            });
                            Quat::from_axis_angle(Vec3::Z, -PI / 2.0)
                        }
                        BlockFace::Left => {
                            commands.entity(entity).insert(PlayerAlignment {
                                axis: AlignmentAxis::X,
                                aligned_to,
                            });
                            Quat::from_axis_angle(Vec3::Z, PI / 2.0)
                        }
                    },
                0.3,
            );
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

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
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

#[derive(Debug, Component, Clone, Copy, PartialEq, Eq)]
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
            .before(LocationPhysicsSet::DoPhysics)
            .chain(),
    );
}
