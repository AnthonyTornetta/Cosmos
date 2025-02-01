//! Handles the creation of lasers

use bevy::{prelude::*, utils::HashMap};
use bevy_rapier3d::{plugin::RapierContextEntityLink, prelude::RapierContextSimulation};
use bevy_renet2::renet2::*;
use cosmos_core::{
    netty::{
        cosmos_encoder, server_laser_cannon_system_messages::ServerStructureSystemMessages, sync::mapping::NetworkMapping,
        system_sets::NetworkingSystemsSet, NettyChannelServer,
    },
    physics::location::{CosmosBundleSet, LocationPhysicsSet},
    projectiles::{causer::Causer, laser::Laser},
    state::GameState,
};

use crate::structure::{
    shields::ShieldRender,
    systems::{laser_cannon_system::LaserCannonSystemFiredEvent, missile_launcher_system::MissileLauncherSystemFiredEvent},
};

#[derive(Resource)]
struct LaserMesh(Handle<Mesh>);

#[derive(Resource, Default)]
struct LaserMaterials(HashMap<u32, Handle<StandardMaterial>>);

fn create_laser_mesh(mut meshes: ResMut<Assets<Mesh>>, mut commands: Commands) {
    commands.init_resource::<LaserMaterials>();
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
    q_default_world: Query<Entity, With<RapierContextSimulation>>,
    mut laser_materials: ResMut<LaserMaterials>,
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
                causer,
            } => {
                if let Some(server_entity) = no_hit {
                    if let Some(client_entity) = network_mapping.client_from_server(&server_entity) {
                        no_hit = Some(client_entity);
                    }
                }

                let causer = causer.map(|c| network_mapping.client_from_server(&c.0)).and_then(|e| e.map(Causer));

                fn color_hash(color: Srgba) -> u32 {
                    let (r, g, b, a) = (
                        (color.red * 255.0) as u8,
                        (color.green * 255.0) as u8,
                        (color.blue * 255.0) as u8,
                        (color.alpha * 255.0) as u8,
                    );

                    u32::from_be_bytes([r, g, b, a])
                }
                let color = color.unwrap_or(Color::WHITE);

                let material = laser_materials.0.entry(color_hash(color.into())).or_insert_with(|| {
                    materials.add(StandardMaterial {
                        base_color: color,
                        unlit: true,
                        ..Default::default()
                    })
                });

                Laser::spawn(
                    location,
                    laser_velocity,
                    firer_velocity,
                    strength,
                    no_hit,
                    &time,
                    RapierContextEntityLink(q_default_world.single()),
                    &mut commands,
                    causer,
                )
                .insert((
                    Visibility::default(),
                    Mesh3d(laser_mesh.0.clone_weak()),
                    MeshMaterial3d(material.clone_weak()),
                ));
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
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .ambiguous_with(NetworkingSystemsSet::ReceiveMessages)
            .before(LocationPhysicsSet::DoPhysics)
            .run_if(in_state(GameState::Playing)),
    );
}
