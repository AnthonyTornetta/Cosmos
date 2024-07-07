//! Handles the creation of lasers

use bevy::prelude::*;
use bevy_rapier3d::prelude::DEFAULT_WORLD_ID;
use bevy_renet2::renet2::*;
use cosmos_core::{
    ecs::bundles::CosmosPbrBundle,
    netty::{
        cosmos_encoder, server_laser_cannon_system_messages::ServerStructureSystemMessages, sync::mapping::NetworkMapping,
        NettyChannelServer,
    },
    physics::location::CosmosBundleSet,
    projectiles::laser::Laser,
};

use crate::{
    state::game_state::GameState,
    structure::{
        shields::ShieldRender,
        systems::{laser_cannon_system::LaserCannonSystemFiredEvent, missile_launcher_system::MissileLauncherSystemFiredEvent},
    },
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
    mut q_shield_render: Query<&mut ShieldRender>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::StructureSystems) {
        let msg: ServerStructureSystemMessages = cosmos_encoder::deserialize(&message).unwrap();

        match msg {
            ServerStructureSystemMessages::CreateLaser {
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
                        mesh: laser_mesh.0.clone_weak(),
                        material: materials.add(StandardMaterial {
                            base_color: color.unwrap_or(Color::WHITE),
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
            ServerStructureSystemMessages::LaserCannonSystemFired { ship_entity } => {
                let Some(ship_entity) = network_mapping.client_from_server(&ship_entity) else {
                    continue;
                };

                ev_writer_laser_cannon_fired.send(LaserCannonSystemFiredEvent(ship_entity));
            }
            ServerStructureSystemMessages::MissileLauncherSystemFired { ship_entity } => {
                let Some(ship_entity) = network_mapping.client_from_server(&ship_entity) else {
                    continue;
                };

                ev_writer_missile_launcher_fired.send(MissileLauncherSystemFiredEvent(ship_entity));
            }
            ServerStructureSystemMessages::ShieldHit {
                shield_entity,
                relative_location,
            } => {
                let Some(shield_entity) = network_mapping.client_from_server(&shield_entity) else {
                    continue;
                };

                let Ok(mut shield_render) = q_shield_render.get_mut(shield_entity) else {
                    continue;
                };

                shield_render.add_hit_point(relative_location);
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), create_laser_mesh).add_systems(
        Update,
        lasers_netty
            .before(CosmosBundleSet::HandleCosmosBundles)
            .run_if(in_state(GameState::Playing)),
    );
}
