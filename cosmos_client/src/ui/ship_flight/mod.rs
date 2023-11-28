//! Displays the information a player sees while piloting a ship

use bevy::{asset::LoadState, prelude::*, utils::HashMap};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        planet::Planet,
        ship::{pilot::Pilot, Ship},
    },
};

use crate::{asset::asset_loader::load_assets, netty::flags::LocalPlayer, rendering::MainCamera, state::game_state::GameState};

#[derive(Clone, Copy, Component, Debug)]
pub struct IndicatorSettings {
    pub color: Color,
    pub offset: Vec3,
    pub max_distance: f32,
}

#[derive(Component, Debug)]
struct Indicator(Entity);

#[derive(Component, Debug)]
struct Indicating(Entity);

#[derive(Resource, Default)]
struct IndicatorImages(HashMap<u32, Handle<Image>>);

fn create_indicator(
    entity: Entity,
    commands: &mut Commands,
    base_texture: Handle<Image>,
    images: &mut Assets<Image>,
    color: Color,
    indicator_images: &mut IndicatorImages,
) {
    let indicator_entity = commands
        .spawn((
            Name::new("Indicator waypoint"),
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
            let (r, g, b) = ((color.r() * 255.0) as u8, (color.g() * 255.0) as u8, (color.b() * 255.0) as u8);

            let color_hash = u32::from_be_bytes([r, g, b, 0]);

            let handle = indicator_images.0.get(&color_hash).map(|x| x.clone_weak()).unwrap_or_else(|| {
                let mut img = images.get(&base_texture).expect("Waypoint diamond image removed?").clone();

                for [img_r, img_g, img_b, _] in img.data.iter_mut().array_chunks::<4>() {
                    *img_r = r;
                    *img_g = g;
                    *img_b = b;
                }

                let handle = images.add(img);

                let weak_clone = handle.clone_weak();

                indicator_images.0.insert(color_hash, handle);

                weak_clone
            });

            let img = images.get(&handle).expect("Missing indicator image.");

            p.spawn(ImageBundle {
                image: UiImage::new(handle),
                style: Style {
                    margin: UiRect {
                        left: Val::Px(img.width() as f32 / -2.0),
                        bottom: Val::Px(img.height() as f32 / -2.0),
                        ..Default::default()
                    },
                    ..Default::default()
                },
                ..Default::default()
            });
        })
        .id();

    commands.entity(entity).insert(Indicator(indicator_entity));
}

#[derive(Resource)]
struct IndicatorImage(Handle<Image>);

fn add_indicators(
    mut commands: Commands,

    all_indicators: Query<(Entity, &Indicator)>,
    nearby_entities: Query<(Entity, &Location, &IndicatorSettings, Option<&Indicator>)>,
    player_piloting: Query<&Pilot, With<LocalPlayer>>,
    location_query: Query<&Location>,

    indicator_image: Res<IndicatorImage>,
    mut images: ResMut<Assets<Image>>,
    mut indicator_images: ResMut<IndicatorImages>,
) {
    let despawn_indicator = |(entity, indicator): (Entity, &Indicator)| {
        commands.entity(indicator.0).despawn_recursive();
        commands.entity(entity).remove::<Indicator>();
    };

    let Ok(pilot) = player_piloting.get_single() else {
        all_indicators.for_each(despawn_indicator);
        return;
    };

    let Ok(player_location) = location_query.get(pilot.entity) else {
        all_indicators.for_each(despawn_indicator);
        return;
    };

    nearby_entities.for_each(|(entity, location, indicator_settings, indicator)| {
        if pilot.entity == entity {
            // Don't put an indicator on the ship you're currently flying
            return;
        }

        let max_distance = indicator_settings.max_distance;
        let max_dist_sqrd = max_distance * max_distance;

        let distance = location.distance_sqrd(player_location);

        if distance <= max_dist_sqrd {
            if indicator.is_none() {
                println!("Creating indicator");
                create_indicator(
                    entity,
                    &mut commands,
                    indicator_image.0.clone_weak(),
                    &mut images,
                    indicator_settings.color,
                    &mut indicator_images,
                );
            }
        } else {
            if let Some(indicator) = indicator {
                println!("Killing indicator!");
                commands.entity(entity).remove::<Indicator>();
                if let Some(ecmds) = commands.get_entity(indicator.0) {
                    ecmds.despawn_recursive();
                }
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
        commands.entity(ent).insert(IndicatorSettings {
            color: Color::hex("FF5733").unwrap(),
            max_distance: 10_000.0,
            offset: Vec3::new(0.5, 0.5, 0.5), // Accounts for the ship core being at 0.5, 0.5, 0.5 instead of the origin
        });
    });
    planet_query.for_each(|ent| {
        commands.entity(ent).insert(IndicatorSettings {
            color: Color::hex("BC8F8F").unwrap(),
            max_distance: 200_000.0,
            offset: Vec3::ZERO,
        });
    });
    player_query.for_each(|ent| {
        commands.entity(ent).insert(IndicatorSettings {
            color: Color::WHITE,
            max_distance: 5_000.0,
            offset: Vec3::ZERO,
        });
    });
}

fn position_diamonds(
    cam_query: Query<(&Camera, &GlobalTransform), With<MainCamera>>,
    mut indicators: Query<(Entity, &mut Style, &Indicating)>,
    global_trans_query: Query<&GlobalTransform>,
    mut commands: Commands,
) {
    let Ok((cam, cam_trans)) = cam_query.get_single() else {
        warn!("Missing main camera.");
        return;
    };

    for (entity, mut style, indicating) in indicators.iter_mut() {
        let Ok(indicating_global_trans) = global_trans_query.get(indicating.0) else {
            // The thing we are indicating has despawned, so despawn the indicator
            commands.entity(entity).despawn_recursive();
            continue;
        };

        let offset = Vec3::splat(0.5);
        let cam_rot = Quat::from_affine3(&indicating_global_trans.affine());

        let entity_location = indicating_global_trans.translation() + cam_rot.mul_vec3(offset);

        // X/Y normalized to [-1, 1] when it's on the screen
        let Some(mut normalized_screen_pos) = cam.world_to_ndc(cam_trans, entity_location) else {
            continue;
        };

        let rot_diff = cam_rot.mul_quat(Quat::from_affine3(&indicating_global_trans.affine()).inverse());

        normalized_screen_pos = rot_diff.inverse().mul_vec3(normalized_screen_pos);

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

        normalized_screen_pos.x = normalized_screen_pos.x.clamp(-1.0, 1.0) / 2.0 + 0.45;
        normalized_screen_pos.y = normalized_screen_pos.y.clamp(-1.0, 1.0) / 2.0 + 0.45;

        style.left = Val::Percent(normalized_screen_pos.x * 100.0);
        style.bottom = Val::Percent(normalized_screen_pos.y * 100.0);

        // Turns it into a circle
        // let clamped = normalized_screen_pos.clamp_length(-0.9, 0.9);

        // println!("{clamped}");
    }
}

const OFFSET_BORDER: f32 = 0.0;

fn is_target_visible(normalized_screen_position: Vec3) -> bool {
    normalized_screen_position.z > 0.0
        && normalized_screen_position.x >= OFFSET_BORDER - 1.0
        && normalized_screen_position.x <= 1.0 - OFFSET_BORDER
        && normalized_screen_position.y >= OFFSET_BORDER - 1.0
        && normalized_screen_position.y <= 1.0 - OFFSET_BORDER
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

    app.init_resource::<IndicatorImages>().add_systems(
        Update,
        (add_indicators.run_if(resource_exists::<IndicatorImage>()), added, position_diamonds).run_if(in_state(GameState::Playing)),
    );
}
