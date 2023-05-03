//! Aligns a player to the planet

use std::f32::consts::PI;

use bevy::prelude::{
    App, Commands, Component, Entity, Parent, Quat, Query, Transform, Vec3, With, Without,
};
use cosmos_core::{
    block::BlockFace,
    physics::{gravity_system::GravityEmitter, location::Location},
    structure::planet::Planet,
};

use crate::netty::flags::LocalPlayer;

fn align_player(
    mut player: Query<(Entity, &Location, &mut Transform), (With<LocalPlayer>, Without<Parent>)>,
    planets: Query<(&Location, &GravityEmitter), With<Planet>>,
    mut commands: Commands,
) {
    if let Ok((entity, location, mut transform)) = player.get_single_mut() {
        let mut best_planet = None;
        let mut best_dist = f32::INFINITY;

        for (loc, ge) in planets.iter() {
            let dist = loc.distance_sqrd(location);
            if dist < best_dist {
                best_dist = dist;
                best_planet = Some((loc, ge));
            }
        }

        if let Some((loc, ge)) = best_planet {
            let relative_position = loc.relative_coords_to(location);

            let dist = relative_position.max_element();

            if dist <= ge.radius {
                let face = Planet::planet_face_relative(relative_position);

                transform.rotation = transform.rotation.lerp(
                    match face {
                        BlockFace::Top => {
                            commands.entity(entity).insert(PlayerAlignment(Axis::Y));
                            Quat::IDENTITY
                        }
                        BlockFace::Bottom => {
                            commands.entity(entity).insert(PlayerAlignment(Axis::Y));
                            Quat::from_axis_angle(Vec3::X, PI)
                        }
                        BlockFace::Back => {
                            commands.entity(entity).insert(PlayerAlignment(Axis::Z));
                            Quat::from_axis_angle(Vec3::X, -PI / 2.0)
                        }
                        BlockFace::Front => {
                            commands.entity(entity).insert(PlayerAlignment(Axis::Z));
                            Quat::from_axis_angle(Vec3::X, PI / 2.0)
                        }
                        BlockFace::Right => {
                            commands.entity(entity).insert(PlayerAlignment(Axis::X));
                            Quat::from_axis_angle(Vec3::Z, -PI / 2.0)
                        }
                        BlockFace::Left => {
                            commands.entity(entity).insert(PlayerAlignment(Axis::X));
                            Quat::from_axis_angle(Vec3::Z, PI / 2.0)
                        }
                    },
                    0.1,
                );
            }
        }
    }
}

#[derive(Debug, Default, Clone, Copy)]
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

#[derive(Debug, Component, Default)]
/// Used to represent the player's orientation on a planet
pub struct PlayerAlignment(pub Axis);

pub(super) fn register(app: &mut App) {
    app.add_system(align_player);
}
