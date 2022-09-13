pub mod plugin;
pub mod rendering;
pub mod state;
pub mod structure;

use cosmos_core::entities::sun::Sun;
use cosmos_core::structure::chunk::Chunk;
use cosmos_core::structure::planet::planet_builder::TPlanetBuilder;
use state::game_state::{self, GameState};
use structure::chunk_retreiver;

use crate::plugin::client_plugin::ClientPluginGroup;
use crate::rendering::structure_renderer::{
    monitor_block_updates_system, monitor_needs_rendered_system, NeedsNewRenderingEvent,
};
use crate::rendering::uv_mapper::UVMapper;
use crate::structure::planet::client_planet_builder::ClientPlanetBuilder;
use bevy::prelude::*;
use bevy::render::camera::{Projection, RenderTarget};
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::render::texture::ImageSettings;
use bevy_rapier3d::na::clamp;
use bevy_rapier3d::prelude::{Collider, LockedAxes, RigidBody, Vect, Velocity};
use bevy_renet::renet::{ClientAuthentication, RenetClient};
use bevy_renet::RenetClientPlugin;
use cosmos_core::entities::player::Player;
use cosmos_core::netty::netty::ClientUnreliableMessages::PlayerBody;
use cosmos_core::netty::netty::{
    client_connection_config, NettyChannel, ServerReliableMessages, ServerUnreliableMessages,
};
use cosmos_core::netty::netty_rigidbody::NettyRigidBody;
use cosmos_core::physics::structure_physics::{
    listen_for_new_physics_event, listen_for_structure_event, NeedsNewPhysicsEvent,
};
use cosmos_core::plugin::cosmos_core_plugin::CosmosCorePluginGroup;
use cosmos_core::structure::structure::{
    BlockChangedEvent, ChunkSetEvent, Structure, StructureCreated,
};
use std::collections::HashMap;
use std::f32::consts::PI;
use std::net::UdpSocket;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Component, Default)]
struct CameraHelper {
    pub last_x: f32,
    pub last_y: f32,
    pub ready: bool,

    pub angle_y: f32,
    pub angle_x: f32,
}

enum AtlasName {
    Main,
}

struct LoadingAsset {
    atlas_name: AtlasName,
    handle: Handle<Image>,
}

struct AssetsLoading(Vec<LoadingAsset>);

pub struct MainAtlas {
    //handle: Handle<Image>,
    material: Handle<StandardMaterial>,
    uv_mapper: UVMapper,
}

fn setup(server: Res<AssetServer>, mut loading: ResMut<AssetsLoading>) {
    let main_atlas = server.load("images/atlas/main.png");

    loading.0.push(LoadingAsset {
        handle: main_atlas,
        atlas_name: AtlasName::Main,
    });
}

fn check_assets_ready(
    mut commands: Commands,
    server: Res<AssetServer>,
    loading: Res<AssetsLoading>,
    mut state: ResMut<State<GameState>>,
    mut images: ResMut<Assets<Image>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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
                            let mut n = match y % IMAGE_DIMENSIONS as usize == 0
                                || (y + 1) % IMAGE_DIMENSIONS as usize == 0
                            {
                                true => 1 + PADDING,
                                false => 1,
                            };

                            while n > 0 {
                                let og_i = i;

                                for x in 0..img_size.x as usize {
                                    if x % IMAGE_DIMENSIONS as usize == 0
                                        || (x + 1) % IMAGE_DIMENSIONS as usize == 0
                                    {
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

                        let img = Image::new(
                            Extent3d {
                                height,
                                width,
                                depth_or_array_layers: 1,
                            },
                            TextureDimension::D2,
                            data,
                            TextureFormat::Rgba8UnormSrgb,
                        );

                        let handle = images.set(asset.handle.clone(), img);

                        let material_handle = materials.add(StandardMaterial {
                            base_color_texture: Some(handle.clone()),
                            alpha_mode: AlphaMode::Mask(0.5),
                            unlit: false,
                            metallic: 0.0,
                            reflectance: 0.0,

                            ..default()
                        });

                        commands.insert_resource(MainAtlas {
                            //handle,
                            material: material_handle,
                            uv_mapper: UVMapper::new(
                                width as usize,
                                height as usize,
                                IMAGE_DIMENSIONS as usize,
                                IMAGE_DIMENSIONS as usize,
                                PADDING as usize,
                                PADDING as usize,
                            ),
                        });
                    }
                }
            }

            // this might be a good place to transition into your in-game state

            // remove the resource to drop the tracking handles

            commands.remove_resource::<AssetsLoading>();
            // (note: if you don't have any other handles to the assets
            // elsewhere, they will get unloaded after this)

            state.set(GameState::Connecting).unwrap();
        }
        _ => {
            // NotLoaded/Loading: not fully ready yet
        }
    }
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
enum InputState {
    JustPressed,
    Pressed,
    JustReleased,
    Released,
}

#[derive(Clone, Copy, Eq, PartialEq, Hash)]
enum CosmosInputs {
    MoveForward,
    MoveBackward,
    MoveUpOrJump,
    SlowDown,
    MoveLeft,
    MoveRight,
    Sprint,
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

        keycode.is_some() && inputs.just_released(keycode.unwrap())
    }

    pub fn check_released(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>) -> bool {
        !self.check_pressed(input_code, inputs)
    }

    pub fn check_just_pressed(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>) -> bool {
        let keycode = self.keycode_for(input_code);

        keycode.is_some() && inputs.just_pressed(keycode.unwrap())
    }

    pub fn check_pressed(&self, input_code: CosmosInputs, inputs: &Input<KeyCode>) -> bool {
        let keycode = self.keycode_for(input_code);

        keycode.is_some() && inputs.pressed(keycode.unwrap())
    }

    pub fn set_keycode(&mut self, input: CosmosInputs, keycode: Option<KeyCode>) {
        if keycode.is_none() {
            self.input_mapping.remove(&input);
        } else {
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

fn process_player_camera(
    mut wnds: ResMut<Windows>,
    mut query: Query<(&Camera, &mut Transform, &mut CameraHelper)>,
) {
    // get the camera info and transform
    // assuming there is exactly one main camera entity, so query::single() is OK
    let (camera, mut camera_transform, mut camera_helper) = query.single_mut();

    // get the window that the camera is displaying to (or the primary window)
    let wnd = if let RenderTarget::Window(id) = camera.target {
        wnds.get_mut(id).unwrap()
    } else {
        wnds.get_primary_mut().unwrap()
    };

    // check if the cursor is inside the window and get its position
    if let Some(screen_pos) = wnd.cursor_position() {
        if !camera_helper.ready {
            camera_helper.ready = true;
        } else {
            let dx = screen_pos.x - camera_helper.last_x;
            let dy = screen_pos.y - camera_helper.last_y;

            camera_helper.angle_x += dy * 0.005;
            camera_helper.angle_y += -dx * 0.005;

            camera_helper.angle_x = clamp(camera_helper.angle_x, -PI / 2.0, PI / 2.0);

            camera_transform.rotation = Quat::from_axis_angle(Vec3::Y, camera_helper.angle_y)
                * Quat::from_axis_angle(Vec3::X, camera_helper.angle_x);
        }

        let pos = Vec2::new(wnd.width() / 2.0, wnd.height() / 2.0);

        camera_helper.last_x = pos.x;
        camera_helper.last_y = pos.y;

        wnd.set_cursor_position(pos);
    }
}

fn process_player_movement(
    keys: Res<Input<KeyCode>>,
    time: Res<Time>,
    input_handler: ResMut<CosmosInputHandler>,
    mut query: Query<&mut Velocity, With<LocalPlayer>>,
    cam_query: Query<&Transform, With<Camera>>,
) {
    let mut velocity = query.single_mut();

    let cam_trans = cam_query.single();

    let max_speed: f32 = match input_handler.check_pressed(CosmosInputs::Sprint, &keys) {
        false => 5.0,
        true => 20.0,
    };

    let mut forward = cam_trans.forward().clone();
    let mut right = cam_trans.right().clone();
    let up = Vect::new(0.0, 1.0, 0.0);

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
        if amt.dot(amt) > max_speed * max_speed {
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

fn new_renet_client() -> RenetClient {
    let port: u16 = 1337;

    let server_addr = format!("127.0.0.1:{}", port).parse().unwrap();
    let socket = UdpSocket::bind("127.0.0.1:0").unwrap();

    let connection_config = client_connection_config();
    let cur_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    let client_id = cur_time.as_millis() as u64;

    let auth = ClientAuthentication::Unsecure {
        client_id,
        protocol_id: cosmos_core::netty::netty::PROTOCOL_ID,
        server_addr,
        user_data: None,
    };

    RenetClient::new(cur_time, socket, client_id, connection_config, auth).unwrap()
}

#[derive(Default)]
struct NetworkMapping {
    server_to_client: HashMap<Entity, Entity>,
    client_to_server: HashMap<Entity, Entity>,
}

impl NetworkMapping {
    pub fn add_mapping(&mut self, client_entity: &Entity, server_entity: &Entity) {
        self.server_to_client
            .insert(server_entity.clone(), client_entity.clone());
        self.client_to_server
            .insert(client_entity.clone(), server_entity.clone());
    }

    pub fn client_from_server(&self, server_entity: &Entity) -> Option<&Entity> {
        self.server_to_client.get(server_entity)
    }

    pub fn server_from_client(&self, client_entity: &Entity) -> Option<&Entity> {
        self.client_to_server.get(client_entity)
    }

    pub fn remove_mapping_from_server_entity(&mut self, server_entity: &Entity) {
        let client_ent = self.server_to_client.remove(server_entity);
        if client_ent.is_some() {
            self.client_to_server.remove(&client_ent.unwrap());
        }
    }
}

#[derive(Debug)]
struct PlayerInfo {
    client_entity: Entity,
    server_entity: Entity,
}

#[derive(Debug, Default)]
struct ClientLobby {
    players: HashMap<u64, PlayerInfo>,
}

#[derive(Debug)]
struct MostRecentTick(Option<u32>);

#[derive(Component, Default)]
struct LocalPlayer;

fn send_position(
    mut client: ResMut<RenetClient>,
    query: Query<(&Velocity, &Transform), (With<Player>, With<LocalPlayer>)>,
) {
    if let Ok((velocity, transform)) = query.get_single() {
        let msg = PlayerBody {
            body: NettyRigidBody::new(&velocity, &transform),
        };

        let serialized_message = bincode::serialize(&msg).unwrap();

        client.send_message(NettyChannel::Unreliable.id(), serialized_message);
    }
}

fn client_sync_players(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut client: ResMut<RenetClient>,
    mut lobby: ResMut<ClientLobby>,
    mut network_mapping: ResMut<NetworkMapping>,
    mut set_chunk_event_writer: EventWriter<ChunkSetEvent>,
    query_player: Query<&Player>,
    mut query_body: Query<(&mut Transform, &mut Velocity, Option<&LocalPlayer>)>,
    mut query_structure: Query<&mut Structure>,
) {
    let client_id = client.client_id();

    while let Some(message) = client.receive_message(NettyChannel::Unreliable.id()) {
        let msg: ServerUnreliableMessages = bincode::deserialize(&message).unwrap();

        match msg {
            ServerUnreliableMessages::PlayerBody { id, body } => {
                let entity = lobby.players.get(&id).unwrap().client_entity.clone();

                let (mut transform, mut velocity, _) = query_body.get_mut(entity).unwrap();

                transform.translation = body.translation.into();
                transform.rotation = body.rotation.into();

                velocity.linvel = body.body_vel.linvel.into();
                velocity.angvel = body.body_vel.angvel.into();
            }
            ServerUnreliableMessages::BulkBodies { bodies, time_stamp } => {
                for (server_entity, body) in bodies.iter() {
                    let maybe_exists = network_mapping.client_from_server(&server_entity);
                    if maybe_exists.is_some() {
                        let entity = maybe_exists.unwrap();

                        let (mut transform, mut velocity, local) =
                            query_body.get_mut(*entity).unwrap();

                        if local.is_none() {
                            transform.translation = body.translation.into();
                            transform.rotation = body.rotation.into();

                            velocity.linvel = body.body_vel.linvel.into();
                            velocity.angvel = body.body_vel.angvel.into();
                        }
                    }
                }
            }
        }
    }

    while let Some(message) = client.receive_message(NettyChannel::Reliable.id()) {
        let msg: ServerReliableMessages = bincode::deserialize(&message).unwrap();

        match msg {
            ServerReliableMessages::PlayerCreate {
                body,
                id,
                entity,
                name,
            } => {
                println!("Player {} ({}) connected!", name.as_str(), id);

                let mut client_entity = commands.spawn();

                client_entity
                    .insert_bundle(PbrBundle {
                        transform: body.create_transform(),
                        mesh: meshes.add(shape::Capsule::default().into()),
                        ..default()
                    })
                    .insert(Collider::capsule_y(0.5, 0.25))
                    .insert(LockedAxes::ROTATION_LOCKED)
                    .insert(RigidBody::Dynamic)
                    .insert(body.create_velocity())
                    .insert(Player::new(name, id));

                if client_id == id {
                    client_entity
                        .insert(LocalPlayer::default())
                        .with_children(|parent| {
                            parent
                                .spawn_bundle(Camera3dBundle {
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

                let player_info = PlayerInfo {
                    server_entity: entity,
                    client_entity: client_entity.id(),
                };

                lobby.players.insert(id, player_info);
                network_mapping.add_mapping(&client_entity.id(), &entity);
            }
            ServerReliableMessages::PlayerRemove { id } => {
                if let Some(PlayerInfo {
                    client_entity,
                    server_entity,
                }) = lobby.players.remove(&id)
                {
                    let mut entity = commands.entity(client_entity);

                    let name = query_player.get(client_entity).unwrap().name.clone();
                    entity.despawn();
                    network_mapping.remove_mapping_from_server_entity(&server_entity);

                    println!("Player {} ({}) disconnected", name, id);
                }
            }
            ServerReliableMessages::StructureCreate {
                entity: server_entity,
                length,
                height,
                width,
                body,
            } => {
                println!("Got Structure!");

                let mut entity = commands.spawn();
                let mut structure = Structure::new(width, height, length, entity.id());

                let builder = ClientPlanetBuilder::default();
                builder.insert_planet(&mut entity, body.create_transform(), &mut structure);

                entity.insert(structure);

                network_mapping.add_mapping(&entity.id(), &server_entity);

                // create_structure_writer.send(StructureCreated {
                //     entity: entity.id()
                // });
            }
            ServerReliableMessages::ChunkData {
                structure_entity: server_structure_entity,
                serialized_chunk,
            } => {
                let s_entity = network_mapping
                    .client_from_server(&server_structure_entity)
                    .expect("Got chunk data for structure that doesn't exist on client");

                let mut structure = query_structure.get_mut(s_entity.clone()).unwrap();

                let chunk: Chunk = bincode::deserialize(&serialized_chunk).unwrap();

                let (x, y, z) = (
                    chunk.structure_x(),
                    chunk.structure_y(),
                    chunk.structure_z(),
                );

                structure.set_chunk(chunk);

                println!("Got chunk!");

                set_chunk_event_writer.send(ChunkSetEvent {
                    x,
                    y,
                    z,
                    structure_entity: s_entity.clone(),
                });
            }
            ServerReliableMessages::StructureRemove {
                entity: server_entity,
            } => {
                commands
                    .entity(
                        network_mapping
                            .client_from_server(&server_entity)
                            .unwrap()
                            .clone(),
                    )
                    .despawn_recursive();
            }
            ServerReliableMessages::MOTD { motd } => {
                println!("Server MOTD: {}", motd);
            }
        }
    }
}

fn establish_connection(mut commands: Commands) {
    println!("Establishing connection w/ server...");
    commands.insert_resource(ClientLobby::default());
    commands.insert_resource(MostRecentTick(None));
    commands.insert_resource(new_renet_client());
    commands.insert_resource(NetworkMapping::default());
}

fn wait_for_connection(mut state: ResMut<State<GameState>>, client: Res<RenetClient>) {
    println!("Waiting...");
    if client.is_connected() {
        println!("Loading server data...");
        state.set(GameState::LoadingWorld).unwrap();
    }
}

fn wait_for_done_loading(
    mut state: ResMut<State<GameState>>,
    query: Query<&Player, With<LocalPlayer>>,
) {
    if query.get_single().is_ok() {
        println!("Got local player, starting game!");
        state
            .set(GameState::Playing)
            .expect("Unable to change state into playing");
    }
}

fn setup_window(mut windows: ResMut<Windows>) {
    let window = windows.primary_mut();
    window.set_title("Cosmos".into());
    window.set_cursor_lock_mode(true);
    window.set_cursor_visibility(false);
}

fn create_sun(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    commands
        .spawn_bundle(PointLightBundle {
            // transform: Transform::from_xyz(5.0, 8.0, 2.0),
            transform: Transform::from_xyz(0.0, 24.0, 0.0),
            point_light: PointLight {
                intensity: 1600.0, // lumens - roughly a 100W non-halogen incandescent bulb
                range: 16.0,
                color: Color::WHITE,
                shadows_enabled: true,
                ..default()
            },
            ..default()
        })
        .with_children(|builder| {
            builder.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::UVSphere {
                    radius: 0.1,
                    ..default()
                })),
                material: materials.add(StandardMaterial {
                    base_color: Color::RED,
                    emissive: Color::rgba_linear(100.0, 0.0, 0.0, 0.0),
                    ..default()
                }),
                ..default()
            });
        });

    commands.spawn_bundle(PbrBundle {
        transform: Transform::from_xyz(0.0, 60.0, 0.0),
        mesh: meshes.add(Mesh::from(shape::UVSphere {
            radius: 1.0,
            ..default()
        })),
        material: materials.add(StandardMaterial {
            base_color: Color::BLUE,
            // emissive: Color::rgba_linear(0.0, 0.0, 100.0, 0.0),
            ..default()
        }),
        ..default()
    });
}

fn main() {
    let mut app = App::new();

    game_state::register(&mut app);

    app.insert_resource(ImageSettings::default_nearest()) // MUST be before default plugins!
        .add_plugins(CosmosCorePluginGroup::default())
        .add_plugins(ClientPluginGroup::default())
        .add_plugin(RenetClientPlugin {})
        .insert_resource(CosmosInputHandler::new())
        .add_event::<BlockChangedEvent>()
        .add_event::<NeedsNewPhysicsEvent>()
        .add_event::<NeedsNewRenderingEvent>()
        .add_event::<StructureCreated>()
        .add_event::<ChunkSetEvent>()
        .add_startup_system(init_input)
        .add_startup_system(setup_window)
        .insert_resource(AssetsLoading { 0: Vec::new() })
        .add_startup_system(setup) // add the app state type
        // systems to run only in the main menu
        .add_system_set(SystemSet::on_update(GameState::Loading).with_system(check_assets_ready))
        .add_system_set(
            SystemSet::on_enter(GameState::Connecting).with_system(establish_connection),
        )
        .add_system_set(
            SystemSet::on_update(GameState::Connecting).with_system(wait_for_connection),
        )
        .add_system_set(SystemSet::on_enter(GameState::LoadingWorld).with_system(create_sun))
        .add_system_set(
            SystemSet::on_update(GameState::LoadingWorld)
                .with_system(client_sync_players)
                .with_system(wait_for_done_loading)
                .with_system(monitor_needs_rendered_system)
                .with_system(monitor_block_updates_system)
                .with_system(listen_for_structure_event)
                .with_system(listen_for_new_physics_event),
        )
        .add_system_set(
            SystemSet::on_update(GameState::Playing)
                .with_system(process_player_movement)
                .with_system(process_player_camera)
                .with_system(send_position)
                .with_system(client_sync_players)
                .with_system(monitor_needs_rendered_system)
                .with_system(monitor_block_updates_system)
                .with_system(listen_for_structure_event)
                .with_system(listen_for_new_physics_event),
        );

    chunk_retreiver::register(&mut app);

    app.run();
}
