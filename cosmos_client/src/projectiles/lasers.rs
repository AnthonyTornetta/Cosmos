//! Handles the creation of lasers

use bevy::prelude::*;
use bevy_rapier3d::prelude::DEFAULT_WORLD_ID;
use bevy_renet::renet::*;
use cosmos_core::{
    ecs::bundles::CosmosPbrBundle,
    netty::{cosmos_encoder, server_laser_cannon_system_messages::ServerLaserCannonSystemMessages, NettyChannelServer},
    projectiles::{laser::Laser, missile::Missile},
};

use crate::{
    netty::mapping::NetworkMapping,
    state::game_state::GameState,
    structure::systems::{laser_cannon_system::LaserCannonSystemFiredEvent, missile_launcher_system::MissileLauncherSystemFiredEvent},
};

#[derive(Resource)]
struct LaserMesh(Handle<Mesh>);

fn create_laser_mesh(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands) {
    commands.insert_resource(LaserMesh(meshes.add(Mesh::from(Cuboid::new(0.1, 0.1, 1.0)))));
}

fn lasers_netty(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    network_mapping: Res<NetworkMapping>,
    laser_mesh: Res<LaserMesh>,
    mut ev_writer_laser_cannon_fired: EventWriter<LaserCannonSystemFiredEvent>,
    mut ev_writer_missile_launcher_fired: EventWriter<MissileLauncherSystemFiredEvent>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::LaserCannonSystem) {
        let msg: ServerLaserCannonSystemMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            ServerLaserCannonSystemMessages::CreateLaser {
                color,
                location,
                laser_velocity,
                firer_velocity,
                strength,
                mut no_hit,
            } => {
                if let Some(server_entity) = no_hit {
                    if let Some(client_entity) = network_mapping.client_from_server(&server_entity) {
                        no_hit = Some(client_entity);
                    }
                }

                Laser::spawn_custom_pbr(
                    location,
                    laser_velocity,
                    firer_velocity,
                    strength,
                    no_hit,
                    CosmosPbrBundle {
                        mesh: laser_mesh.0.clone(),
                        material: materials.add(StandardMaterial {
                            base_color: color,
                            // emissive: color,
                            unlit: true,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    &time,
                    DEFAULT_WORLD_ID,
                    &mut commands,
                );
            }
            ServerLaserCannonSystemMessages::CreateMissile {
                color,
                location,
                laser_velocity,
                firer_velocity,
                strength,
                mut no_hit,
            } => {
                if let Some(server_entity) = no_hit {
                    if let Some(client_entity) = network_mapping.client_from_server(&server_entity) {
                        no_hit = Some(client_entity);
                    }
                }

                Missile::spawn_custom_pbr(
                    location,
                    laser_velocity,
                    firer_velocity,
                    strength,
                    no_hit,
                    CosmosPbrBundle {
                        mesh: laser_mesh.0.clone(),
                        material: materials.add(StandardMaterial {
                            base_color: color,
                            // emissive: color,
                            unlit: true,
                            ..Default::default()
                        }),
                        ..Default::default()
                    },
                    &time,
                    DEFAULT_WORLD_ID,
                    &mut commands,
                );
            }
            ServerLaserCannonSystemMessages::LaserCannonSystemFired { ship_entity } => {
                let Some(ship_entity) = network_mapping.client_from_server(&ship_entity) else {
                    continue;
                };

                ev_writer_laser_cannon_fired.send(LaserCannonSystemFiredEvent(ship_entity));
            }
            ServerLaserCannonSystemMessages::MissileLauncherSystemFired { ship_entity } => {
                let Some(ship_entity) = network_mapping.client_from_server(&ship_entity) else {
                    continue;
                };

                ev_writer_missile_launcher_fired.send(MissileLauncherSystemFiredEvent(ship_entity));
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), create_laser_mesh)
        .add_systems(Update, lasers_netty.run_if(in_state(GameState::Playing)));
}
