//! Client map display logic

use std::f32::consts::PI;

use bevy::{
    app::Update,
    asset::{AssetServer, Assets},
    color::{Alpha, palettes::css},
    core::Name,
    core_pipeline::bloom::Bloom,
    input::mouse::{MouseScrollUnit, MouseWheel},
    math::{Dir3, Quat, Vec3},
    pbr::{MeshMaterial3d, StandardMaterial},
    prelude::{
        AlphaMode, App, BuildChildren, Camera, Camera3d, Capsule3d, Changed, ChildBuild, Commands, Component, Cuboid, Entity, EventReader,
        IntoSystemConfigs, Mesh, Mesh3d, MouseButton, OnEnter, PerspectiveProjection, Projection, Query, Res, ResMut, Sphere, Text,
        Transform, Visibility, With, Without, in_state,
    },
    reflect::Reflect,
    render::view::RenderLayers,
    text::TextFont,
    time::Time,
    ui::{AlignSelf, FlexDirection, JustifyContent, Node, PositionType, TargetCamera, UiRect, Val},
};
// use bevy_mod_billboard::{BillboardDepth, BillboardTextBundle};
use cosmos_core::{
    ecs::NeedsDespawned,
    faction::FactionRelation,
    netty::{client::LocalPlayer, sync::events::client_event::NettyEventWriter, system_sets::NetworkingSystemsSet},
    physics::location::{Location, Sector, SectorUnit},
    registry::{Registry, identifiable::Identifiable},
    state::GameState,
    structure::planet::biosphere::Biosphere,
    universe::map::system::{
        Destination, GalaxyMap, GalaxyMapResponseEvent, RequestGalaxyMap, RequestSystemMap, SystemMap, SystemMapResponseEvent,
    },
    utils::ecs::DespawnWith,
};
use waypoint::Waypoint;

use crate::{
    input::inputs::{CosmosInputs, InputChecker, InputHandler},
    structure::planet::biosphere::BiosphereColor,
    ui::{OpenMenu, UiSystemSet, components::show_cursor::ShowCursor},
    window::setup::DeltaCursorPosition,
};

pub mod waypoint;

#[derive(Component, Debug)]
enum GalaxyMapDisplay {
    Loading,
    WaitingGalaxy(SystemMap),
    WaitingSystem(GalaxyMap),
    Map { galaxy_map: GalaxyMap, system_map: SystemMap },
}

const CAMERA_LAYER: usize = 0b1000;

#[derive(Component, Reflect)]
struct MapCamera {
    sector: Sector,
    lerp_sector: Vec3,
    zoom: f32,
    yaw: f32,
    pitch: f32,
}

impl Default for MapCamera {
    fn default() -> Self {
        Self {
            sector: Sector::default(),
            lerp_sector: Vec3::ZERO,
            zoom: 2.0,
            yaw: 0.0,
            pitch: 0.0,
        }
    }
}

fn create_map_camera(mut commands: Commands) {
    commands.spawn((
        Camera {
            hdr: true,
            msaa_writeback: false, // override all other cameras
            order: 20,
            is_active: false,
            clear_color: bevy::prelude::ClearColorConfig::Custom(css::BLACK.into()),
            ..Default::default()
        },
        Transform::default(),
        Projection::from(PerspectiveProjection {
            fov: (90.0 / 180.0) * std::f32::consts::PI,
            ..Default::default()
        }),
        Camera3d::default(),
        Bloom { ..Default::default() },
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

#[derive(Component)]
struct MapSelectedSectorText;

#[derive(Component)]
struct WaypointText;

fn toggle_map(
    asset_server: Res<AssetServer>,
    q_galaxy_map_display: Query<Entity, With<GalaxyMapDisplay>>,
    input_handler: InputChecker,
    q_player: Query<&Location, With<LocalPlayer>>,
    mut commands: Commands,
    mut q_map_camera: Query<(Entity, &mut Transform), With<MapCamera>>,
    mut nevw_system_map: NettyEventWriter<RequestSystemMap>,
    mut nevw_galaxy_map: NettyEventWriter<RequestGalaxyMap>,
    q_open_menus: Query<(), With<OpenMenu>>,
) {
    if !input_handler.check_just_pressed(CosmosInputs::ToggleMap) {
        return;
    }

    let Ok((map_cam_ent, mut map_camera)) = q_map_camera.get_single_mut() else {
        return;
    };

    if let Ok(galaxy_map_entity) = q_galaxy_map_display.single() {
        commands.entity(galaxy_map_entity).insert(NeedsDespawned);
        return;
    }

    if !q_open_menus.is_empty() {
        // Don't open the map while there are other menus open
        return;
    }

    let Ok(player_loc) = q_player.single() else {
        return;
    };

    let player_system = player_loc.get_system_coordinates();

    map_camera.translation = Vec3::ZERO;

    let map_display = commands
        .spawn((
            GalaxyMapDisplay::Loading,
            OpenMenu::new(0),
            RenderLayers::from_layers(&[CAMERA_LAYER]),
            Name::new("System map display"),
            Transform::default(),
            Visibility::default(),
            ShowCursor,
        ))
        .id();

    let font = asset_server.load("fonts/PixeloidSans.ttf");
    let big_text = TextFont {
        font_size: 48.0,
        font: font.clone(),
        ..Default::default()
    };

    let small_text = TextFont {
        font_size: 24.0,
        font: font.clone(),
        ..Default::default()
    };

    commands
        .spawn((
            TargetCamera(map_cam_ent),
            DespawnWith(map_display),
            Name::new("Galaxy map UI root"),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::SpaceBetween,
                ..Default::default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                TargetCamera(map_cam_ent),
                DespawnWith(map_display),
                Name::new("Galaxy map text UI"),
                Node {
                    justify_content: JustifyContent::Center,
                    margin: UiRect::top(Val::Px(10.0)),
                    row_gap: Val::Px(12.0),
                    flex_direction: FlexDirection::Column,
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Name::new("Sector text"),
                    MapSelectedSectorText,
                    Node {
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                    Text::new(""),
                    big_text.clone(),
                ));
                p.spawn((
                    Name::new("Waypoint text"),
                    WaypointText,
                    Node {
                        align_self: AlignSelf::Center,
                        ..Default::default()
                    },
                    Text::new(""),
                    small_text.clone(),
                ));
            });

            p.spawn((
                Name::new("Controls text"),
                Node {
                    width: Val::Percent(100.0),
                    flex_direction: FlexDirection::Column,
                    margin: UiRect::all(Val::Px(30.0)),
                    ..Default::default()
                },
            ))
            .with_children(|p| {
                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    Text::new(format!(
                        "{}{}{}{}{}{} to Move",
                        input_handler
                            .get_control(CosmosInputs::MoveForward)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into()),
                        input_handler
                            .get_control(CosmosInputs::MoveLeft)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into()),
                        input_handler
                            .get_control(CosmosInputs::MoveBackward)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into()),
                        input_handler
                            .get_control(CosmosInputs::MoveRight)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into()),
                        input_handler
                            .get_control(CosmosInputs::MoveUp)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into()),
                        input_handler
                            .get_control(CosmosInputs::MoveDown)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into()),
                    )),
                    small_text.clone(),
                ));

                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    Text::new("Scroll to Zoom"),
                    small_text.clone(),
                ));

                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    Text::new(format!(
                        "{} to Reset to Your Sector",
                        input_handler
                            .get_control(CosmosInputs::ResetMapPosition)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into())
                    )),
                    small_text.clone(),
                ));

                p.spawn((
                    Node {
                        width: Val::Percent(100.0),
                        ..Default::default()
                    },
                    Text::new(format!(
                        "{} to Set/Unset Waypoint",
                        input_handler
                            .get_control(CosmosInputs::ToggleWaypoint)
                            .map(|x| x.to_string())
                            .unwrap_or("[None]".into()),
                    )),
                    small_text.clone(),
                ));
            });
        });
    nevw_system_map.write(RequestSystemMap { system: player_system });
    nevw_galaxy_map.write(RequestGalaxyMap);
}

fn update_waypoint_text(
    input_handler: InputChecker,
    q_waypoint: Query<&Location, With<Waypoint>>,
    mut q_text: Query<&mut Text, With<WaypointText>>,
) {
    let Ok(mut text) = q_text.get_single_mut() else {
        return;
    };

    let waypoint_loc = q_waypoint.single();

    let waypoint_text = waypoint_loc
        .map(|x| format!("{}, {}, {}", x.sector.x(), x.sector.y(), x.sector.z()))
        .unwrap_or(format!(
            "<{} to set>",
            input_handler
                .get_control(CosmosInputs::MoveDown)
                .map(|x| x.to_string())
                .unwrap_or("[None]".into()),
        ));

    text.as_mut().0 = format!("Waypoint: {waypoint_text}");
}

fn update_sector_text(q_cam: Query<&MapCamera, Changed<MapCamera>>, mut q_text: Query<&mut Text, With<MapSelectedSectorText>>) {
    let Ok(cam) = q_cam.single() else {
        return;
    };

    let Ok(mut text) = q_text.get_single_mut() else {
        return;
    };

    text.as_mut().0 = format!("{}, {}, {}", cam.sector.x(), cam.sector.y(), cam.sector.z());
}

const SECTOR_SCALE: f32 = 1.0;

fn position_camera(mut q_camera: Query<(&mut Transform, &mut MapCamera)>) {
    let Ok((mut trans, mut cam)) = q_camera.get_single_mut() else {
        return;
    };

    let s = cam.sector;
    let vec_sec = Vec3::new(s.x() as f32, s.y() as f32, s.z() as f32) * SECTOR_SCALE;
    cam.lerp_sector = cam.lerp_sector.lerp(vec_sec, 0.1);

    trans.rotation = Quat::from_rotation_y(cam.yaw) * Quat::from_rotation_x(-cam.pitch);
    trans.translation = cam.lerp_sector + trans.rotation * Vec3::new(0.0, 0.0, cam.zoom * SECTOR_SCALE);
}

fn handle_waypoint_sector(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q_selected_sector: Query<(&mut Visibility, &mut Transform, &MeshMaterial3d<StandardMaterial>), With<WaypointSector>>,
    q_waypoint: Query<&Location, With<Waypoint>>,
    time: Res<Time>,
    mut q_sector_text: Query<&mut Text, With<WaypointSectorText>>,
) {
    let Ok((mut text_vis, mut sector_trans, standard_material)) = q_selected_sector.get_single_mut() else {
        return;
    };

    let Ok(waypoint_loc) = q_waypoint.single() else {
        *text_vis = Visibility::Hidden;
        return;
    };

    *text_vis = Visibility::Inherited;

    let ws = waypoint_loc.sector();
    sector_trans.translation = Vec3::new(ws.x() as f32, ws.y() as f32, ws.z() as f32) * SECTOR_SCALE;

    let standard_material = materials.get_mut(standard_material).expect("Material missing");
    standard_material.base_color.set_alpha(time.elapsed_secs().sin().abs() * 0.1);

    let Ok(mut text) = q_sector_text.get_single_mut() else {
        return;
    };

    text.as_mut().0 = format!("{}, {}, {}", ws.x(), ws.y(), ws.z());
}
fn handle_selected_sector(
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut q_selected_sector: Query<(&mut Transform, &MeshMaterial3d<StandardMaterial>), (With<SelectedSector>, Without<MapCamera>)>,
    q_camera: Query<&MapCamera>,
    time: Res<Time>,
    mut q_sector_text: Query<&mut Text, With<SelectedSectorText>>,
) {
    let Ok(cam) = q_camera.single() else {
        return;
    };

    let Ok((mut sector_trans, standard_material)) = q_selected_sector.get_single_mut() else {
        return;
    };

    sector_trans.translation = cam.lerp_sector;
    let standard_material = materials.get_mut(standard_material).expect("Material missing");
    standard_material.base_color.set_alpha(time.elapsed_secs().sin().abs() * 0.1);

    let Ok(mut text) = q_sector_text.get_single_mut() else {
        return;
    };

    text.as_mut().0 = format!("{}, {}, {}", cam.sector.x(), cam.sector.y(), cam.sector.z());
}

#[derive(Component)]
struct SelectedSector;

#[derive(Component)]
struct WaypointSector;

#[derive(Component)]
struct SelectedSectorText;

#[derive(Component)]
struct WaypointSectorText;

fn render_galaxy_map(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    q_changed_map: Query<(Entity, &GalaxyMapDisplay), Changed<GalaxyMapDisplay>>,
    q_player_loc: Query<&Location, With<LocalPlayer>>,
    mut q_camera: Query<&mut MapCamera>,
    biospheres: Res<Registry<Biosphere>>,
    biosphere_color: Res<Registry<BiosphereColor>>,
    asset_server: Res<AssetServer>,
) {
    for (ent, galaxy_map_display) in q_changed_map.iter() {
        let GalaxyMapDisplay::Map { galaxy_map, system_map } = galaxy_map_display else {
            continue;
        };

        let Ok(player) = q_player_loc.single() else {
            return;
        };

        let Ok(mut cam) = q_camera.get_single_mut() else {
            return;
        };

        cam.sector = player.sector();
        cam.lerp_sector = Vec3::new(cam.sector.x() as f32, cam.sector.y() as f32, cam.sector.z() as f32) * SECTOR_SCALE;
        // let player_translation = Vec3::new(diff.x() as f32, diff.y() as f32, diff.z() as f32) * SECTOR_SCALE;
        // cam_trans.translation = player_translation + Vec3::new(1.0, 2.0, 2.0) * SECTOR_SCALE;
        // cam.relative_sector = diff + Sector::new(1, 2, 2);
        // cam_trans.look_at(player_translation, Vec3::Y);

        let font = asset_server.load("fonts/PixeloidSans.ttf");
        let _text_style = TextFont {
            font_size: 22.0,
            font: font.clone(),
            ..Default::default()
        };

        commands.entity(ent).with_children(|p| {
            p.spawn((
                Name::new("Selected Sector"),
                SelectedSector,
                RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                Mesh3d(meshes.add(Cuboid::new(SECTOR_SCALE, SECTOR_SCALE, SECTOR_SCALE))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: css::YELLOW.into(),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..Default::default()
                })),
                Transform::from_translation(cam.lerp_sector),
            ))
            .with_children(|p| {
                p.spawn((
                    RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                    SelectedSectorText,
                    Name::new("Selected text"),
                    // BillboardTextBundle {
                    //     billboard_depth: BillboardDepth(false),
                    //     transform: Transform::from_scale(Vec3::splat(0.008)),
                    //     text: Text::from_section(
                    //         format!("{}, {}, {}", cam.sector.x(), cam.sector.y(), cam.sector.z()),
                    //         text_style.clone(),
                    //     ),
                    //     ..Default::default()
                    // },
                ));
            });

            p.spawn((
                Name::new("Waypoint Sector"),
                WaypointSector,
                RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                Mesh3d(meshes.add(Cuboid::new(SECTOR_SCALE, SECTOR_SCALE, SECTOR_SCALE))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: css::WHITE.into(),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..Default::default()
                })),
                Visibility::Hidden,
            ))
            .with_children(|p| {
                p.spawn((
                    RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                    WaypointSectorText,
                    Name::new("Waypoint text"),
                    // BillboardTextBundle {
                    //     billboard_depth: BillboardDepth(false),
                    //     transform: Transform::from_scale(Vec3::splat(0.008)),
                    //     text: Text::from_section(
                    //         format!("{}, {}, {}", cam.sector.x(), cam.sector.y(), cam.sector.z()),
                    //         text_style.clone(),
                    //     ),
                    //     ..Default::default()
                    // },
                ));
            });

            for (sector, destination, sector_offset) in system_map
                .destinations()
                // stars are already covered by the galaxy map
                .filter(|(_, x)| !matches!(x, Destination::Star(_)))
                .map(|(sec, des)| (sec, des, system_map.system.negative_most_sector()))
                .chain(galaxy_map.destinations().map(|(sec, des)| (sec, des, Sector::ZERO)))
            {
                let sector = *sector + sector_offset;
                let transform = Transform::from_xyz(
                    sector.x() as f32 * SECTOR_SCALE,
                    sector.y() as f32 * SECTOR_SCALE,
                    sector.z() as f32 * SECTOR_SCALE,
                );

                let mesh = match destination {
                    Destination::Star(_) => meshes.add(Sphere::new(1.0)),
                    Destination::Planet(_) => meshes.add(Cuboid::new(0.5, 0.5, 0.5)),
                    Destination::Player(_) => meshes.add(Capsule3d::new(0.05, 0.1)),
                    Destination::Asteroid(_) => meshes.add(Cuboid::new(0.25, 0.25, 0.25)),
                    Destination::Unknown(_) => meshes.add(Sphere::new(0.1)),
                    Destination::Ship(_) => meshes.add(Cuboid::new(0.25, 0.25, 0.25)),
                    Destination::Station(_) => meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
                };

                // let size = match destination {
                //     Destination::Star(_) => 1.0,
                //     Destination::Planet(_) => 0.6,
                //     Destination::Asteroid(_) => 0.5,
                //     Destination::Station(_) => 0.4,
                //     Destination::Ship(_) => 0.3,
                //     Destination::Unknown(_) => 0.2,
                //     Destination::Player(_) => 0.1,
                // };

                let material = match destination {
                    Destination::Star(star) => materials.add(StandardMaterial::from_color(star.star.color())),
                    Destination::Planet(planet) => materials.add(StandardMaterial {
                        base_color: biosphere_color
                            .from_id(biospheres.from_numeric_id(planet.biosphere_id).unlocalized_name())
                            .map(|x| x.color())
                            .unwrap_or(css::HOT_PINK.into()),
                        unlit: true,
                        ..Default::default()
                    }),
                    Destination::Player(_) => materials.add(StandardMaterial::from_color(css::GREEN)),
                    Destination::Asteroid(_) => materials.add(StandardMaterial::from_color(css::GREY)),
                    Destination::Unknown(_) => materials.add(StandardMaterial {
                        base_color: css::WHITE.into(),
                        unlit: true,
                        ..Default::default()
                    }),
                    Destination::Ship(_) => materials.add(StandardMaterial::from_color(css::ORANGE)),
                    Destination::Station(s) => {
                        if s.shop_count > 0 && s.status != FactionRelation::Enemy {
                            materials.add(StandardMaterial {
                                base_color: css::PURPLE.into(),
                                unlit: true,
                                ..Default::default()
                            })
                        } else if s.status != FactionRelation::Enemy {
                            materials.add(StandardMaterial {
                                base_color: css::BLUE.into(),
                                unlit: true,
                                ..Default::default()
                            })
                        } else {
                            materials.add(StandardMaterial {
                                base_color: css::RED.into(),
                                unlit: true,
                                ..Default::default()
                            })
                        }
                    }
                };

                let mut ecmds = p.spawn((
                    RenderLayers::from_layers(&[CAMERA_LAYER]), // https://github.com/bevyengine/bevy/issues/12461
                    transform,
                    Mesh3d(mesh),
                    MeshMaterial3d(material),
                ));

                if matches!(destination, Destination::Star(_)) {
                    ecmds.insert(ScaleWithZoom);
                }
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
            cam.sector = cam.sector + sector_direction(trans.forward(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveBackward) {
            cam.sector = cam.sector + sector_direction(trans.back(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveLeft) {
            cam.sector = cam.sector + sector_direction(trans.left(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveRight) {
            cam.sector = cam.sector + sector_direction(trans.right(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveUp) {
            cam.sector = cam.sector + sector_direction(trans.up(), amount);
        }
        if input_handler.check_just_pressed(CosmosInputs::MoveDown) {
            cam.sector = cam.sector + sector_direction(trans.down(), amount);
        }

        if input_handler.check_just_pressed(CosmosInputs::ResetMapPosition) {
            let Ok(player_loc) = q_local_player.single() else {
                continue;
            };

            cam.sector = player_loc.sector();
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

            let amount = if input_handler.check_pressed(CosmosInputs::Sprint) {
                3.0
            } else {
                1.0
            };
            cam.zoom -= dy * amount;
        }

        cam.zoom = cam.zoom.clamp(0.05, 5000.0);
    }
}

fn handle_map_camera(mut q_map_camera: Query<&mut Camera, With<MapCamera>>, q_exists: Query<(), With<GalaxyMapDisplay>>) {
    let Ok(mut cam) = q_map_camera.get_single_mut() else {
        return;
    };

    cam.is_active = !q_exists.is_empty();
}

fn receive_map(
    mut nevr_galaxy_map: EventReader<GalaxyMapResponseEvent>,
    mut nevr_system_map: EventReader<SystemMapResponseEvent>,
    mut q_galaxy_map: Query<&mut GalaxyMapDisplay>,
) {
    for ev in nevr_galaxy_map.read() {
        let Ok(mut gmap) = q_galaxy_map.get_single_mut() else {
            return;
        };

        match gmap.as_ref() {
            GalaxyMapDisplay::WaitingGalaxy(system_map) => {
                *gmap = GalaxyMapDisplay::Map {
                    system_map: system_map.clone(),
                    galaxy_map: ev.map.clone(),
                }
            }
            GalaxyMapDisplay::Map { galaxy_map: _, system_map } => {
                *gmap = GalaxyMapDisplay::Map {
                    system_map: system_map.clone(),
                    galaxy_map: ev.map.clone(),
                }
            }
            _ => *gmap = GalaxyMapDisplay::WaitingSystem(ev.map.clone()),
        }
    }

    for ev in nevr_system_map.read() {
        let Ok(mut gmap) = q_galaxy_map.get_single_mut() else {
            return;
        };

        match gmap.as_ref() {
            GalaxyMapDisplay::WaitingSystem(galaxy_map) => {
                *gmap = GalaxyMapDisplay::Map {
                    system_map: ev.map.clone(),
                    galaxy_map: galaxy_map.clone(),
                }
            }
            GalaxyMapDisplay::Map { galaxy_map, system_map: _ } => {
                *gmap = GalaxyMapDisplay::Map {
                    system_map: ev.map.clone(),
                    galaxy_map: galaxy_map.clone(),
                }
            }
            _ => *gmap = GalaxyMapDisplay::WaitingGalaxy(ev.map.clone()),
        }
    }
}

fn map_active(q_map: Query<(), With<GalaxyMapDisplay>>) -> bool {
    !q_map.is_empty()
}

fn teleport_at(mut q_player: Query<&mut Location, With<LocalPlayer>>, inputs: InputChecker, q_camera: Query<&MapCamera>) {
    if inputs.check_just_pressed(CosmosInputs::TeleportSelected) {
        let Ok(mut loc) = q_player.get_single_mut() else {
            return;
        };
        let Ok(cam) = q_camera.single() else {
            return;
        };
        loc.sector = cam.sector;
    }
}

#[derive(Component)]
struct ScaleWithZoom;
fn scale_with_zoom(mut q_scale_with_zoom: Query<&mut Transform, With<ScaleWithZoom>>, q_camera: Query<&MapCamera>) {
    let Ok(cam) = q_camera.single() else {
        return;
    };

    for mut t in q_scale_with_zoom.iter_mut() {
        t.scale = Vec3::splat((cam.zoom / 500.0).max(1.0));
    }
}

pub(super) fn register(app: &mut App) {
    waypoint::register(app);

    app.add_systems(OnEnter(GameState::Playing), create_map_camera)
        .add_systems(
            Update,
            (
                (
                    toggle_map,
                    receive_map,
                    render_galaxy_map,
                    (
                        camera_movement,
                        position_camera,
                        handle_selected_sector,
                        handle_waypoint_sector,
                        teleport_at,
                        update_sector_text,
                        update_waypoint_text,
                        scale_with_zoom,
                    )
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
