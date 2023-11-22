//! Displays the information a player sees while piloting a ship

use bevy::{asset::LoadState, prelude::*};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        planet::Planet,
        ship::{pilot::Pilot, Ship},
    },
    utils::smooth_clamp::SmoothClamp,
};

use crate::{asset::asset_loader::load_assets, netty::flags::LocalPlayer, state::game_state::GameState};

#[derive(Clone, Copy, Component, Debug)]
enum IndicatorType {
    Ship,
    Planet,
    Player,
}

#[derive(Component, Debug)]
struct Indicator(Entity);

#[derive(Component, Debug)]
struct Indicating(Entity);

fn create_indicator(entity: Entity, commands: &mut Commands, texture: Handle<Image>) {
    let indicator_entity = commands
        .spawn((
            Indicating(entity),
            NodeBundle {
                style: Style {
                    position_type: PositionType::Absolute,
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn(ImageBundle {
                image: UiImage::new(texture),
                ..Default::default()
            });
        })
        .id();

    // https://forum.unity.com/threads/hud-waypoint-indicator-with-problem.1102957/

    commands.entity(entity).insert(Indicator(indicator_entity));
}

#[derive(Resource)]
struct IndicatorImage(Handle<Image>);

fn add_indicators(
    mut commands: Commands,

    all_indicators: Query<&Indicator>,
    nearby_entities: Query<(Entity, &Location, &IndicatorType, Option<&Indicator>)>,
    player_piloting: Query<&Pilot, With<LocalPlayer>>,
    location_query: Query<&Location>,

    indicator_image: Res<IndicatorImage>,
) {
    let Ok(pilot) = player_piloting.get_single() else {
        all_indicators.for_each(|indicator| commands.entity(indicator.0).despawn());
        return;
    };

    let Ok(player_location) = location_query.get(pilot.entity) else {
        all_indicators.for_each(|indicator| commands.entity(indicator.0).despawn());
        return;
    };

    nearby_entities.for_each(|(entity, location, indicator_type, indicator)| {
        let max_distance = match indicator_type {
            IndicatorType::Planet => 50_000.0,
            IndicatorType::Ship => 10_000.0,
            IndicatorType::Player => 5_000.0,
        };

        let max_dist_sqrd = max_distance * max_distance;

        let distance = location.distance_sqrd(player_location);

        if distance <= max_dist_sqrd {
            if indicator.is_none() {
                create_indicator(entity, &mut commands, indicator_image.0.clone_weak());
            }
        } else {
            if let Some(indicator) = indicator {
                commands.entity(indicator.0).despawn_recursive();
            }
        }
    });
}

fn added(
    ship_query: Query<Entity, Added<Ship>>,
    planet_query: Query<Entity, Added<Planet>>,
    player_query: Query<Entity, (Added<Player>, Without<LocalPlayer>)>,
    mut commands: Commands,
) {
    ship_query.for_each(|ent| {
        commands.entity(ent).insert(IndicatorType::Ship);
    });
    planet_query.for_each(|ent| {
        commands.entity(ent).insert(IndicatorType::Planet);
    });
    player_query.for_each(|ent| {
        commands.entity(ent).insert(IndicatorType::Player);
    });
}

pub(super) fn register(app: &mut App) {
    load_assets::<Image, IndicatorImage>(
        app,
        GameState::PreLoading,
        vec!["cosmos/images/ui/diamond.png"],
        |mut commands, mut handles| {
            let (handle, state) = handles.remove(0);
            if state != LoadState::Loaded {
                warn!("Failed to load diamond.png for ship UI!");
                return;
            }
            commands.insert_resource(IndicatorImage(handle));
        },
    );

    app.add_systems(
        Update,
        (add_indicators.run_if(resource_exists::<IndicatorImage>()), added).run_if(in_state(GameState::Playing)),
    );
}
