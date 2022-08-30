mod rendering;

use std::cell::RefCell;
use std::rc::Rc;
use cosmos_core::structure::chunk::CHUNK_DIMENSIONS;

use std::thread::sleep;
use std::time::{Duration, SystemTime};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, FilterMode, TextureDimension, TextureFormat};
use bevy::render::texture::{HdrTextureLoader, ImageSettings};
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::plugin::{NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use bevy_rapier3d::prelude::{Collider, LockedAxes, RigidBody, Vect};
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use cosmos_core::block::blocks::{DIRT, GRASS, STONE};
use cosmos_core::structure::structure::Structure;
use crate::rendering::structure_renderer::{StructureRenderer};
use crate::rendering::uv_mapper::UVMapper;
use cosmos_core::physics::structure_physics::StructurePhysics;

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

fn init_physics(mut phys: ResMut<RapierConfiguration>) {
    phys.gravity = Vect::new(0.0, -1.0, 0.0);
}

fn add_player(mut commands: Commands) {
    commands.spawn().insert_bundle(PbrBundle {
        transform: Transform::from_xyz(0.0, 60.0, 20.0),
        ..default()
    })
        .insert(Collider::capsule_y(0.5, 0.25))
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(RigidBody::Dynamic)
    .with_children(|parent| {
        parent.spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.75, 0.0),
            ..default()
        });
    });
}

enum AtlasName {
    Main
}

struct LoadingAsset {
    atlas_name: AtlasName,
    handle: Handle<Image>
}

struct AssetsLoading(Vec<LoadingAsset>);

struct MainAtlas {
    handle: Handle<Image>,
    uv_mapper: UVMapper
}

fn setup(server: Res<AssetServer>, mut loading: ResMut<AssetsLoading>) {
    let main_atlas = server.load("images/atlas/main.png");

    loading.0.push(LoadingAsset { handle: main_atlas, atlas_name: AtlasName::Main });
}

fn check_assets_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    loading: Res<AssetsLoading>,
    mut state: ResMut<State<GameStatee>>,
    mut images: ResMut<Assets<Image>>
) {
    use bevy::asset::LoadState;

    match server.get_group_load_state(loading.0.iter().map(|h| h.handle.id)) {
        LoadState::Failed => {
            panic!("Failed to load asset!!");
        }
        LoadState::Loaded => {
            // all assets are now ready

            for asset in &loading.0 {
                match asset.atlas_name {
                    AtlasName::Main => {

                        const PADDING: u32 = 2;
                        const IMAGE_DIMENSIONS: u32 = 16;

                        let image = images.get(&asset.handle).unwrap();

                        let img_size = image.size();

                        let mut data: Vec<u8> = Vec::new();

                        let mut i = 0;

                        for y in 0..img_size.y as usize {
                            let mut n = match y % IMAGE_DIMENSIONS as usize == 0 || (y + 1) % IMAGE_DIMENSIONS as usize == 0 {
                                true => 1 + PADDING,
                                false => 1
                            };

                            while n > 0 {
                                let og_i = i;

                                for x in 0..img_size.x as usize {
                                    if x % IMAGE_DIMENSIONS as usize == 0 || (x + 1) % IMAGE_DIMENSIONS as usize == 0 {
                                        for _ in 0..(PADDING + 1) {
                                            data.push(image.data[i]);
                                            data.push(image.data[i + 1]);
                                            data.push(image.data[i + 2]);
                                            data.push(image.data[i + 3]);
                                        }
                                    } else {
                                        data.push(image.data[i]);
                                        data.push(image.data[i + 1]);
                                        data.push(image.data[i + 2]);
                                        data.push(image.data[i + 3]);
                                    }

                                    i += 4;
                                }

                                n -= 1;

                                if n != 0 {
                                    i = og_i;
                                }
                            }
                        }

                        let y = img_size.y as u32;
                        let x = img_size.x as u32;

                        let height = (y / IMAGE_DIMENSIONS) * (IMAGE_DIMENSIONS + PADDING * 2);
                        let width = (x / IMAGE_DIMENSIONS) * (IMAGE_DIMENSIONS + PADDING * 2);

                        // debug save
                        // image::save_buffer(&Path::new("image.png"), data.as_slice(), width, height, image::ColorType::Rgba8);

                        let img = Image::new(Extent3d {
                            height,
                            width,
                            depth_or_array_layers: 1,
                        }, TextureDimension::D2, data,
                        TextureFormat::Rgba8UnormSrgb);

                        let handle = images.set(asset.handle.clone(), img);

                        commands.insert_resource(MainAtlas {
                            handle,
                            uv_mapper: UVMapper::new(width as usize, height as usize,
                                                     IMAGE_DIMENSIONS as usize, IMAGE_DIMENSIONS as usize,
                                                     PADDING as usize, PADDING as usize)
                        });
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
        unlit: false,
        ..default()
    });

    let mut structure = Structure::new(2, 2, 2);

    let renderer = Rc::new(RefCell::new(StructureRenderer::new(&structure)));
    let physics_updater = Rc::new(RefCell::new(StructurePhysics::new()));

    structure.add_structure_listener(renderer.clone());
    structure.add_structure_listener(physics_updater.clone());

    for z in 0..CHUNK_DIMENSIONS * structure.length() {
        for x in 0..CHUNK_DIMENSIONS * structure.width() {
            let y: f32 = (CHUNK_DIMENSIONS * structure.height()) as f32 - ((x + z) as f32 / 12.0).sin().abs() * 4.0;

            let y_max = y.ceil() as usize;
            for yy in 0..y_max {
                if yy == y_max - 1 {
                    structure.set_block_at(x, yy, z, &GRASS);
                }
                else if yy > y_max - 5 {
                    structure.set_block_at(x, yy, z, &DIRT);
                }
                else {
                    structure.set_block_at(x, yy, z, &STONE);
                }
            }
        }
    }

    renderer.borrow_mut().render(&structure, &main_atlas.uv_mapper);

    let renders = renderer.borrow_mut().create_meshes();
    let mut colliders = physics_updater.borrow_mut().create_colliders(&structure);

    commands.spawn()
        .insert_bundle(PbrBundle {
            transform: Transform {
                translation: Vec3::new(0.0, 0.0, 0.0),
                ..default()
            },
            ..default()
        })
        .insert(RigidBody::Fixed)
        .with_children(|parent| {
            for item in renders {
                let coords = Vector3::new(item.x, item.y, item.z);

                let rel_pos = structure.chunk_relative_position(item.x, item.y, item.z);

                let mut child = parent.spawn_bundle(PbrBundle {
                    mesh: meshes.add(item.mesh),
                    material: material_handle.clone(),
                    transform: Transform::from_xyz(rel_pos.x, rel_pos.y, rel_pos.z),
                    ..default()
                });

                for i in 0..colliders.len() {
                    if colliders[i].chunk_coords == coords {
                        child.insert(colliders.swap_remove(i).collider);
                        break;
                    }
                }
            }
        });
    //
    // commands.spawn_bundle(PbrBundle {
    //     mesh: meshes.add(Cube::new(40.0).into()),
    //     transform: Transform::from_xyz(0.0, 0.0, 0.0),
    //     material: material_handle.clone(),
    //     ..default()
    // });

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
    App::new()
        .insert_resource(ImageSettings::default_nearest()) // MUST be before default plugins!
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        // .add_plugin(RapierDebugRenderPlugin::default())
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