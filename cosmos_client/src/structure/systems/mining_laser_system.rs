//! Client-side laser cannon system logic

use bevy::{
    asset::LoadState,
    pbr::{NotShadowCaster, NotShadowReceiver},
    prelude::*,
    utils::HashMap,
};
use bevy_kira_audio::prelude::*;
use bevy_rapier3d::{dynamics::PhysicsWorld, pipeline::QueryFilter, plugin::RapierContext};
use cosmos_core::{
    block::BlockFace,
    ecs::NeedsDespawned,
    structure::{
        shared::DespawnWithStructure,
        systems::{
            energy_storage_system::EnergyStorageSystem, mining_laser_system::MiningLaserSystem, StructureSystem, StructureSystems,
            SystemActive,
        },
        Structure,
    },
};

use crate::{
    asset::asset_loader::load_assets,
    audio::{AudioEmission, CosmosAudioEmitter, DespawnOnNoEmissions},
    state::game_state::GameState,
};

use super::sync::sync_system;

const BEAM_SIZE: f32 = 0.2;

/// TODO: sync from server
const BEAM_MAX_RANGE: f32 = 250.0;

#[derive(Event)]
/// This event is fired whenever a laser cannon system is fired
pub struct LaserCannonSystemFiredEvent(pub Entity);

#[derive(Resource)]
struct LaserCannonFireHandles(Vec<Handle<bevy_kira_audio::prelude::AudioSource>>);

#[derive(Resource)]
struct MiningLaserMesh(Handle<Mesh>);

#[derive(Resource, Default)]
struct MiningLaserMaterialCache(HashMap<u32, Handle<StandardMaterial>>);

fn color_hash(color: Color) -> u32 {
    let (r, g, b, a) = (
        (color.r() * 255.0) as u8,
        (color.g() * 255.0) as u8,
        (color.b() * 255.0) as u8,
        (color.a() * 255.0) as u8,
    );

    u32::from_be_bytes([r, g, b, a])
}

#[derive(Component)]
struct MiningLaser {
    /// Relative to structure
    start_loc: Vec3,
    max_length: f32,
    laser_direction: BlockFace, // which direction (relative to ship's core) the laser is going. (ex: Front is +Z)
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
            if let Some(mut beam) = commands.get_entity(*beam) {
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

    q_structure: Query<(&Structure, &PhysicsWorld)>,
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

        let idx = rand::random::<usize>() % audio_handles.0.len();

        let handle = audio_handles.0[idx].clone_weak();
        let playing_sound: Handle<AudioInstance> = audio.play(handle.clone_weak()).handle();

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
                TransformBundle::from_transform(Transform::from_xyz(0.5, 0.5, 1.0)),
            ));
        });

        let mut active_beams = Vec::with_capacity(mining_laser_system.lines.len());

        commands.entity(structure_system.structure_entity()).with_children(|p| {
            for line in &mining_laser_system.lines {
                let color = line.color.unwrap_or(Color::WHITE);

                let hashed = color_hash(color);

                if !materials_cache.0.contains_key(&hashed) {
                    materials_cache.0.insert(
                        hashed,
                        materials.add(StandardMaterial {
                            unlit: true,
                            base_color: Color::Rgba {
                                red: color.r(),
                                green: color.g(),
                                blue: color.b(),
                                alpha: 0.8,
                            },
                            emissive: color,
                            alpha_mode: AlphaMode::Add,
                            ..Default::default()
                        }),
                    );
                }

                let material = materials_cache.0.get(&hashed).expect("Added above");

                let beam_direction = line.direction.direction_vec3();

                let laser_start = structure.block_relative_position(line.start.coords());
                let beam_ent = p
                    .spawn((
                        PbrBundle {
                            transform: Transform::from_translation(laser_start).looking_to(beam_direction, Vec3::Y),
                            material: material.clone_weak(),
                            mesh: mesh.0.clone_weak(),
                            ..Default::default()
                        },
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
    q_parent: Query<&Parent>,
    mut q_lasers: Query<(&GlobalTransform, &mut Transform, &PhysicsWorld, &MiningLaser, &Parent)>,
    q_global_trans: Query<&GlobalTransform>,
    rapier_context: Res<RapierContext>,
) {
    for (g_trans, mut trans, phys_world, mining_laser, parent) in q_lasers.iter_mut() {
        let parent_structure_ent = parent.get();

        let Ok(parent_g_trans) = q_global_trans.get(parent_structure_ent) else {
            warn!("Mining laser missing parent!");
            continue;
        };

        let mut laser_start = mining_laser.start_loc;

        let parent_rot = Quat::from_affine3(&parent_g_trans.affine());

        laser_start = parent_g_trans.translation() + parent_rot.mul_vec3(laser_start);

        let toi = match rapier_context.cast_ray(
            phys_world.world_id,
            laser_start,
            g_trans.forward(),
            mining_laser.max_length,
            true,
            QueryFilter::predicate(QueryFilter::default(), &|entity| {
                if parent_structure_ent == entity {
                    false
                } else if let Ok(parent) = q_parent.get(entity) {
                    parent.get() != parent_structure_ent
                } else {
                    false
                }
            }),
        ) {
            Ok(Some((_, toi))) => toi,
            _ => mining_laser.max_length,
        };

        trans.scale.z = toi * 2.0;
        match mining_laser.laser_direction {
            BlockFace::Front => trans.translation.z = mining_laser.start_loc.z + toi / 2.0,
            BlockFace::Back => trans.translation.z = mining_laser.start_loc.z - toi / 2.0,
            BlockFace::Top => trans.translation.y = mining_laser.start_loc.y + toi / 2.0,
            BlockFace::Bottom => trans.translation.y = mining_laser.start_loc.y - toi / 2.0,
            BlockFace::Right => trans.translation.x = mining_laser.start_loc.x + toi / 2.0,
            BlockFace::Left => trans.translation.x = mining_laser.start_loc.x - toi / 2.0,
        }
    }
}

fn create_mining_laser_mesh(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let shape = meshes.add(Cuboid::new(BEAM_SIZE, BEAM_SIZE, 0.5));

    commands.insert_resource(MiningLaserMesh(shape));
}

fn rotate_mining_lasers(time: Res<Time>, mut q_transforms: Query<(&mut Transform, &MiningLaser)>) {
    for (mut trans, mining_laser) in q_transforms.iter_mut() {
        trans.rotation =
            Quat::from_axis_angle(mining_laser.laser_direction.direction_vec3(), time.delta_seconds()).mul_quat(trans.rotation);
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

    load_assets::<bevy_kira_audio::prelude::AudioSource, LaserCannonLoadingFlag>(
        app,
        GameState::PreLoading,
        vec![
            "cosmos/sounds/sfx/laser-fire-1.ogg",
            "cosmos/sounds/sfx/laser-fire-2.ogg",
            "cosmos/sounds/sfx/laser-fire-3.ogg",
        ],
        |mut commands, handles| {
            commands.insert_resource(LaserCannonFireHandles(
                handles.into_iter().filter(|x| x.1 == LoadState::Loaded).map(|x| x.0).collect(),
            ));
        },
    );

    app.configure_sets(Update, (LasersSystemSet::CreateLasers, LasersSystemSet::UpdateLasers).chain());

    app.add_event::<LaserCannonSystemFiredEvent>()
        .init_resource::<MiningLaserMaterialCache>()
        .add_systems(Startup, create_mining_laser_mesh)
        .add_systems(
            Update,
            (
                apply_mining_effects.in_set(LasersSystemSet::CreateLasers),
                (resize_mining_lasers, rotate_mining_lasers).in_set(LasersSystemSet::UpdateLasers),
                remove_dead_mining_beams.in_set(LasersSystemSet::UpdateLasers),
            )
                .chain()
                .run_if(in_state(GameState::Playing)),
        );
}
