use std::f32::consts::PI;

use bevy::{
    app::Update,
    asset::{AssetServer, Assets, Handle},
    color::{palettes::css, Alpha, Color},
    core::Name,
    core_pipeline::bloom::BloomSettings,
    input::mouse::{MouseScrollUnit, MouseWheel},
    math::{Dir3, Quat, Vec3},
    pbr::{PbrBundle, StandardMaterial},
    prelude::{
        in_state, AlphaMode, App, BuildChildren, Camera, Camera3dBundle, Capsule3d, Changed, Commands, Component, Cuboid, Entity,
        EventReader, IntoSystemConfigs, Mesh, MouseButton, OnEnter, PerspectiveProjection, Projection, Query, Res, ResMut, Sphere,
        Transform, TransformBundle, VisibilityBundle, With, Without,
    },
    reflect::Reflect,
    render::view::RenderLayers,
    text::{Text, TextStyle},
    time::Time,
};
use bevy_mod_billboard::{BillboardDepth, BillboardTextBundle};
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    physics::location::{Location, Sector, SectorUnit, SystemCoordinate},
    registry::{identifiable::Identifiable, Registry},
    state::GameState,
    structure::planet::biosphere::Biosphere,
    universe::map::system::{Destination, RequestSystemMap, SystemMap, SystemMapResponseEvent},
};

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    structure::planet::biosphere::BiosphereColor,
    ui::{components::show_cursor::ShowCursor, OpenMenu, UiSystemSet},
    window::setup::DeltaCursorPosition,
};

#[derive(Component, Debug)]
enum GalaxyMapDisplay {
    Loading(SystemCoordinate),
    Map { map: SystemMap, system: SystemCoordinate },
}

const CAMERA_LAYER: usize = 0b1000;

#[derive(Component, Reflect)]
struct MapCamera {
    relative_sector: Sector,
    lerp_sector: Vec3,
    zoom: f32,
    yaw: f32,
    pitch: f32,
}

impl Default for MapCamera {
    fn default() -> Self {
        Self {
            relative_sector: Sector::default(),
            lerp_sector: Vec3::ZERO,
            zoom: 2.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn create_map_camera(mut commands: Commands) {
    commands.spawn((
        Camera3dBundle {
            camera: Camera {
                hdr: true,
                msaa_writeback: false, // override all other cameras
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
        MapCamera::default(),
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
    mut q_map_camera: Query<&mut Transform, With<MapCamera>>,
    mut nevw_galaxy_map: NettyEventWriter<RequestSystemMap>,
) {
    if !input_handler.check_just_pressed(CosmosInputs::ToggleMap) {
        return;
    }

    let Ok(mut map_camera) = q_map_camera.get_single_mut() else {
        return;
    };

    if let Ok((galaxy_map_entity, galaxy_map_display)) = q_galaxy_map_display.get_single() {
        commands.entity(galaxy_map_entity).insert(NeedsDespawned);
        return;
    }

    let Ok(player_loc) = q_player.get_single() else {
        return;
    };

    let player_system = player_loc.get_system_coordinates();

    map_camera.translation = Vec3::ZERO;

    commands.spawn((
        GalaxyMapDisplay::Loading(player_system),
        OpenMenu::new(0),
        RenderLayers::from_layers(&[CAMERA_LAYER]),
        Name::new("System map display"),
        TransformBundle::default(),
        VisibilityBundle::default(),
        ShowCursor,
    ));
    println!("Sending map!!");
    nevw_galaxy_map.send(RequestSystemMap { system: player_system });
}

const SECTOR_SCALE: f32 = 1.0;

fn position_camera(mut q_camera: Query<(&mut Transform, &mut MapCamera)>) {
    let Ok((mut trans, mut cam)) = q_camera.get_single_mut() else {
        return;
    };

    let s = cam.relative_sector;
    let vec_sec = Vec3::new(s.x() as f32, s.y() as f32, s.z() as f32) * SECTOR_SCALE;
    cam.lerp_sector = cam.lerp_sector.lerp(vec_sec, 0.1);

    trans.rotation = Quat::from_rotation_y(cam.yaw) * Quat::from_rotation_x(-cam.pitch);
    trans.translation = cam.lerp_sector + trans.rotation * Vec3::new(0.0, 0.0, cam.zoom * SECTOR_SCALE);
}

fn handle_selected_sector(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q_selected_sector: Query<(&mut Transform, &Handle<StandardMaterial>), (With<SelectedSector>, Without<MapCamera>)>,
    q_camera: Query<&MapCamera>,
    time: Res<Time>,
    mut q_sector_text: Query<&mut Text, With<SelectedSectorText>>,
) {
    let Ok(cam) = q_camera.get_single() else {
        return;
    };

    let Ok((mut sector_trans, standard_material)) = q_selected_sector.get_single_mut() else {
        return;
    };

    sector_trans.translation = cam.lerp_sector;
    let standard_material = materials.get_mut(standard_material).expect("Material missing");
    standard_material.base_color.set_alpha(time.elapsed_seconds().sin().abs() * 0.1);

    let Ok(mut text) = q_sector_text.get_single_mut() else {
        return;
    };

    text.sections[0].value = format!(
        "{}, {}, {}",
        cam.relative_sector.x(),
        cam.relative_sector.y(),
        cam.relative_sector.z()
    );
}

#[derive(Component)]
struct SelectedSector;

#[derive(Component)]
struct SelectedSectorText;

fn render_galaxy_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_changed_map: Query<(Entity, &GalaxyMapDisplay), Changed<GalaxyMapDisplay>>,
    q_player_loc: Query<&Location, With<LocalPlayer>>,
    mut q_camera: Query<(&mut Transform, &mut MapCamera)>,
    biospheres: Res<Registry<Biosphere>>,
    biosphere_color: Res<Registry<BiosphereColor>>,
    asset_server: Res<AssetServer>,
) {
    for (ent, galaxy_map_display) in q_changed_map.iter() {
        let GalaxyMapDisplay::Map { map, system } = galaxy_map_display else {
            continue;
        };

        let Ok(player) = q_player_loc.get_single() else {
            return;
        };

        let Ok((mut cam_trans, mut cam)) = q_camera.get_single_mut() else {
            return;
        };

        cam.relative_sector = player.relative_sector();
        cam.lerp_sector = Vec3::new(
            cam.relative_sector.x() as f32,
            cam.relative_sector.y() as f32,
            cam.relative_sector.z() as f32,
        ) * SECTOR_SCALE;
        // let player_translation = Vec3::new(diff.x() as f32, diff.y() as f32, diff.z() as f32) * SECTOR_SCALE;
        // cam_trans.translation = player_translation + Vec3::new(1.0, 2.0, 2.0) * SECTOR_SCALE;
        // cam.relative_sector = diff + Sector::new(1, 2, 2);
        // cam_trans.look_at(player_translation, Vec3::Y);

        let font = asset_server.load("fonts/PixeloidSans.ttf");
        let text_style = TextStyle {
            color: Color::WHITE,
            font_size: 22.0,
            font: font.clone(),
        };

        commands.entity(ent).with_children(|p| {
            p.spawn((
                Name::new("Selected Sector"),
                SelectedSector,
                RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                PbrBundle {
                    mesh: meshes.add(Cuboid::new(SECTOR_SCALE, SECTOR_SCALE, SECTOR_SCALE)),
                    material: materials.add(StandardMaterial {
                        base_color: css::YELLOW.into(),
                        unlit: true,
                        alpha_mode: AlphaMode::Blend,
                        ..Default::default()
                    }),
                    transform: Transform::from_translation(cam.lerp_sector),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                    SelectedSectorText,
                    Name::new("Selected text"),
                    BillboardTextBundle {
                        billboard_depth: BillboardDepth(false),
                        transform: Transform::from_scale(Vec3::splat(0.008)),
                        text: Text::from_section(
                            format!(
                                "{}, {}, {}",
                                cam.relative_sector.x(),
                                cam.relative_sector.y(),
                                cam.relative_sector.z()
                            ),
                            text_style,
                        ),
                        ..Default::default()
                    },
                ));
            });

            for (sector, destination) in map.destinations() {
                let transform = Transform::from_xyz(
                    sector.x() as f32 * SECTOR_SCALE,
                    sector.y() as f32 * SECTOR_SCALE,
                    sector.z() as f32 * SECTOR_SCALE,
                );

                let mesh = match destination {
                    Destination::Star(_) => meshes.add(Sphere::new(0.75)),
                    Destination::Planet(_) => meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
                    Destination::Player(_) => meshes.add(Capsule3d::new(0.05, 0.1)),
                    Destination::Asteroid(_) => meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
                    Destination::Unknown(_) => meshes.add(Sphere::new(0.1)),
                    Destination::Ship(_) => meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
                    Destination::Station(_) => meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
                    // _ => meshes.add(Cuboid::new(0.1, 0.1, 0.1)),
                };

                let size = match destination {
                    Destination::Star(_) => 1.0,
                    Destination::Planet(_) => 0.6,
                    Destination::Asteroid(_) => 0.5,
                    Destination::Station(_) => 0.4,
                    Destination::Ship(_) => 0.3,
                    Destination::Unknown(_) => 0.2,
                    Destination::Player(_) => 0.1,
                };

                let material = match destination {
                    Destination::Star(star) => materials.add(StandardMaterial::from_color(star.star.color())),
                    Destination::Planet(planet) => materials.add(StandardMaterial::from_color(
                        biosphere_color
                            .from_id(biospheres.from_numeric_id(planet.biosphere_id).unlocalized_name())
                            .map(|x| x.color())
                            .unwrap_or(css::HOT_PINK.into()),
                    )),
                    Destination::Player(_) => materials.add(StandardMaterial::from_color(css::GREEN)),
                    Destination::Asteroid(_) => materials.add(StandardMaterial::from_color(css::GREY)),
                    Destination::Unknown(_) => materials.add(StandardMaterial::from_color(css::WHITE)),
                    Destination::Ship(_) => materials.add(StandardMaterial::from_color(css::ORANGE)),
                    Destination::Station(_) => materials.add(StandardMaterial::from_color(css::PURPLE)),
                };

                let mut ecmds = p.spawn((
                    RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                    PbrBundle {
                        transform,
                        mesh,
                        material,
                        ..Default::default()
                    },
                ));

                // p.spawn((TransformBundle::from_transform(transform), VisibilityBundle::default()));
            }
        });
    }
}

fn sector_direction(v: Dir3, amount: SectorUnit) -> Sector {
    let x = v.x.abs();
    let y = v.y.abs();
    let z = v.z.abs();

    if x >= y && x >= z {
        Sector::new(v.x.signum() as SectorUnit * amount, 0, 0)
    } else if y >= x && y >= z {
        Sector::new(0, v.y.signum() as SectorUnit * amount, 0)
    } else {
        Sector::new(0, 0, v.z.signum() as SectorUnit * amount)
    }
}

fn camera_movement(
    delta: Res<DeltaCursorPosition>,
    input_handler: InputChecker,
    mut q_camera: Query<(&Transform, &mut MapCamera)>,
    mut evr_mouse_wheel: EventReader<MouseWheel>,
    q_local_player: Query<&Location, With<LocalPlayer>>,
) {
    for (trans, mut cam) in q_camera.iter_mut() {
        let amount = if input_handler.check_pressed(CosmosInputs::Sprint) { 10 } else { 1 };

        if input_handler.check_just_pressed(CosmosInputs::MoveForward) {
            cam.relative_sector = cam.relative_sector + sector_direction(trans.forward(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveBackward) {
            cam.relative_sector = cam.relative_sector + sector_direction(trans.back(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveLeft) {
            cam.relative_sector = cam.relative_sector + sector_direction(trans.left(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveRight) {
            cam.relative_sector = cam.relative_sector + sector_direction(trans.right(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveUp) {
            cam.relative_sector = cam.relative_sector + sector_direction(trans.up(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveDown) {
            cam.relative_sector = cam.relative_sector + sector_direction(trans.down(), amount);
        }

        if input_handler.check_just_pressed(CosmosInputs::ResetMapPosition) {
            let Ok(player_loc) = q_local_player.get_single() else {
                continue;
            };

            cam.relative_sector = player_loc.relative_sector();
        }

        if input_handler.mouse_inputs().pressed(MouseButton::Left) {
            let pitch_pi_interval = ((cam.pitch + PI / 2.0) / (PI / 1.0)).floor() as i32;
            let yaw_sign = if pitch_pi_interval % 2 == 0 { -1 } else { 1 };

            cam.pitch += -delta.y * 0.001;
            cam.yaw += yaw_sign as f32 * delta.x * 0.001;
        }

        for mw in evr_mouse_wheel.read() {
            let dy = match mw.unit {
                MouseScrollUnit::Line => mw.y,
                MouseScrollUnit::Pixel => mw.y * 0.005,
            };

            cam.zoom -= dy;
        }

        cam.zoom = cam.zoom.clamp(0.05, 100.0);
    }
}

fn handle_map_camera(mut q_map_camera: Query<&mut Camera, With<MapCamera>>, q_exists: Query<(), With<GalaxyMapDisplay>>) {
    let Ok(mut cam) = q_map_camera.get_single_mut() else {
        return;
    };

    cam.is_active = !q_exists.is_empty();
}

fn receive_map(mut nevr: EventReader<SystemMapResponseEvent>, mut q_galaxy_map: Query<&mut GalaxyMapDisplay>) {
    for ev in nevr.read() {
        let Ok(mut gmap) = q_galaxy_map.get_single_mut() else {
            return;
        };

        *gmap = GalaxyMapDisplay::Map {
            map: ev.map.clone(),
            system: ev.system,
        };
    }
}

fn map_active(q_map: Query<(), With<GalaxyMapDisplay>>) -> bool {
    !q_map.is_empty()
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Playing), create_map_camera)
        .add_systems(
            Update,
            (
                (
                    toggle_map,
                    receive_map,
                    render_galaxy_map,
                    (camera_movement, position_camera, handle_selected_sector)
                        .chain()
                        .run_if(map_active),
                )
                    .chain()
                    .before(UiSystemSet::DoUi),
                handle_map_camera.after(UiSystemSet::FinishUi),
            )
                .chain()
                .run_if(in_state(GameState::Playing))
                .in_set(NetworkingSystemsSet::Between),
        )
        .register_type::<MapCamera>();
}
