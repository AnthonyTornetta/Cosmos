//! Map-waypoint logic

use bevy::{color::palettes::css, prelude::*};
use cosmos_core::{
    ecs::{NeedsDespawned, sets::FixedUpdateSet},
    netty::client::LocalPlayer,
    physics::location::Location,
    structure::ship::{pilot::Pilot, warp::DesiredLocation},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::ship_flight::indicators::{FocusedWaypointEntity, Indicating, IndicatorSettings},
};

use super::{GalaxyMapDisplay, MapCamera};

#[derive(Component)]
/// A point that the client has marked on their map.
///
/// The entity this is on should have a [`Location`], which is where the waypoint is.
pub struct Waypoint;

fn create_waypoint(
    input_checker: InputChecker,
    q_open_map: Query<&GalaxyMapDisplay>,
    q_map_cam: Query<&MapCamera>,
    q_waypoint: Query<Entity, With<Waypoint>>,
    mut commands: Commands,
) {
    if q_open_map.iter().next().is_none() {
        return;
    }

    if !input_checker.check_just_pressed(CosmosInputs::ToggleWaypoint) {
        return;
    }

    if let Ok(waypoint) = q_waypoint.single() {
        commands.entity(waypoint).insert(NeedsDespawned);
    } else {
        let Ok(map_cam) = q_map_cam.single() else {
            return;
        };

        commands.spawn((
            Name::new("Waypoint"),
            IndicatorSettings {
                color: css::WHITE.into(),
                max_distance: f32::INFINITY,
                offset: Vec3::ZERO,
            },
            Location::new(Vec3::ZERO, map_cam.sector),
            Waypoint,
        ));
    }
}

fn set_desired_location(
    q_waypoint: Query<&Indicating, With<FocusedWaypointEntity>>,
    q_loc: Query<&Location>,
    q_piloting: Query<&Pilot, With<LocalPlayer>>,
    mut commands: Commands,
) {
    let Ok(ind) = q_waypoint.single() else {
        return;
    };

    let Ok(loc) = q_loc.get(ind.0) else {
        return;
    };

    let Ok(pilot) = q_piloting.single() else {
        return;
    };

    commands.entity(pilot.entity).insert(DesiredLocation(*loc));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, create_waypoint)
        .add_systems(FixedUpdate, set_desired_location.in_set(FixedUpdateSet::PostPhysics));
}
