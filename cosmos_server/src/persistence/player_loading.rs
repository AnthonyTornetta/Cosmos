//! Loads/unloads entities that are close to/far away from players

use std::{
    ffi::OsStr,
    fs::{self},
};

use bevy::{
    prelude::{App, Commands, Entity, Query, ResMut, With, Without},
    utils::HashSet,
};
use cosmos_core::{
    entities::player::Player,
    physics::location::{Location, SECTOR_DIMENSIONS},
};
use walkdir::WalkDir;

use super::{
    loading::NeedsLoaded,
    saving::{NeedsSaved, NeedsUnloaded},
    EntityId, SaveFileIdentifier, SectorsCache,
};

const UNLOAD_DISTANCE: f32 = SECTOR_DIMENSIONS * 10.0;
const LOAD_DISTANCE: f32 = SECTOR_DIMENSIONS * 8.0;

fn unload_far(
    query: Query<&Location, With<Player>>,
    others: Query<(&Location, Entity), (Without<Player>, Without<NeedsUnloaded>)>,
    mut commands: Commands,
) {
    for (loc, ent) in others.iter() {
        if let Some(min_dist) = query.iter().map(|l| l.distance_sqrd(loc)).reduce(f32::min) {
            if min_dist < UNLOAD_DISTANCE * UNLOAD_DISTANCE {
                continue;
            }
        }
        // uncomment if need to generate planet again
        // else {
        //     continue;
        // }

        println!("Flagged for saving + unloading!");

        commands.entity(ent).insert((NeedsSaved, NeedsUnloaded));
    }
}

fn load_near(
    query: Query<&Location, With<Player>>,
    loaded_entities: Query<&EntityId>,
    mut sectors_cache: ResMut<SectorsCache>,
    mut commands: Commands,
) {
    for loc in query.iter() {
        let delta_ld = (LOAD_DISTANCE / SECTOR_DIMENSIONS) as i64;

        for dz in -delta_ld..=delta_ld {
            for dy in -delta_ld..=delta_ld {
                for dx in -delta_ld..delta_ld {
                    let sector = (dx + loc.sector_x, dy + loc.sector_y, dz + loc.sector_z);

                    if let Some(vec) = sectors_cache.0.get(&sector) {
                        for entity_id in vec.iter() {
                            if !loaded_entities.iter().any(|x| x == entity_id) {
                                commands.spawn((
                                    SaveFileIdentifier {
                                        entity_id: entity_id.clone(),
                                        sector: Some(sector),
                                    },
                                    NeedsLoaded,
                                ));
                            }
                        }
                    } else {
                        let mut cache = HashSet::new();

                        let (x, y, z) = sector;

                        let dir = format!("world/{x}_{y}_{z}");
                        if fs::try_exists(&dir).unwrap_or(false) {
                            for file in WalkDir::new(&dir).into_iter() {
                                if let Ok(file) = file {
                                    let path = file.path();

                                    if file.file_type().is_file()
                                        && path.extension() == Some(OsStr::new("cent"))
                                    {
                                        let entity_id = path
                                            .file_stem()
                                            .expect("Failed to get file stem")
                                            .to_str()
                                            .expect("Failed to convert to entity id")
                                            .to_owned();

                                        let entity_id = EntityId::new(entity_id);

                                        cache.insert(entity_id.clone());

                                        if !loaded_entities.iter().any(|x| x == &entity_id) {
                                            commands.spawn((
                                                SaveFileIdentifier {
                                                    entity_id,
                                                    sector: Some((x, y, z)),
                                                },
                                                NeedsLoaded,
                                            ));
                                        }
                                    }
                                }
                            }
                        }

                        sectors_cache.0.insert((x, y, z), cache);
                    }
                }
            }
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.insert_resource(SectorsCache::default())
        .add_system(unload_far)
        .add_system(load_near);
}
