//! Aligns a player to the planet

use std::f32::consts::PI;

use bevy::prelude::*;
use cosmos_core::{
    block::block_face::BlockFace,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    physics::{
        gravity_system::GravityEmitter,
        location::{CosmosBundleSet, Location},
    },
    structure::{planet::Planet, ship::pilot::Pilot},
};

#[derive(Debug, Component)]
struct PreviousOrientation(Axis);

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
                    BlockFace::Back | BlockFace::Front => Axis::Z,
                    BlockFace::Left | BlockFace::Right => Axis::X,
                    BlockFace::Top | BlockFace::Bottom => Axis::Y,
                };

                if old_atlas != a.axis {
                    commands.entity(entity).insert(PreviousOrientation(a.axis));
                }
            }

            let aligned_to = Some(planet_ent);

            transform.rotation = transform.rotation.lerp(
                planet_rotation
                    * match face {
                        BlockFace::Top => {
                            commands.entity(entity).insert(PlayerAlignment { axis: Axis::Y, aligned_to });
                            Quat::IDENTITY
                        }
                        BlockFace::Bottom => {
                            commands.entity(entity).insert(PlayerAlignment { axis: Axis::Y, aligned_to });

                            match prev_orientation {
                                // Fixes the player rotating in a weird direction when coming from
                                // the left/right faces of a planet.
                                Some(PreviousOrientation(Axis::X)) => Quat::from_axis_angle(Vec3::Z, PI),
                                _ => Quat::from_axis_angle(Vec3::X, PI),
                            }
                        }
                        BlockFace::Front => {
                            commands.entity(entity).insert(PlayerAlignment { axis: Axis::Z, aligned_to });
                            Quat::from_axis_angle(Vec3::X, -PI / 2.0)
                        }
                        BlockFace::Back => {
                            commands.entity(entity).insert(PlayerAlignment { axis: Axis::Z, aligned_to });
                            Quat::from_axis_angle(Vec3::X, PI / 2.0)
                        }
                        BlockFace::Right => {
                            commands.entity(entity).insert(PlayerAlignment { axis: Axis::X, aligned_to });
                            Quat::from_axis_angle(Vec3::Z, -PI / 2.0)
                        }
                        BlockFace::Left => {
                            commands.entity(entity).insert(PlayerAlignment { axis: Axis::X, aligned_to });
                            Quat::from_axis_angle(Vec3::Z, PI / 2.0)
                        }
                    },
                0.1,
            );
        } else {
            commands.entity(entity).remove::<PlayerAlignment>();
        }
    }
}

fn align_on_ship(query: Query<Entity, (With<LocalPlayer>, With<Pilot>)>, mut commands: Commands) {
    if let Ok(ent) = query.single() {
        commands.entity(ent).insert(PlayerAlignment {
            aligned_to: None,
            axis: Axis::Y,
        });
    }
}

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
/// Represents an X/Y/Z axis
///
/// Used for orientation on a planet
pub enum Axis {
    /// X axis
    X,
    #[default]
    /// Y axis
    Y,
    /// Z axis
    Z,
}

#[derive(Debug, Component, Clone, Copy, PartialEq, Eq)]
/// Used to represent the player's orientation on a planet
pub struct PlayerAlignment {
    /// The entity this player is aligned to
    pub aligned_to: Option<Entity>,
    /// The axis RELATIVE to the `aligned_to`'s rotation
    pub axis: Axis,
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        (align_player, align_on_ship)
            .in_set(NetworkingSystemsSet::Between)
            .before(CosmosBundleSet::HandleCosmosBundles)
            .chain(),
    );
}
