//! Displays the information a player sees while piloting a ship

use bevy::{asset::LoadState, platform::collections::HashMap, prelude::*};
use cosmos_core::{
    ecs::compute_totally_accurate_global_transform,
    faction::{Faction, FactionId, FactionRelation, Factions},
    netty::client::LocalPlayer,
    physics::location::Location,
    state::GameState,
    structure::{
        asteroid::Asteroid,
        planet::Planet,
        shared::MeltingDown,
        ship::{Ship, pilot::Pilot},
        station::Station,
    },
};

use crate::{
    asset::asset_loader::load_assets,
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    rendering::MainCamera,
    ui::font::DefaultFont,
};

use super::super::components::show_cursor::no_open_menus;

#[derive(Clone, Copy, Component, Debug, Reflect)]
/// Represents something that should have an indicator displayed when the player is piloting a ship
pub struct IndicatorSettings {
    /// The color of the waypoint
    pub color: Color,
    /// The offset of this waypoint from the entity's location
    pub offset: Vec3,
    /// The maximum distance until this waypoint shouldn't be displayed
    ///
    /// To always display it, you can use [`f32::INFINITY`]
    pub max_distance: f32,
}

#[derive(Component, Debug, Reflect)]
/// Represents the entity that is the indicator for this entity
struct HasIndicator(Entity);

#[derive(Component, Debug, Reflect)]
/// Indicates which entity this waypoint is a waypoint for.
pub struct Indicating(pub Entity);

#[derive(Resource, Default)]
struct IndicatorImages(HashMap<u32, Handle<Image>>);

#[derive(Component)]
/// The entity the player has intentionally focused while piloting a ship
pub struct FocusedWaypointEntity;

#[derive(Component)]
struct IndicatorTextEntity(Entity);

#[derive(Resource, Default)]
/// Waypoint closest to the center of your screen NOT your character/ship
pub struct ClosestWaypoint(pub Option<Entity>);

fn get_distance_text(distance: f32) -> String {
    const METERS_TO_KM: f32 = 1.0 / 1000.0;
    const METERS_TO_MEGA_METERS: f32 = METERS_TO_KM / 1000.0;

    if distance < 1_000.0 {
        format!("{}m", distance as i32)
    } else if distance < 1_000_000.0 {
        format!("{:.1}k", distance * METERS_TO_KM)
    } else {
        format!("{:.1}M", distance * METERS_TO_MEGA_METERS)
    }
}

fn create_indicator(
    entity: Entity,
    commands: &mut Commands,
    base_texture: Handle<Image>,
    images: &mut Assets<Image>,
    color: Color,
    indicator_images: &mut IndicatorImages,
    default_font: &DefaultFont,
) {
    let text_color = TextColor(color);
    let text_font = TextFont {
        font: default_font.0.clone(),
        font_size: 16.0,
        ..Default::default()
    };
    let text_style = (text_font, text_color);
    let text_ent = commands
        .spawn((
            Node {
                align_self: AlignSelf::Center,
                margin: UiRect {
                    // Horizontally centers the text - textalign center doesn't work for some reason (shrug)
                    left: Val::Auto,
                    right: Val::Auto,
                    top: Val::Px(90.0),
                    ..Default::default()
                },
                ..default()
            },
            Visibility::Hidden,
            Text::new(""),
            text_style.clone(),
        ))
        .id();

    let indicator_entity = commands
        .spawn((
            Name::new("Indicator Waypoint"),
            IndicatorTextEntity(text_ent),
            Indicating(entity),
            Node {
                position_type: PositionType::Absolute,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            let c = Srgba::from(color);

            let (r, g, b, a) = (
                (c.red * 255.0) as u8,
                (c.green * 255.0) as u8,
                (c.blue * 255.0) as u8,
                (c.alpha * 255.0) as u8,
            );

            let color_hash = u32::from_be_bytes([r, g, b, 0]);

            let handle = indicator_images.0.get(&color_hash).map(|x| x.clone()).unwrap_or_else(|| {
                let mut img = images.get(&base_texture).expect("Waypoint diamond image removed?").clone();

                for [img_r, img_g, img_b, img_a] in img.data.as_mut().expect("Invalid image data").iter_mut().array_chunks::<4>() {
                    *img_r = r;
                    *img_g = g;
                    *img_b = b;
                    *img_a = ((*img_a as f32) / 255.0 * a as f32) as u8;
                }

                let handle = images.add(img);

                let weak_clone = handle.clone();

                indicator_images.0.insert(color_hash, handle);

                weak_clone
            });

            let img = images.get(&handle).expect("Missing indicator image.");

            let (w, h) = (img.width() as f32, img.height() as f32);

            p.spawn((
                ImageNode::new(handle),
                Node {
                    width: Val::Px(w),
                    height: Val::Px(h),
                    margin: UiRect {
                        left: Val::Px(w / -2.0),
                        bottom: Val::Px(h / -2.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            ))
            .add_child(text_ent);
        })
        .id();

    commands.entity(entity).insert(HasIndicator(indicator_entity));
}

#[derive(Resource)]
struct IndicatorImage(Handle<Image>);

fn add_indicators(
    mut commands: Commands,

    all_indicators: Query<(Entity, &HasIndicator)>,
    nearby_entities: Query<(Entity, &Location, &IndicatorSettings, Option<&HasIndicator>)>,
    player_piloting: Query<&Pilot, With<LocalPlayer>>,
    location_query: Query<&Location>,
    mut text_query: Query<&mut Text>,
    indicator_image: Res<IndicatorImage>,
    mut images: ResMut<Assets<Image>>,
    mut indicator_images: ResMut<IndicatorImages>,
    default_font: Res<DefaultFont>,
    q_text_entity_with_focus: Query<&IndicatorTextEntity, With<FocusedWaypointEntity>>,
) {
    let despawn_indicator = |(entity, indicator): (Entity, &HasIndicator)| {
        commands.entity(indicator.0).despawn();
        commands.entity(entity).remove::<HasIndicator>();
    };

    let Ok(pilot) = player_piloting.single() else {
        all_indicators.iter().for_each(despawn_indicator);
        return;
    };

    let Ok(player_location) = location_query.get(pilot.entity) else {
        all_indicators.iter().for_each(despawn_indicator);
        return;
    };

    nearby_entities
        .iter()
        .for_each(|(entity, location, indicator_settings, has_indicator)| {
            if pilot.entity == entity {
                // Don't put an indicator on the ship you're currently flying
                return;
            }

            let max_distance = indicator_settings.max_distance;

            let max_dist_sqrd = max_distance * max_distance;

            let distance_sqrd = location.distance_sqrd(player_location);

            if distance_sqrd <= max_dist_sqrd {
                if let Some(has_indicator) = has_indicator {
                    if let Ok(text_entity) = q_text_entity_with_focus.get(has_indicator.0)
                        && let Ok(mut text) = text_query.get_mut(text_entity.0)
                    {
                        text.0 = get_distance_text(distance_sqrd.sqrt());
                    }
                } else {
                    create_indicator(
                        entity,
                        &mut commands,
                        indicator_image.0.clone(),
                        &mut images,
                        indicator_settings.color,
                        &mut indicator_images,
                        &default_font,
                    );
                }
            } else if let Some(has_indicator) = has_indicator {
                commands.entity(entity).remove::<HasIndicator>();
                if let Ok(mut ecmds) = commands.get_entity(has_indicator.0) {
                    ecmds.despawn();
                }
            }
        });
}

fn added(
    q_local_faction: Query<&FactionId, With<LocalPlayer>>,
    ship_query: Query<(Entity, Option<&FactionId>), (With<Ship>, Or<(Added<Ship>, Changed<FactionId>)>)>,
    station_query: Query<(Entity, Option<&FactionId>), (With<Station>, Or<(Added<Station>, Changed<FactionId>)>)>,
    q_melting_down: Query<Entity, Added<MeltingDown>>,
    asteroid_query: Query<Entity, Added<Asteroid>>,
    planet_query: Query<Entity, Added<Planet>>,
    factions: Res<Factions>,
    mut commands: Commands,
) {
    ship_query.iter().for_each(|(ent, fac)| {
        let player_faction = q_local_faction.single().ok().and_then(|x| factions.from_id(x));
        let relation = Faction::relation_with_option(fac.and_then(|x| factions.from_id(x)), player_faction);

        commands.entity(ent).insert(IndicatorSettings {
            color: Srgba::hex(match relation {
                FactionRelation::Ally => "58ff337F",
                FactionRelation::Neutral => "339eff7F",
                FactionRelation::Enemy => "ff57337F",
            })
            .unwrap()
            .into(),
            max_distance: 20_000.0,
            offset: Vec3::new(0.5, 0.5, 0.5), // Accounts for the ship core being at 0.5, 0.5, 0.5 instead of the origin
        });
    });
    station_query.iter().for_each(|(ent, fac)| {
        let player_faction = q_local_faction.single().ok().and_then(|x| factions.from_id(x));
        let relation = Faction::relation_with_option(fac.and_then(|x| factions.from_id(x)), player_faction);

        commands.entity(ent).insert(IndicatorSettings {
            color: Srgba::hex(match relation {
                FactionRelation::Ally => "00ffbe7F",
                FactionRelation::Neutral => "5b4fff7F",
                FactionRelation::Enemy => "ff00997F",
            })
            .unwrap()
            .into(),
            max_distance: 20_000.0,
            offset: Vec3::new(0.5, 0.5, 0.5), // Accounts for the station core being at 0.5, 0.5, 0.5 instead of the origin
        });
    });
    planet_query.iter().for_each(|ent| {
        commands.entity(ent).insert(IndicatorSettings {
            color: Srgba::hex("BC8F8F7F").unwrap().into(),
            max_distance: 200_000.0,
            offset: Vec3::ZERO,
        });
    });
    asteroid_query.iter().for_each(|ent| {
        commands.entity(ent).insert(IndicatorSettings {
            color: Srgba::hex("6159427F").unwrap().into(),
            max_distance: 20_000.0,
            offset: Vec3::ZERO,
        });
    });
    q_melting_down.iter().for_each(|ent| {
        commands.entity(ent).insert(IndicatorSettings {
            color: Srgba::hex("7777777F").unwrap().into(),
            max_distance: 20_000.0,
            offset: Vec3::new(0.5, 0.5, 0.5), // Accounts for the core being at 0.5, 0.5, 0.5 instead of the origin
        });
    });
}

fn position_diamonds(
    cam_query: Query<(Entity, &Camera), With<MainCamera>>,
    mut indicators: Query<(Entity, &mut Node, &Indicating)>,
    indicator_settings_query: Query<&IndicatorSettings>,
    mut commands: Commands,
    mut closest_waypoint: ResMut<ClosestWaypoint>,
    q_trans: Query<(&Transform, Option<&ChildOf>)>,
) {
    let Ok((cam_ent, cam)) = cam_query.single() else {
        warn!("Missing main camera.");
        return;
    };

    let Some(cam_g_trans) = compute_totally_accurate_global_transform(cam_ent, &q_trans) else {
        warn!("Invalid camera heirarchy.");
        return;
    };

    const MAX_DIST_FROM_CENTER: f32 = 0.4;
    let mut closest = None;

    for (entity, mut style, indicating) in indicators.iter_mut() {
        let Ok(settings) = indicator_settings_query.get(indicating.0) else {
            commands.entity(entity).despawn();
            continue;
        };

        let Some(indicating_global_trans) = compute_totally_accurate_global_transform(indicating.0, &q_trans) else {
            continue;
        };

        let offset = settings.offset;
        let indicating_item_rotation = indicating_global_trans.rotation();

        let entity_location = indicating_global_trans.translation() + indicating_item_rotation * offset;

        // X/Y normalized to [-1, 1] when it's on the screen
        let Some(mut normalized_screen_pos) = cam.world_to_ndc(&cam_g_trans, entity_location) else {
            continue;
        };

        // This code is largely based on https://forum.unity.com/threads/hud-waypoint-indicator-with-problem.1102957/

        if !is_target_visible(normalized_screen_pos) {
            if normalized_screen_pos.z < 0.0 {
                // When z is negative, the x/y coords are inverted
                normalized_screen_pos *= -1.0;
            }

            // Angle between the x-axis (bottom of screen) and a vector starting at zero(bottom-left corner of screen) and terminating at screenPosition.
            let angle = normalized_screen_pos.y.atan2(normalized_screen_pos.x);
            // Slope of the line starting from zero and terminating at screenPosition.
            let slope = angle.tan();

            // Two point's line's form is (y2 - y1) = m (x2 - x1) + c,
            // starting point (x1, y1) is screen botton-left (0, 0),
            // ending point (x2, y2) is one of the screenBounds,
            // m is the slope
            // c is y intercept which will be 0, as line is passing through origin.
            // Final equation will be y = mx.
            if normalized_screen_pos.x > 0.0 {
                // Keep the x screen position to the maximum x bounds and
                // find the y screen position using y = mx.
                normalized_screen_pos = Vec3::new(1.0, slope, 0.0);
            } else {
                normalized_screen_pos = Vec3::new(-1.0, -slope, 0.0);
            }
            // Incase the y ScreenPosition exceeds the y screenBounds
            if normalized_screen_pos.y > 1.0 {
                // Keep the y screen position to the maximum y bounds and
                // find the x screen position using x = y/m.
                normalized_screen_pos = Vec3::new(1.0 / slope, 1.0, 0.0);
            } else if normalized_screen_pos.y < -1.0 {
                normalized_screen_pos = Vec3::new(-1.0 / slope, -1.0, 0.0);
            }
        }

        let x = normalized_screen_pos.x.abs();
        let y = normalized_screen_pos.y.abs();
        if x < MAX_DIST_FROM_CENTER && y < MAX_DIST_FROM_CENTER {
            let dist_sqrd = x * x + y * y;

            if closest.as_ref().map(|(_, dist)| dist_sqrd < *dist).unwrap_or(true) {
                closest = Some((entity, dist_sqrd));
            }
        }

        normalized_screen_pos.x = normalized_screen_pos.x.clamp(-0.9, 0.9) / 2.0 + 0.5;
        normalized_screen_pos.y = normalized_screen_pos.y.clamp(-0.9, 0.9) / 2.0 + 0.5;

        style.left = Val::Percent(normalized_screen_pos.x * 100.0);
        style.bottom = Val::Percent(normalized_screen_pos.y * 100.0);
    }

    closest_waypoint.0 = closest.map(|x| x.0);
}

fn focus_waypoint(
    inputs: InputChecker,
    focused: Query<(Entity, &IndicatorTextEntity), With<FocusedWaypointEntity>>,
    q_indicator_text: Query<&IndicatorTextEntity>,
    mut visibility: Query<&mut Visibility>,
    closest_waypoint: Res<ClosestWaypoint>,
    mut commands: Commands,
) {
    if !inputs.check_just_pressed(CosmosInputs::FocusWaypoint) {
        return;
    }

    if let Ok((current_ent, indicator_text_ent)) = focused.single() {
        *visibility.get_mut(indicator_text_ent.0).expect("This always has visibility") = Visibility::Hidden;
        commands.entity(current_ent).remove::<FocusedWaypointEntity>();

        if let Some(closest_waypoint) = closest_waypoint.0
            && current_ent != closest_waypoint
        {
            let Ok(closest) = q_indicator_text.get(closest_waypoint) else {
                return;
            };

            *visibility.get_mut(closest.0).expect("This always has visibility") = Visibility::Visible;
            commands.entity(closest_waypoint).insert(FocusedWaypointEntity);
        }
    } else if let Some(closest_waypoint) = closest_waypoint.0 {
        let Ok(closest) = q_indicator_text.get(closest_waypoint) else {
            return;
        };

        *visibility.get_mut(closest.0).expect("This always has visibility") = Visibility::Visible;
        commands.entity(closest_waypoint).insert(FocusedWaypointEntity);
    }
}

fn is_target_visible(normalized_screen_position: Vec3) -> bool {
    normalized_screen_position.z > 0.0
        && normalized_screen_position.x >= -1.0
        && normalized_screen_position.x <= 1.0
        && normalized_screen_position.y >= -1.0
        && normalized_screen_position.y <= 1.0
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// The creation and focusing of waypoints when flying a ship
pub enum WaypointSet {
    /// Creates waypoints when the player becomes a pilot or new entities enter within range
    CreateWaypoints,
    /// Handles the user focusing/unfocusing waypoints
    FocusWaypoints,
}

pub(super) fn register(app: &mut App) {
    load_assets::<Image, IndicatorImage, 1>(
        app,
        GameState::PreLoading,
        ["cosmos/images/ui/diamond.png"],
        |mut commands, [(handle, state)]| {
            if !matches!(state, LoadState::Loaded) {
                warn!("Failed to load diamond.png for ship UI!");
                return;
            }
            commands.insert_resource(IndicatorImage(handle));
        },
    );

    app.configure_sets(Update, (WaypointSet::CreateWaypoints, WaypointSet::FocusWaypoints).chain());

    app.init_resource::<IndicatorImages>()
        .init_resource::<ClosestWaypoint>()
        .add_systems(
            Update,
            (
                (add_indicators.run_if(resource_exists::<IndicatorImage>), added, position_diamonds)
                    .chain()
                    .in_set(WaypointSet::CreateWaypoints),
                focus_waypoint.in_set(WaypointSet::FocusWaypoints).run_if(no_open_menus),
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        )
        .register_type::<IndicatorSettings>()
        .register_type::<Indicating>()
        .register_type::<HasIndicator>();
}
