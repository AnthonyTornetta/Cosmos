//! Loads/unloads entities that are close to/far away from players

use std::{
    ffi::OsStr,
    fs::{self},
    time::Duration,
};

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
    time::common_conditions::on_timer,
};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    persistence::{LOAD_DISTANCE, LoadingDistance},
    physics::{
        location::{Location, LocationPhysicsSet, SECTOR_DIMENSIONS, Sector, SectorUnit, systems::Anchor},
        player_world::PlayerWorld,
    },
    state::GameState,
};
use futures_lite::future;
use uuid::Uuid;
use walkdir::{DirEntry, WalkDir};

use crate::persistence::{NORMAL_ENTITY_EXTENSION, WorldRoot, loading::PreLoadingStages};

use super::{EntityId, SaveFileIdentifier, SectorsCache, loading::NeedsLoaded, saving::NeedsSaved};

fn unload_far(
    query: Query<&Location, (Without<PlayerWorld>, Or<(With<Player>, With<Anchor>)>)>,
    others: Query<
        (Option<&Name>, Option<&EntityId>, &Location, Entity, &LoadingDistance),
        (Without<Anchor>, Without<Player>, Without<NeedsDespawned>, Without<PlayerWorld>),
    >,
    mut commands: Commands,
) {
    for (name, ent_id, loc, ent, ul_distance) in others.iter() {
        let ul_distance = ul_distance.unload_block_distance();

        if let Some(min_dist) = query.iter().map(|l| l.relative_coords_to(loc).abs().max_element()).reduce(f32::min)
            && min_dist <= ul_distance
        {
            continue;
        }

        if let Some(name) = name {
            info!("Unloading {name} ({ent:?}) ({ent_id:?}) at {loc} - too far away from any anchor.");
        } else {
            info!("Unloading {ent:?} ({ent_id:?}) at {loc} - too far away from any anchor.");
        }

        commands.entity(ent).insert((NeedsSaved, NeedsDespawned));
    }
}

const SEARCH_RANGE: SectorUnit = 25;
const DEFAULT_LOAD_DISTANCE: u32 = (LOAD_DISTANCE / SECTOR_DIMENSIONS) as u32;

#[derive(Resource, Debug)]
struct LoadingTask(Task<Vec<SaveFileIdentifier>>);

fn monitor_loading_task(
    // Because entities can be added while the scan task is in progress,
    // we need to re-check all the loaded entities before actually spawning them.
    loaded_entities: Query<&EntityId>,
    mut task: ResMut<LoadingTask>,
    mut commands: Commands,
) {
    if let Some(save_file_ids) = future::block_on(future::poll_once(&mut task.0)) {
        commands.remove_resource::<LoadingTask>();

        for sfi in save_file_ids {
            if !loaded_entities.iter().any(|x| {
                x == sfi
                    .entity_id()
                    .expect("A non-entity id SaveFileIdentifier was attempted to be loaded in load_near")
            }) {
                let entity_id = *sfi.entity_id().expect("Missing entity id");

                let name = format!("Needs Loaded Entity - {entity_id}");

                info!("Loading {entity_id}");

                commands.spawn((sfi, entity_id, NeedsLoaded, Name::new(name)));
            }
        }
    }
}

#[derive(Component)]
/// This component can be added to any entity, which will signify to the loading system that its
/// children may not all be loaded. This component will immediately trigger a load of all this
/// entity's children in the next [`LoadingSystemSet`] execution.
pub struct RecomputeNeedLoadedChildren;

fn recompute_need_loaded_children(
    q_need_reloaded_children: Query<
        (
            Option<&Location>,
            Option<&LoadingDistance>,
            Option<&SaveFileIdentifier>,
            &EntityId,
            Entity,
        ),
        With<RecomputeNeedLoadedChildren>,
    >,
    mut commands: Commands,
    loaded_entities: Query<&EntityId>,
) {
    if q_need_reloaded_children.is_empty() {
        return;
    }

    let loaded_entities = loaded_entities.iter().copied().collect::<Vec<_>>();

    // These need to be done immediately, because one of the entities already exists and we need to
    // fix the invalid heirarchy now, not in some async task that can be take multiple frames.

    let mut to_load = vec![];
    for (loc, loading_distance, sfi, entity_id, entity) in q_need_reloaded_children {
        let sfi = sfi.cloned().or_else(|| {
            let loc = loc?;
            let loading_distance = loading_distance?;

            Some(SaveFileIdentifier::new(
                Some(loc.sector()),
                *entity_id,
                Some(loading_distance.load_distance()),
            ))
        });

        let Some(sfi) = sfi else {
            error!("Unable to compute save file identifier for {entity_id:?} ({entity:?})");
            continue;
        };

        let child_dir = sfi.get_children_directory();

        info!("Fixing Parent Heirarchy: {child_dir:?}");

        for file in WalkDir::new(&child_dir)
            .max_depth(1)
            .into_iter()
            .flatten()
            .filter(|x| x.file_type().is_file())
        {
            load_all(sfi.clone(), file, &mut to_load, &loaded_entities);
        }

        if !loaded_entities.iter().any(|x| Some(x) == sfi.entity_id()) {
            to_load.push(sfi);
        }

        commands.entity(entity).remove::<RecomputeNeedLoadedChildren>();
    }

    for sfi in to_load {
        let entity_id = *sfi.entity_id().expect("Missing entity id");

        let name = format!("Needs Loaded Entity - {entity_id}");

        info!("Loading {entity_id}");

        commands.spawn((sfi, entity_id, NeedsLoaded, Name::new(name)));
    }
}

/// Performance hot spot
fn load_near(
    q_player_locations: Query<&Location, With<Anchor>>,
    loaded_entities: Query<&EntityId>,
    // This is modified below, despite it being cloned. Use ResMut to make purpose clear
    sectors_cache: ResMut<SectorsCache>,
    mut commands: Commands,
    world_root: Res<WorldRoot>,
) {
    if q_player_locations.is_empty() {
        // Don't bother if there are no players
        return;
    }

    let thread_pool = AsyncComputeTaskPool::get();

    let sectors = q_player_locations.iter().map(|l| l.sector()).collect::<Vec<Sector>>();

    // Shallow clone - we are only cloning the Arc<Mutex<...>> not the ...
    let mut sectors_cache = sectors_cache.clone();

    // If this ever gets laggy, either of this clone could be the cause
    let loaded_entities = loaded_entities.iter().copied().collect::<Vec<_>>();

    let world_root = world_root.clone();

    let task = thread_pool.spawn(async move {
        let mut to_load = vec![];

        for sector in sectors {
            for dz in -SEARCH_RANGE..=SEARCH_RANGE {
                for dy in -SEARCH_RANGE..=SEARCH_RANGE {
                    for dx in -SEARCH_RANGE..SEARCH_RANGE {
                        let sector = Sector::new(dx + sector.x(), dy + sector.y(), dz + sector.z());
                        let max_delta = dz.abs().max(dy.abs()).max(dx.abs()) as u32;

                        if let Some(entities) = sectors_cache.get(&sector) {
                            for (entity_id, load_distance) in entities.lock().expect("Failed to lock").iter() {
                                if max_delta <= load_distance.unwrap_or(DEFAULT_LOAD_DISTANCE) && !loaded_entities.contains(entity_id) {
                                    to_load.push(SaveFileIdentifier::new(Some(sector), *entity_id, *load_distance));
                                }
                            }
                        } else {
                            let dir = world_root.path_for(format!("{}_{}_{}", sector.x(), sector.y(), sector.z()).as_str());

                            if fs::exists(&dir).unwrap_or(false) {
                                for file in WalkDir::new(&dir)
                                    .max_depth(1)
                                    .into_iter()
                                    .flatten()
                                    .filter(|x| x.file_type().is_file())
                                {
                                    let path = file.path();

                                    if path.extension() == Some(OsStr::new(NORMAL_ENTITY_EXTENSION)) {
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

                                        let entity_id = EntityId::new(Uuid::parse_str(entity_id).expect("Failed to parse entity id!"));

                                        sectors_cache.insert(sector, entity_id, load_distance);

                                        if max_delta <= load_distance.unwrap_or(DEFAULT_LOAD_DISTANCE)
                                            && !loaded_entities.contains(&entity_id)
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

        // This will load any children of the ones we marked to load
        let mut new_to_load = Vec::with_capacity(to_load.len());
        for sfi in to_load {
            let child_dir = sfi.get_children_directory();

            for file in WalkDir::new(&child_dir)
                .max_depth(1)
                .into_iter()
                .flatten()
                .filter(|x| x.file_type().is_file())
            {
                load_all(sfi.clone(), file, &mut new_to_load, &loaded_entities);
            }

            if !loaded_entities.iter().any(|x| Some(x) == sfi.entity_id()) {
                new_to_load.push(sfi);
            }
        }

        new_to_load
    });

    commands.insert_resource(LoadingTask(task));
}

fn load_all(base: SaveFileIdentifier, file: DirEntry, to_load: &mut Vec<SaveFileIdentifier>, loaded_entities: &[EntityId]) {
    let path = file.path();

    if path.extension() != Some(OsStr::new(NORMAL_ENTITY_EXTENSION)) {
        return;
    }

    let entity_information = path.file_stem().expect("Failed to get file stem").to_str().expect("to_str failed");

    let entity_id = EntityId::new(
        Uuid::parse_str(entity_information).unwrap_or_else(|e| panic!("Failed to parse entity id `{entity_information}` {e:?}!")),
    );

    let sfi = SaveFileIdentifier::sub_entity(base, entity_id);

    let child_dir = sfi.get_children_directory();

    for file in WalkDir::new(child_dir)
        .max_depth(1)
        .into_iter()
        .flatten()
        .filter(|x| x.file_type().is_file())
    {
        load_all(sfi.clone(), file, to_load, loaded_entities);
    }

    if !loaded_entities.iter().any(|x| Some(x) == sfi.entity_id()) {
        to_load.push(sfi);
    }
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<SectorsCache>().add_systems(
        FixedUpdate,
        (
            unload_far
                .in_set(NetworkingSystemsSet::Between)
                .after(LocationPhysicsSet::DoPhysics),
            // .run_if(on_timer(Duration::from_millis(1000))),
            recompute_need_loaded_children.in_set(PreLoadingStages::EnsureCorrectHeirarchies),
            load_near
                .run_if(not(resource_exists::<LoadingTask>))
                .in_set(NetworkingSystemsSet::Between)
                .run_if(on_timer(Duration::from_millis(1000))),
            monitor_loading_task.run_if(resource_exists::<LoadingTask>),
        )
            .run_if(in_state(GameState::Playing)),
    );
}
