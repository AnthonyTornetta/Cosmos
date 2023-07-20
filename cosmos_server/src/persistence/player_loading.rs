//! Loads/unloads entities that are close to/far away from players

use std::{ffi::OsStr, fs, time::Duration};

use bevy::{
    prelude::{warn, App, Commands, Component, DespawnRecursiveExt, Entity, Query, Res, ResMut, Update, With, Without},
    tasks::{AsyncComputeTaskPool, Task},
    time::common_conditions::on_timer,
};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    persistence::{LoadingDistance, LOAD_DISTANCE},
    physics::location::{Location, Sector, SectorUnit, SECTOR_DIMENSIONS},
};
use futures_lite::future;
use walkdir::WalkDir;

use super::{loading::NeedsLoaded, saving::NeedsSaved, EntityId, SaveFileIdentifier, SectorsCache};

fn unload_far(
    query: Query<&Location, With<Player>>,
    others: Query<(&Location, Entity, &LoadingDistance), (Without<Player>, Without<NeedsDespawned>)>,
    mut commands: Commands,
) {
    for (loc, ent, ul_distance) in others.iter() {
        let ul_distance = ul_distance.unload_block_distance();

        if let Some(min_dist) = query.iter().map(|l| l.relative_coords_to(loc).abs().max_element()).reduce(f32::min) {
            if min_dist <= ul_distance {
                continue;
            }
        }

        println!("Flagged for saving + unloading!");

        commands.entity(ent).insert((NeedsSaved, NeedsDespawned));
    }
}

const SEARCH_RANGE: SectorUnit = 25;
const DEFAULT_LOAD_DISTANCE: u32 = (LOAD_DISTANCE / SECTOR_DIMENSIONS) as u32;

#[derive(Component, Debug)]
struct LoadingTask(Task<(SectorsCache, Vec<SaveFileIdentifier>)>);

fn monitor_loading_task(
    // Because entities can be added while the scan task is in progress,
    // we need to re-check all the loaded entities before actually spawning them.
    loaded_entities: Query<&EntityId>,
    mut query: Query<(Entity, &mut LoadingTask)>,
    mut commands: Commands,
    mut sectors_cache: ResMut<SectorsCache>,
) {
    let Ok((entity, mut task)) = query.get_single_mut() else {
        return;
    };

    if let Some((cache, save_file_ids)) = future::block_on(future::poll_once(&mut task.0)) {
        commands.entity(entity).despawn_recursive();

        for sfi in save_file_ids {
            if !loaded_entities.iter().any(|x| {
                x == sfi
                    .entity_id()
                    .expect("A non-base SaveFileIdentifier was attempted to be loaded in load_near")
            }) {
                commands.spawn((sfi, NeedsLoaded));
            }
        }

        *sectors_cache = cache;
    }
}

/// Performance hot spot
fn load_near(
    query: Query<&Location, With<Player>>,
    loaded_entities: Query<&EntityId>,
    sectors_cache: Res<SectorsCache>,
    mut commands: Commands,

    already_exists: Query<(), With<LoadingTask>>,
) {
    if !already_exists.is_empty() {
        // If one is already loading, no need to spawn another task.
        return;
    }

    let thread_pool = AsyncComputeTaskPool::get();

    let sectors = query.iter().map(|l| l.sector()).collect::<Vec<Sector>>();

    // If this ever gets laggy, either of these two clones could be the cause
    let mut sectors_cache = sectors_cache.clone();
    let loaded_entities = loaded_entities.iter().cloned().collect::<Vec<EntityId>>();

    let task = thread_pool.spawn(async move {
        let mut to_load = vec![];

        for sector in sectors {
            for dz in -SEARCH_RANGE..=SEARCH_RANGE {
                for dy in -SEARCH_RANGE..=SEARCH_RANGE {
                    for dx in -SEARCH_RANGE..SEARCH_RANGE {
                        let sector = Sector::new(dx + sector.x(), dy + sector.y(), dz + sector.z());
                        let max_delta = dz.abs().max(dy.abs()).max(dx.abs()) as u32;

                        if let Some(entities) = sectors_cache.get(&sector) {
                            for (entity_id, load_distance) in entities.iter() {
                                if max_delta <= load_distance.unwrap_or(DEFAULT_LOAD_DISTANCE)
                                    && !loaded_entities.iter().any(|x| x == entity_id)
                                {
                                    to_load.push(SaveFileIdentifier::new(Some(sector), entity_id.clone(), *load_distance));
                                }
                            }
                        } else {
                            let dir = format!("world/{}_{}_{}", sector.x(), sector.y(), sector.z());

                            if fs::try_exists(&dir).unwrap_or(false) {
                                for file in WalkDir::new(&dir)
                                    .max_depth(1)
                                    .into_iter()
                                    .flatten()
                                    .filter(|x| x.file_type().is_file())
                                {
                                    let path = file.path();

                                    if path.extension() == Some(OsStr::new("cent")) {
                                        let mut entity_information = path
                                            .file_stem()
                                            .expect("Failed to get file stem")
                                            .to_str()
                                            .expect("Failed to convert to entity id")
                                            .split('_');

                                        let mut entity_id = entity_information.next().unwrap();
                                        let mut load_distance = None;

                                        if let Some(other_info) = entity_information.next() {
                                            if let Ok(ld) = entity_id.parse::<u32>() {
                                                load_distance = Some(ld);
                                                entity_id = other_info;
                                            } else {
                                                warn!("Invalid load distance: {other_info}");
                                            }
                                        }

                                        let entity_id = EntityId::new(entity_id);

                                        sectors_cache.insert(sector, entity_id.clone(), load_distance);

                                        if max_delta <= load_distance.unwrap_or(DEFAULT_LOAD_DISTANCE)
                                            && !loaded_entities.iter().any(|x| x == &entity_id)
                                        {
                                            to_load.push(SaveFileIdentifier::new(Some(sector), entity_id, load_distance));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        (sectors_cache, to_load)
    });

    commands.spawn(LoadingTask(task));
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(SectorsCache::default()).add_systems(
        Update,
        (
            unload_far.run_if(on_timer(Duration::from_millis(1000))),
            load_near.run_if(on_timer(Duration::from_millis(1000))),
            monitor_loading_task,
        ),
    );
}
