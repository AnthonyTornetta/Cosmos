use bevy::{
    asset::LoadState,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
};
use bevy_kira_audio::{Audio, AudioControl, AudioInstance};
use bevy_rapier3d::prelude::{RigidBody, Velocity};
use cosmos_core::{
    ecs::NeedsDespawned,
    netty::{client::LocalPlayer, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    prelude::{DespawnWithStructure, Structure},
    state::GameState,
    structure::systems::railgun_system::{RailgunFiredEvent, RailgunSystem},
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    rendering::MainCamera,
};

use super::sync::sync_system;

#[derive(Resource)]
struct RailgunRendering {
    mesh: Mesh3d,
}

fn create_railgun_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    commands.insert_resource(RailgunRendering {
        mesh: Mesh3d(meshes.add(Cuboid::new(0.5, 0.5, 1.0))),
    });
}

#[derive(Component, Default)]
struct RailgunBlast {
    alive_sec: f32,
}

const TTL: f32 = 1.5;

fn on_fire_railgun(
    mut commands: Commands,
    q_structure: Query<(&Location, &GlobalTransform, &Structure, &Velocity)>,
    mut nevr_railgun_fired: EventReader<RailgunFiredEvent>,
    railgun_mesh: Res<RailgunRendering>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    railgun_sound: Res<RailgunSound>,
    audio: Res<Audio>,
    q_main_camera: Query<&Transform, With<MainCamera>>,
    q_local_player: Query<(&GlobalTransform, &Location), With<LocalPlayer>>,
) {
    for ev in nevr_railgun_fired.read() {
        let Ok((s_loc, s_g_trans, structure, velocity)) = q_structure.get(ev.structure) else {
            warn!("Bad structure");
            continue;
        };

        let mut closest_sound: Option<(f32, Location, Vec3)> = None;

        let Ok((g_trans, local_loc)) = q_local_player.single() else {
            return;
        };

        let Ok(main_cam_trans) = q_main_camera.single() else {
            return;
        };

        let local_loc = *local_loc + g_trans.rotation() * main_cam_trans.translation;

        for railgun in ev.railguns.iter() {
            let relative_pos = structure.block_relative_position(railgun.origin);
            let railgun_loc = *s_loc + (s_g_trans.rotation() * relative_pos);
            let loc = railgun_loc + (railgun.direction * railgun.length / 2.0);

            let dist = local_loc.distance_sqrd(&railgun_loc);
            if closest_sound.map(|x| x.0 > dist).unwrap_or(true) {
                closest_sound = Some((dist, railgun_loc, relative_pos));
            }

            commands.spawn((
                RailgunBlast::default(),
                loc,
                Transform::from_scale(Vec3::new(1.0, 1.0, railgun.length)).looking_to(railgun.direction, Vec3::Y),
                Mesh3d(railgun_mesh.mesh.clone_weak()),
                NotShadowCaster,
                NotShadowReceiver,
                Name::new("Railgun Visual"),
                Velocity {
                    linvel: velocity.linvel,
                    ..Default::default()
                },
                RigidBody::KinematicVelocityBased,
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::WHITE,
                    emissive: Color::WHITE.into(),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..Default::default()
                })),
            ));
        }

        if let Some((dist, loc, relative_pos)) = closest_sound {
            if dist > 1000.0 {
                continue;
            }
            let playing_sound: Handle<AudioInstance> = audio.play(railgun_sound.0.clone()).with_volume(0.0).handle();

            let sound_emission = CosmosAudioEmitter::with_emissions(vec![AudioEmission {
                instance: playing_sound,
                max_distance: 1000.0,
                peak_volume: 2.0,
                ..Default::default()
            }]);

            commands.entity(ev.structure).with_children(|p| {
                p.spawn((
                    loc,
                    Transform::from_translation(relative_pos),
                    Name::new("Railgun Sound"),
                    DespawnWithStructure,
                    DespawnOnNoEmissions,
                    sound_emission,
                ));
            });
        }
    }
}

fn fade_railgun_blast(
    mut materials: ResMut<Assets<StandardMaterial>>,
    time: Res<Time>,
    mut q_railgun: Query<(Entity, &mut RailgunBlast, &MeshMaterial3d<StandardMaterial>)>,
    mut commands: Commands,
) {
    for (ent, mut rgb, mm) in q_railgun.iter_mut() {
        if rgb.alive_sec >= TTL {
            commands.entity(ent).insert(NeedsDespawned);
        }

        let Some(material) = materials.get_mut(&mm.0) else {
            continue;
        };

        material.emissive.set_alpha(1.0 - rgb.alive_sec / TTL);
        material.base_color.set_alpha(1.0 - rgb.alive_sec / TTL);

        rgb.alive_sec += time.delta_secs();
    }
}

#[derive(Resource)]
struct RailgunSound(Handle<bevy_kira_audio::prelude::AudioSource>);

pub(super) fn register(app: &mut App) {
    sync_system::<RailgunSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, RailgunSound, 1>(
        app,
        GameState::PreLoading,
        ["cosmos/sounds/sfx/railgun.ogg"],
        |mut commands, [(handle, state)]| {
            if !matches!(state, LoadState::Loaded) {
                warn!("Failed to load railgun.ogg for railgun!");
            }
            commands.insert_resource(RailgunSound(handle));
        },
    );

    app.add_systems(OnEnter(GameState::PreLoading), create_railgun_mesh);
    app.add_systems(
        Update,
        (on_fire_railgun, fade_railgun_blast).chain().run_if(in_state(GameState::Playing)),
    );
}
