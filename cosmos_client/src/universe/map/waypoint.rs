use std::f32::INFINITY;

use bevy::{
    app::Update,
    color::{palettes::css, Srgba},
    core::Name,
    math::Vec3,
    prelude::{App, Commands, Component, Entity, IntoSystemConfigs, Query, With},
};
use cosmos_core::{ecs::NeedsDespawned, netty::system_sets::NetworkingSystemsSet, physics::location::Location};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    ui::ship_flight::indicators::IndicatorSettings,
};

use super::{GalaxyMapDisplay, MapCamera};

#[derive(Component)]
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

    if let Ok(waypoint) = q_waypoint.get_single() {
        commands.entity(waypoint).insert(NeedsDespawned);
    } else {
        let Ok(map_cam) = q_map_cam.get_single() else {
            return;
        };

        commands.spawn((
            Name::new("Waypoint"),
            IndicatorSettings {
                color: css::WHITE.into(),
                max_distance: INFINITY,
                offset: Vec3::ZERO,
            },
            Location::new(Vec3::ZERO, map_cam.sector),
            Waypoint,
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, create_waypoint.in_set(NetworkingSystemsSet::Between));
}
