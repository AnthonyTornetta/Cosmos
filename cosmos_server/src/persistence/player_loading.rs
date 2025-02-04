//! Loads/unloads entities that are close to/far away from players

use std::{
    ffi::OsStr,
    fs::{self},
    time::Duration,
};

use bevy::{
    log::warn,
    prelude::{not, resource_exists, App, Commands, Entity, IntoSystemConfigs, Name, Or, Query, ResMut, Resource, Update, With, Without},
    state::condition::in_state,
    tasks::{AsyncComputeTaskPool, Task},
    time::common_conditions::on_timer,
};
use cosmos_core::{
    ecs::NeedsDespawned,
    entities::player::Player,
    netty::system_sets::NetworkingSystemsSet,
    persistence::{LoadingDistance, LOAD_DISTANCE},
    physics::location::{systems::Anchor, Location, LocationPhysicsSet, Sector, SectorUnit, SECTOR_DIMENSIONS},
    state::GameState,
};
use futures_lite::future;
use walkdir::{DirEntry, WalkDir};

use super::{loading::NeedsLoaded, saving::NeedsSaved, EntityId, SaveFileIdentifier, SectorsCache};

fn unload_far(
    query: Query<&Location, (Without<NeedsDespawned>, Or<(With<Player>, With<Anchor>)>)>,
    others: Query<(&Location, Entity, &LoadingDistance), (Without<Anchor>, Without<Anchor>, Without<NeedsDespawned>)>,
    mut commands: Commands,
) {
    for (loc, ent, ul_distance) in others.iter() {
        let ul_distance = ul_distance.unload_block_distance();

        if let Some(min_dist) = query.iter().map(|l| l.relative_coords_to(loc).abs().max_element()).reduce(f32::min) {
            if min_dist <= ul_distance {
                continue;
            }
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
                    .expect("A non-base SaveFileIdentifier was attempted to be loaded in load_near")
            }) {
                let name = format!("Needs Loaded Entity - {:?}", sfi.entity_id());
                commands.spawn((sfi, NeedsLoaded, Name::new(name)));
            }
        }
    }
}

/// Performance hot spot
fn load_near(
    q_player_locations: Query<&Location, With<Player>>,
    loaded_entities: Query<&EntityId>,
    // This is modified below, despite it being cloned. Use ResMut to make purpose clear
    sectors_cache: ResMut<SectorsCache>,
    mut commands: Commands,
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
                            for (entity_id, load_distance) in entities.lock().expect("Failed to lock").iter() {
                                if max_delta <= load_distance.unwrap_or(DEFAULT_LOAD_DISTANCE)
                                    && !loaded_entities.iter().any(|x| x == entity_id)
                                {
                                    to_load.push(SaveFileIdentifier::new(Some(sector), entity_id.clone(), *load_distance));
                                }
                            }
                        } else {
                            let dir = format!("world/{}_{}_{}", sector.x(), sector.y(), sector.z());

                            if fs::exists(&dir).unwrap_or(false) {
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

        let mut new_to_load = Vec::with_capacity(to_load.len());
        for sfi in to_load {
            // TODO: Not sure why this exists... for now I'm keeping it, but remove this in the future.

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

    if path.extension() == Some(OsStr::new("cent")) {
        let entity_information = path.file_stem().expect("Failed to get file stem").to_str().expect("to_str failed");

        let entity_id = EntityId::new(entity_information.to_owned());

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
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(SectorsCache::default()).add_systems(
        Update,
        (
            unload_far
                .in_set(NetworkingSystemsSet::Between)
                .after(LocationPhysicsSet::DoPhysics),
            // .run_if(on_timer(Duration::from_millis(1000))),
            load_near
                .run_if(not(resource_exists::<LoadingTask>))
                .in_set(NetworkingSystemsSet::Between)
                .run_if(on_timer(Duration::from_millis(1000))),
            monitor_loading_task.run_if(resource_exists::<LoadingTask>),
        )
            .run_if(in_state(GameState::Playing)),
    );
}
