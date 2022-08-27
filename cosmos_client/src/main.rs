mod rendering;

use std::cell::RefCell;
use std::process::id;
use std::rc::Rc;
use cosmos_core::structure::chunk::CHUNK_DIMENSIONS;

use std::sync::Arc;
use std::thread::sleep;
use std::time::{Duration, SystemTime};
use bevy::prelude::*;
use bevy_rapier3d::na::Vector3;
use cosmos_core::block::block::BlockProperty::Transparent;
use cosmos_core::block::blocks::{GRASS, STONE};
use cosmos_core::structure::structure::Structure;
use crate::rendering::structure_renderer::{StructureRenderer};

struct CubeExample {
    x: u64,
    y_rot: f32,

    camera_position: Vector3<f32>,
    camera_rotation: Vector3<f32>,
}

impl Default for CubeExample {
    fn default() -> Self {
        Self {
            x: 0,
            y_rot: 0.0,
            camera_position: Vector3::new(0.0, 0.0, 0.0),
            camera_rotation: Vector3::new(0.0, 0.0, 0.0)
        }
    }
}

fn add_player(mut commands: Commands) {
    commands.spawn().insert_bundle(Camera3dBundle {
        transform: Transform::from_xyz(0.0, 30.0, -50.0).looking_at(Vec3::new(0., 1., 0.), Vec3::Y),
        ..default()
    });
}

enum Atlas {
    Main
}

struct LoadingAsset {
    atlas_type: Atlas,
    handle: Handle<Image>
}

struct AssetsLoading(Vec<LoadingAsset>);

struct MainAtlas {
    handle: Handle<Image>
}

fn setup(server: Res<AssetServer>, mut loading: ResMut<AssetsLoading>) {
    let main_atlas = server.load("images/atlas/main.png");

    loading.0.push(LoadingAsset { handle: main_atlas, atlas_type: Atlas::Main });
}

fn check_assets_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    loading: Res<AssetsLoading>,
    mut state: ResMut<State<GameStatee>>
) {
    use bevy::asset::LoadState;

    println!("RANN!!!");

    match server.get_group_load_state(loading.0.iter().map(|h| h.handle.id)) {
        LoadState::Failed => {
            panic!("Failed to load asset!!");
        }
        LoadState::Loaded => {
            // all assets are now ready

            for asset in &loading.0 {
                match asset.atlas_type {
                    Atlas::Main => {
                        commands.insert_resource(MainAtlas { handle: asset.handle.clone() });
                    }
                }
            }

            // this might be a good place to transition into your in-game state

            // remove the resource to drop the tracking handles
            commands.remove_resource::<AssetsLoading>();
            // (note: if you don't have any other handles to the assets
            // elsewhere, they will get unloaded after this)

            state.set(GameStatee::Playing).unwrap();
        }
        _ => {
            println!("Loading.");
            // NotLoaded/Loading: not fully ready yet
        }
    }
}

fn add_structure(mut commands: Commands,
                 mut meshes: ResMut<Assets<Mesh>>,
                 main_atlas: Res<MainAtlas>,
                 mut materials: ResMut<Assets<StandardMaterial>>) {

    let material_handle = materials.add(StandardMaterial {
        base_color_texture: Some(main_atlas.handle.clone()),
        alpha_mode: AlphaMode::Opaque,
        unlit: true,
        ..default()
    });

    let mut structure = Structure::new(1, 1, 1);

    let renderer = Rc::new(RefCell::new(StructureRenderer::new(1, 1, 1)));

    structure.add_structure_listener(renderer.clone());

    for z in 0..CHUNK_DIMENSIONS {
        for x in 0..CHUNK_DIMENSIONS {
            let y: f32 = CHUNK_DIMENSIONS as f32 - ((x + z) as f32 / 12.0).sin().abs() * 4.0;

            for yy in 0..y.ceil() as usize {
                if yy == y.ceil() as usize - 1 {
                    structure.set_block_at(x, yy, z, &GRASS);
                }
                else {
                    structure.set_block_at(x, yy, z, &STONE);
                }
            }
        }
    }

    renderer.borrow_mut().render(&structure);

    // rust needs this for some reason?
    let res = renderer.borrow_mut().create_meshes();

    let mut handles = Vec::new();
    for item in res {
        handles.push(meshes.add(item));
    }

    commands.spawn()
        .insert_bundle(PbrBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                ..default()
            },
            ..default()
        })
        .with_children(|parent| {
            for handle in handles {
                parent.spawn_bundle(PbrBundle {
                    mesh: handle,
                    material: material_handle.clone(),
                    transform: Transform::from_xyz(0.0, 0.0, 0.0),
                    ..default()
                });
            }
        });

    commands.spawn_bundle(PointLightBundle {
        point_light: PointLight {
            intensity: 9000.0,
            range: 1000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 50.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameStatee {
    Loading,
    Playing
}

fn main() {
    shape::Cube::new(1.0);

    App::new()
        .add_plugins(DefaultPlugins)
        .add_state(GameStatee::Loading)
        .insert_resource(AssetsLoading { 0: Vec::new() })
        .add_startup_system(setup)// add the app state type

        // add systems to run regardless of state, as usual
        // .add_system(nothing)

        // systems to run only in the main menu
        .add_system_set(
            SystemSet::on_update(GameStatee::Loading)
                .with_system(check_assets_ready)
        )

        // setup when entering the state

        .add_system_set(
            SystemSet::on_enter(GameStatee::Playing)
                .with_system(add_player)
                .with_system(add_structure)
        )

        .run();
}