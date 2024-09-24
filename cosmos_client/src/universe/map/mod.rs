use bevy::{
    app::Update,
    color::palettes::css,
    core::Name,
    core_pipeline::bloom::BloomSettings,
    prelude::{
        in_state, App, Camera, Camera3d, Camera3dBundle, Commands, Component, Entity, IntoSystemConfigs, OnEnter, PerspectiveProjection,
        Projection, Query, Transform, TransformBundle, VisibilityBundle, With,
    },
    render::{
        camera::RenderTarget,
        view::{ColorGrading, RenderLayers},
    },
};
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    physics::location::{Location, UniverseSystem},
    universe::map::system::SystemMap,
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    state::game_state::GameState,
};

#[derive(Component)]
enum GalaxyMapDisplay {
    Loading(UniverseSystem),
    Map(SystemMap),
}

const CAMERA_LAYER: usize = 0b1000;

#[derive(Component)]
struct MapCamera;

fn create_map_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: false,
                order: 20,
                is_active: false,
                clear_color: bevy::prelude::ClearColorConfig::Custom(css::BLACK.into()),
                ..Default::default()
            },
            transform: Transform::default(),
            projection: Projection::from(PerspectiveProjection {
                fov: (90.0 / 180.0) * std::f32::consts::PI,
                ..Default::default()
            }),
            ..Default::default()
        },
        BloomSettings { ..Default::default() },
        Name::new("Map Camera"),
        RenderLayers::from_layers(&[CAMERA_LAYER]),
        MapCamera,
    ));
    /*
    *Name::new("UI Top Camera"),
            UiTopRoot,
            Camera3dBundle {
                projection: Projection::Orthographic(OrthographicProjection {
                    scaling_mode: ScalingMode::WindowSize(40.0),
                    ..Default::default()
                }),
                camera_3d: Camera3d::default(),
                camera: Camera {
                    order: 2,
                    clear_color: ClearColorConfig::Custom(Color::NONE),
                    hdr: true, // Transparent stuff fails to render properly if this is off - this may be a bevy bug?
                    ..Default::default()
                },
                transform: Transform::from_xyz(0.0, 0.0, 1.0).looking_at(Vec3::ZERO, Vec3::Y),
                ..Default::default()
            },
            RenderLayers::from_layers(&[INVENTORY_SLOT_LAYER]),

    */
}

fn toggle_map(
    q_galaxy_map_display: Query<(Entity, &GalaxyMapDisplay)>,
    input_handler: InputChecker,
    q_player: Query<&Location, With<LocalPlayer>>,
    mut commands: Commands,
    mut q_map_camera: Query<&mut Camera, With<MapCamera>>,
) {
    if !input_handler.check_just_pressed(CosmosInputs::ToggleMap) {
        return;
    }

    let Ok(mut map_camera) = q_map_camera.get_single_mut() else {
        return;
    };

    if let Ok((galaxy_map_entity, galaxy_map_display)) = q_galaxy_map_display.get_single() {
        map_camera.is_active = false;
        commands.entity(galaxy_map_entity).insert(NeedsDespawned);
        return;
    }

    let Ok(player_loc) = q_player.get_single() else {
        return;
    };

    map_camera.is_active = true;
    commands.spawn((
        GalaxyMapDisplay::Loading(player_loc.get_system_coordinates()),
        RenderLayers::from_layers(&[CAMERA_LAYER]),
        TransformBundle::default(),
        VisibilityBundle::default(),
    ));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), create_map_camera).add_systems(
        Update,
        toggle_map
            .run_if(in_state(GameState::Playing))
            .in_set(NetworkingSystemsSet::Between),
    );
}
