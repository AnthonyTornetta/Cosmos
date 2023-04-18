//! Handles the creation of lasers

use bevy::prelude::*;
use bevy_rapier3d::prelude::DEFAULT_WORLD_ID;
use bevy_renet::renet::*;
use cosmos_core::{
    netty::{
        cosmos_encoder, server_laser_cannon_system_messages::ServerLaserCannonSystemMessages,
        NettyChannel,
    },
    physics::{location::Location, player_world::PlayerWorld},
    projectiles::laser::Laser,
};

use crate::{netty::mapping::NetworkMapping, state::game_state::GameState};

#[derive(Resource)]
struct LaserMesh(Handle<Mesh>);

fn create_laser_mesh(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands) {
    commands.insert_resource(LaserMesh(
        meshes.add(Mesh::from(shape::Box::new(0.1, 0.1, 1.0))),
    ));
}

fn lasers_netty(
    mut commands: Commands,
    mut client: ResMut<RenetClient>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    network_mapping: Res<NetworkMapping>,
    laser_mesh: Res<LaserMesh>,
    player_world: Query<&Location, With<PlayerWorld>>,
) {
    while let Some(message) = client.receive_message(NettyChannel::LaserCannonSystem.id()) {
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
                if let Ok(world_location) = player_world.get_single() {
                    if let Some(server_entity) = no_hit {
                        if let Some(client_entity) =
                            network_mapping.client_from_server(&server_entity)
                        {
                            no_hit = Some(client_entity);
                        }
                    }

                    Laser::spawn_custom_pbr(
                        location,
                        laser_velocity,
                        firer_velocity,
                        strength,
                        no_hit,
                        PbrBundle {
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
                        world_location,
                        &mut commands,
                    );
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(create_laser_mesh.in_schedule(OnEnter(GameState::Loading)))
        .add_system(lasers_netty.in_set(OnUpdate(GameState::Playing)));
}
