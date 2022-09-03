mod rendering;

use std::cell::RefCell;
use std::rc::Rc;
use cosmos_core::structure::chunk::CHUNK_DIMENSIONS;

use std::thread::sleep;
use std::time::{Duration, Instant, SystemTime};
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, FilterMode, TextureDimension, TextureFormat};
use bevy::render::texture::{HdrTextureLoader, ImageSettings};
use std::collections::HashMap;
use bevy::render::camera::{Projection, RenderTarget};
use bevy_rapier3d::na::Vector3;
use bevy_rapier3d::plugin::{NoUserData, RapierConfiguration, RapierPhysicsPlugin};
use bevy_rapier3d::prelude::{Collider, LockedAxes, RigidBody, Vect, Velocity};
use bevy_rapier3d::rapier::prelude::RigidBodyVelocity;
use bevy_rapier3d::render::RapierDebugRenderPlugin;
use cosmos_core::block::blocks::{DIRT, CHERRY_LEAF, STONE, CHERRY_LOG, GRASS};
use cosmos_core::entities::player::Player;
use cosmos_core::structure::structure::Structure;
use crate::rendering::structure_renderer::{StructureRenderer};
use crate::rendering::uv_mapper::UVMapper;
use cosmos_core::physics::structure_physics::StructurePhysics;
use rand::Rng;

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

#[derive(Component, Default)]
struct CameraHelper {
    pub last_x: f32,
    pub last_y: f32,
    pub ready: bool,

    pub angle_y: f32,
    pub angle_x: f32,
}

fn add_player(mut commands: Commands) {
    commands.spawn().insert_bundle(PbrBundle {
        transform: Transform::from_xyz(0.0, 60.0, 20.0),
        ..default()
    })
        .insert(Collider::capsule_y(0.5, 0.25))
        .insert(LockedAxes::ROTATION_LOCKED)
        .insert(RigidBody::Dynamic)
        .insert(Velocity::default())
        .insert(Player::new(String::from("joey")))
    .with_children(|parent| {
        parent.spawn_bundle(Camera3dBundle {
            transform: Transform::from_xyz(0.0, 0.75, 0.0),
            projection: Projection::from(PerspectiveProjection {
                fov: (90.0 / 360.0) * (std::f32::consts::PI * 2.0),
                ..default()
            }),
            ..default()
        })
            .insert(CameraHelper::default());
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

    loading.0.push(LoadingAsset {
        handle: main_atlas,
        atlas_name: AtlasName::Main
    });
}

fn check_assets_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    loading: Res<AssetsLoading>,
    mut state: ResMut<State<GameState>>,
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

            state.set(GameState::Playing).unwrap();
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
        alpha_mode: AlphaMode::Mask(0.5),
        unlit: false,
        metallic: 0.0,
        reflectance: 0.0,

        ..default()
    });

    let mut structure = Structure::new(2, 2, 2);

    let renderer = Rc::new(RefCell::new(StructureRenderer::new(&structure)));
    let physics_updater = Rc::new(RefCell::new(StructurePhysics::new()));

    structure.add_structure_listener(renderer.clone());
    structure.add_structure_listener(physics_updater.clone());

    let mut now = Instant::now();
    for z in 0..CHUNK_DIMENSIONS * structure.length() {
        for x in 0..CHUNK_DIMENSIONS * structure.width() {
            let y: f32 = (CHUNK_DIMENSIONS * structure.height()) as f32 - ((x + z) as f32 / 12.0).sin().abs() * 4.0 - 10.0;

            let y_max = y.ceil() as usize;
            for yy in 0..y_max {
                if yy == y_max - 1 {
                    structure.set_block_at(x, yy, z, &GRASS);

                    let mut rng = rand::thread_rng();

                    let n1: u8 = rng.gen();

                    if n1 < 1 {
                         for ty in (yy+1)..(yy + 7) {
                             if ty != yy + 6 {
                                 structure.set_block_at(x, ty, z, &CHERRY_LOG);
                             }
                             else {
                                 structure.set_block_at(x, ty, z, &CHERRY_LEAF);
                             }

                             if ty > yy + 2 {
                                 let range;
                                 if ty < yy + 5 {
                                     range = -2..3;
                                 }
                                 else {
                                     range = -1..2;
                                 }

                                 for tz in range.clone() {
                                     for tx in range.clone() {
                                         if tx == 0 && tz == 0 || (tx + (x as i32) < 0 || tz + (z as i32) < 0 || ((tx + (x as i32)) as usize) >= structure.width() * 32 || ((tz + (z as i32)) as usize) >= structure.length() * 32) {
                                             continue;
                                         }
                                         structure.set_block_at((x as i32 + tx) as usize, ty, (z as i32 + tz) as usize, &CHERRY_LEAF);
                                     }
                                 }
                             }
                         }
                    }
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

    println!("Done in {}ms", now.elapsed().as_millis());

    now = Instant::now();

    renderer.borrow_mut().render(&structure, &main_atlas.uv_mapper);

    println!("Made mesh data in {}ms", now.elapsed().as_millis());

    now = Instant::now();

    let renders = renderer.borrow_mut().create_meshes();

    println!("Meshes converted to bevy meshes in {}ms", now.elapsed().as_millis());

    now = Instant::now();

    let mut colliders = physics_updater.borrow_mut().create_colliders(&structure);

    println!("Phyiscs done in {}ms", now.elapsed().as_millis());

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
            intensity: 50000.0,
            range: 1000.,
            shadows_enabled: true,
            ..default()
        },
        transform: Transform::from_xyz(0.0, 50.0, 0.0).looking_at(Vec3::new(0.0, 0.0, 0.0), Vec3::Y),
        ..default()
    });
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
enum InputState {
    JustPressed,
    Pressed,
    JustReleased,
    Released
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
enum CosmosInputs {
    MoveForward,
    MoveBackward,
    MoveUpOrJump,
    SlowDown,
    MoveLeft,
    MoveRight,
    Sprint
}

struct CosmosInputHandler {
    input_mapping: HashMap<CosmosInputs, KeyCode>,
}

impl CosmosInputHandler {
    pub fn new() -> Self {
        Self {
            input_mapping: HashMap::new(),
        }
    }

    pub fn check_just_released(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>) -> bool {
        let keycode = self.keycode_for(input_code);

        keycode.is_some() &&
            inputs.just_released(keycode.unwrap())
    }

    pub fn check_released(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>) -> bool {
        !self.check_pressed(input_code, inputs)
    }

    pub fn check_just_pressed(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>) -> bool {
        let keycode = self.keycode_for(input_code);

        keycode.is_some() &&
            inputs.just_pressed(keycode.unwrap())
    }

    pub fn check_pressed(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>) -> bool {
        let keycode = self.keycode_for(input_code);

        keycode.is_some() &&
            inputs.pressed(keycode.unwrap())
    }

    pub fn set_keycode(&mut self, input: CosmosInputs, keycode: Option<KeyCode>) {
        if keycode.is_none() {
            self.input_mapping.remove(&input);
        }
        else {
            self.input_mapping.insert(input, keycode.unwrap());
        }
    }

    pub fn keycode_for(&self, input: CosmosInputs) -> Option<KeyCode> {
        if !self.input_mapping.contains_key(&input) {
            return None;
        }

        Some(self.input_mapping[&input])
    }
}

fn init_input(mut input_handler: ResMut<CosmosInputHandler>) {
    // In future load these from settings
    input_handler.set_keycode(CosmosInputs::MoveForward, Some(KeyCode::W));
    input_handler.set_keycode(CosmosInputs::MoveLeft, Some(KeyCode::A));
    input_handler.set_keycode(CosmosInputs::MoveBackward, Some(KeyCode::S));
    input_handler.set_keycode(CosmosInputs::MoveRight, Some(KeyCode::D));
    input_handler.set_keycode(CosmosInputs::SlowDown, Some(KeyCode::LShift));
    input_handler.set_keycode(CosmosInputs::MoveUpOrJump, Some(KeyCode::Space));
    input_handler.set_keycode(CosmosInputs::Sprint, Some(KeyCode::R));
}

fn process_player_camera(mut wnds: Res<Windows>,
        mut query: Query<(&mut Camera, &mut Transform, &mut CameraHelper)>)
{
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (mut camera, mut camera_transform, mut camera_helper) = query.single_mut();

    // get the window that the camera is displaying to (or the primary window)
    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get(id).unwrap()
    } else {
        wnds.get_primary().unwrap()
    };

    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = wnd.cursor_position() {

        if !camera_helper.ready {
            camera_helper.ready = true;
        }
        else {
            let dx = screen_pos.x - camera_helper.last_x;
            let dy = screen_pos.y - camera_helper.last_y;

            camera_helper.angle_x += dy * 0.005;
            camera_helper.angle_y += -dx * 0.005;

            camera_transform.rotation = Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y)
                * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);
        }

        camera_helper.last_x = screen_pos.x;
        camera_helper.last_y = screen_pos.y;
    }
}

#[inline]
fn mul_vec(v: &Vec3, s: f32) -> Vec3 {
    Vec3::new(v.x * s, v.y * s, v.z * s)
}

#[inline]
fn vec_add(a: &mut Vect, b: &Vec3) {
    a.x += b.x;
    a.y += b.y;
    a.z += b.z;
}

// The dot function is dumb
#[inline]
fn dot(vec: &Vect) -> f32{
    vec.x * vec.x + vec.y * vec.y + vec.z * vec.z
}

fn add(vec: &mut Vect, b: &Vect) {
    vec.x += b.x;
    vec.y += b.y;
    vec.z += b.z;
}

fn sub(vec: &mut Vect, b: &Vect) {
    vec.x -= b.x;
    vec.y -= b.y;
    vec.z -= b.z;
}

fn mul(vec: &mut Vect, s: f32) {
    vec.x *= s;
    vec.y *= s;
    vec.z *= s;
}

fn process_player_movement(keys: Res<Input<KeyCode>>, time: Res<Time>,
        mut input_handler: ResMut<CosmosInputHandler>,
        mut query: Query<(&mut Velocity, &mut Player)>,
        mut cam_query: Query<&Transform, With<Camera>>) {

    let (mut velocity, mut player) = query.single_mut();

    let cam_trans = cam_query.single();

    let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint, &keys) {
        false => 5.0,
        true => 20.0
    };

    let mut forward = cam_trans.forward().clone();//-Vect::new(local_z.x, 0., local_z.z);
    let mut right = cam_trans.right().clone();//Vect::new(local_z.z, 0., -local_z.x);
    let mut up = Vect::new(0.0, 1.0, 0.0);

    forward.y = 0.0;
    right.y = 0.0;

    forward = forward.normalize_or_zero() * 100.0;
    right = right.normalize_or_zero() * 100.0;

    let time = time.delta_seconds();

    if input_handler.check_pressed(CosmosInputs::MoveForward, &keys) {
        velocity.linvel += forward * time;
    }
    if input_handler.check_pressed(CosmosInputs::MoveBackward, &keys) {
        velocity.linvel -= forward * time;
    }
    if input_handler.check_just_pressed(CosmosInputs::MoveUpOrJump, &keys) {
        velocity.linvel += up * 5.0;
    }
    if input_handler.check_pressed(CosmosInputs::MoveLeft, &keys) {
        velocity.linvel -= right * time;
    }
    if input_handler.check_pressed(CosmosInputs::MoveRight, &keys) {
        velocity.linvel += right * time;
    }
    if input_handler.check_pressed(CosmosInputs::SlowDown, &keys) {
        let mut amt = velocity.linvel * 0.1;
        if amt.dot(amt) > max_speed * max_speed
        {
            amt = amt.normalize() * max_speed;
        }
        velocity.linvel -= amt;
    }

    let y = velocity.linvel.y;

    velocity.linvel.y = 0.0;

    if velocity.linvel.dot(velocity.linvel.clone()) > max_speed * max_speed {
        velocity.linvel = velocity.linvel.normalize() * max_speed;
    }

    velocity.linvel.y = y;
}

#[derive(Debug, Clone, Eq, PartialEq, Hash)]
enum GameState {
    Loading,
    Playing
}

fn main() {
    App::new()
        .insert_resource(ImageSettings::default_nearest()) // MUST be before default plugins!
        .add_plugins(DefaultPlugins)
        .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
        .insert_resource(CosmosInputHandler::new())
        // .add_plugin(RapierDebugRenderPlugin::default())
        .add_state(GameState::Loading)
        .add_startup_system(init_input)
        .insert_resource(AssetsLoading { 0: Vec::new() })
        .add_startup_system(setup)// add the app state type

        // add systems to run regardless of state, as usual
        // .add_system(nothing)

        // systems to run only in the main menu
        .add_system_set(
            SystemSet::on_update(GameState::Loading)
                .with_system(check_assets_ready)
        )

        // setup when entering the state

        .add_system_set(
            SystemSet::on_enter(GameState::Playing)
                .with_system(add_player)
                .with_system(add_structure)
        )
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(process_player_movement)
                .with_system(process_player_camera)
        )

        .run();
}