//! Client-side laser cannon system logic

use bevy::{
    asset::LoadState,
    light::{NotShadowCaster, NotShadowReceiver},
    platform::collections::HashMap,
    prelude::*,
};
use bevy_kira_audio::prelude::*;
use bevy_rapier3d::{
    geometry::{CollisionGroups, Group},
    pipeline::QueryFilter,
    plugin::{RapierContextEntityLink, ReadRapierContext},
};
use cosmos_core::{
    block::block_direction::BlockDirection,
    ecs::{NeedsDespawned, compute_totally_accurate_global_transform, sets::FixedUpdateSet},
    state::GameState,
    structure::{
        Structure,
        shared::DespawnWithStructure,
        shields::SHIELD_COLLISION_GROUP,
        systems::{
            StructureSystem, StructureSystems, SystemActive, energy_storage_system::EnergyStorageSystem,
            mining_laser_system::MiningLaserSystem,
        },
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
};

use super::sync::sync_system;

const BEAM_SIZE: f32 = 0.2;

/// TODO: sync from server
const BEAM_MAX_RANGE: f32 = 250.0;

#[derive(Message)]
/// This event is fired whenever a laser cannon system is fired
pub struct LaserCannonSystemFiredMessage(pub Entity);

#[derive(Resource)]
struct LaserCannonFireHandles(Vec<Handle<bevy_kira_audio::prelude::AudioSource>>);

#[derive(Resource)]
struct MiningLaserMesh(Handle<Mesh>);

#[derive(Resource, Default)]
struct MiningLaserMaterialCache(HashMap<u32, Handle<StandardMaterial>>);

fn color_hash(color: Srgba) -> u32 {
    let (r, g, b, a) = (
        (color.red * 255.0) as u8,
        (color.green * 255.0) as u8,
        (color.blue * 255.0) as u8,
        (color.alpha * 255.0) as u8,
    );

    u32::from_be_bytes([r, g, b, a])
}

#[derive(Component)]
struct MiningLaser {
    /// Relative to structure
    start_loc: Vec3,
    max_length: f32,
    laser_direction: BlockDirection, // which direction (relative to ship's core) the laser is going.
}

#[derive(Component, Debug)]
struct ActiveBeams(Vec<Entity>);

fn remove_dead_mining_beams(
    mut commands: Commands,
    q_has_beams: Query<&ActiveBeams>,
    mut q_deactivated_systems: RemovedComponents<SystemActive>,
) {
    for deactivated_system in q_deactivated_systems.read() {
        let Ok(active_beams) = q_has_beams.get(deactivated_system) else {
            continue;
        };

        for beam in active_beams.0.iter() {
            if let Ok(mut beam) = commands.get_entity(*beam) {
                beam.insert(NeedsDespawned);
            }
        }

        commands.entity(deactivated_system).remove::<ActiveBeams>();
    }
}

fn apply_mining_effects(
    q_systems: Query<&StructureSystems>,
    q_mining_lasers: Query<(Entity, &StructureSystem, &MiningLaserSystem), Added<SystemActive>>,
    q_energy_storage_system: Query<&EnergyStorageSystem>,
    mut commands: Commands,
    audio: Res<Audio>,
    audio_handles: Res<LaserCannonFireHandles>,

    q_structure: Query<(&Structure, &RapierContextEntityLink)>,
    mut materials_cache: ResMut<MiningLaserMaterialCache>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mesh: Res<MiningLaserMesh>,
) {
    for (system_entity, structure_system, mining_laser_system) in q_mining_lasers.iter() {
        if mining_laser_system.lines.is_empty() {
            continue;
        }

        let Ok((structure, physics_world)) = q_structure.get(structure_system.structure_entity()) else {
            warn!("Mining laser system firing on entity w/out structure?");
            commands.entity(structure_system.structure_entity()).log_components();
            continue;
        };

        let structure_entity = structure_system.structure_entity();

        let Ok(systems) = q_systems.get(structure_entity) else {
            error!("Missing systems for ship {:?}", structure_entity);
            continue;
        };

        let Ok(energy_storage_system) = systems.query(&q_energy_storage_system) else {
            error!("Ship missing energy storage system! {:?}", structure_entity);
            continue;
        };

        // Not an amazing way to check if there is enough energy, will refactor once you have to wire everything up.
        if energy_storage_system.get_energy() <= f32::EPSILON {
            continue;
        }

        let idx = rand::random::<u64>() as usize % audio_handles.0.len();

        let handle = audio_handles.0[idx].clone();
        let playing_sound: Handle<AudioInstance> = audio.play(handle.clone()).handle();

        commands.entity(structure_entity).with_children(|p| {
            p.spawn((
                CosmosAudioEmitter {
                    emissions: vec![AudioEmission {
                        instance: playing_sound,
                        peak_volume: 0.3,
                        handle,
                        ..Default::default()
                    }],
                },
                DespawnOnNoEmissions,
                Transform::from_xyz(0.5, 0.5, 1.0),
            ));
        });

        let mut active_beams = Vec::with_capacity(mining_laser_system.lines.len());

        commands.entity(structure_system.structure_entity()).with_children(|p| {
            for line in &mining_laser_system.lines {
                let color = line.color.unwrap_or(Color::WHITE);

                let color = color.into();
                let hashed = color_hash(color);

                if !materials_cache.0.contains_key(&hashed) {
                    materials_cache.0.insert(
                        hashed,
                        materials.add(StandardMaterial {
                            unlit: true,
                            base_color: Srgba {
                                red: color.red,
                                green: color.green,
                                blue: color.blue,
                                alpha: 0.8,
                            }
                            .into(),
                            emissive: color.into(),
                            alpha_mode: AlphaMode::Add,
                            ..Default::default()
                        }),
                    );
                }

                let material = materials_cache.0.get(&hashed).expect("Added above");

                let beam_direction = line.direction.as_vec3();

                let laser_start = structure.block_relative_position(line.start);
                let beam_ent = p
                    .spawn((
                        Transform::from_translation(laser_start).looking_to(beam_direction, Vec3::Y),
                        MeshMaterial3d(material.clone()),
                        Mesh3d(mesh.0.clone()),
                        NotShadowCaster,
                        NotShadowReceiver,
                        MiningLaser {
                            laser_direction: line.direction,
                            start_loc: laser_start,
                            max_length: BEAM_MAX_RANGE,
                        },
                        DespawnWithStructure,
                        *physics_world,
                    ))
                    .id();

                active_beams.push(beam_ent);
            }
        });

        commands.entity(system_entity).insert(ActiveBeams(active_beams));
    }
}

fn resize_mining_lasers(
    q_parent: Query<&ChildOf>,
    mut q_lasers: Query<(&mut Transform, &RapierContextEntityLink, &MiningLaser, &ChildOf)>,
    q_global_trans: Query<&GlobalTransform>,
    rapier_context_access: ReadRapierContext,
    q_transform: Query<(&Transform, Option<&ChildOf>), Without<MiningLaser>>,
) {
    for (mut trans, phys_world, mining_laser, parent) in q_lasers.iter_mut() {
        let parent_structure_ent = parent.parent();

        let Some(parent_g_trans) = compute_totally_accurate_global_transform(parent.parent(), &q_transform) else {
            continue;
        };

        let g_trans = parent_g_trans * *trans;

        let Ok(parent_g_trans) = q_global_trans.get(parent_structure_ent) else {
            warn!("Mining laser missing parent!");
            continue;
        };

        let mut laser_start = mining_laser.start_loc;

        let parent_rot = Quat::from_affine3(&parent_g_trans.affine());

        laser_start = parent_g_trans.translation() + parent_rot.mul_vec3(laser_start);

        let toi = match rapier_context_access.get(*phys_world).cast_ray(
            laser_start,
            g_trans.forward().into(),
            mining_laser.max_length,
            true,
            QueryFilter::predicate(QueryFilter::default(), &|entity| {
                if parent_structure_ent == entity {
                    false
                } else if let Ok(parent) = q_parent.get(entity) {
                    parent.parent() != parent_structure_ent
                } else {
                    false
                }
            })
            .groups(CollisionGroups::new(
                Group::ALL & !SHIELD_COLLISION_GROUP,
                Group::ALL & !SHIELD_COLLISION_GROUP,
            )),
        ) {
            Some((_, toi)) => toi,
            _ => mining_laser.max_length,
        };

        trans.scale.z = toi * 2.0;
        match mining_laser.laser_direction {
            BlockDirection::PosX => trans.translation.x = mining_laser.start_loc.x + toi / 2.0,
            BlockDirection::NegX => trans.translation.x = mining_laser.start_loc.x - toi / 2.0,
            BlockDirection::PosY => trans.translation.y = mining_laser.start_loc.y + toi / 2.0,
            BlockDirection::NegY => trans.translation.y = mining_laser.start_loc.y - toi / 2.0,
            BlockDirection::PosZ => trans.translation.z = mining_laser.start_loc.z + toi / 2.0,
            BlockDirection::NegZ => trans.translation.z = mining_laser.start_loc.z - toi / 2.0,
        }
    }
}

fn create_mining_laser_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let shape = meshes.add(Cuboid::new(BEAM_SIZE, BEAM_SIZE, 0.5));

    commands.insert_resource(MiningLaserMesh(shape));
}

fn rotate_mining_lasers(time: Res<Time>, mut q_transforms: Query<(&mut Transform, &MiningLaser)>) {
    for (mut trans, mining_laser) in q_transforms.iter_mut() {
        trans.rotation = Quat::from_axis_angle(mining_laser.laser_direction.as_vec3(), time.delta_secs()).mul_quat(trans.rotation);
    }
}

struct LaserCannonLoadingFlag;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum LasersSystemSet {
    CreateLasers,
    UpdateLasers,
}

pub(super) fn register(app: &mut App) {
    sync_system::<MiningLaserSystem>(app);

    load_assets::<bevy_kira_audio::prelude::AudioSource, LaserCannonLoadingFlag, 3>(
        app,
        GameState::PreLoading,
        [
            "cosmos/sounds/sfx/laser-fire-1.ogg",
            "cosmos/sounds/sfx/laser-fire-2.ogg",
            "cosmos/sounds/sfx/laser-fire-3.ogg",
        ],
        |mut commands, handles| {
            commands.insert_resource(LaserCannonFireHandles(
                handles
                    .into_iter()
                    .filter(|x| matches!(x.1, LoadState::Loaded))
                    .map(|x| x.0)
                    .collect(),
            ));
        },
    );

    app.configure_sets(
        FixedUpdate,
        (LasersSystemSet::CreateLasers, LasersSystemSet::UpdateLasers)
            .after(FixedUpdateSet::LocationSyncingPostPhysics)
            .chain(),
    );

    app.add_message::<LaserCannonSystemFiredMessage>()
        .init_resource::<MiningLaserMaterialCache>()
        .add_systems(Startup, create_mining_laser_mesh)
        .add_systems(
            FixedUpdate,
            (
                apply_mining_effects.in_set(LasersSystemSet::CreateLasers),
                (resize_mining_lasers, rotate_mining_lasers).in_set(LasersSystemSet::UpdateLasers),
                remove_dead_mining_beams.in_set(LasersSystemSet::UpdateLasers),
            )
                .chain()
                .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
        );
}
