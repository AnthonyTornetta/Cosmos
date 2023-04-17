//! This handles the saving of different things in the world, such as planets & ships
//!
//! To add your own saving event, add a system after `begin_saving` and before `done_saving`.
//!
//! Use the query: `Query<(Entity, &SerializedData), With<NeedsSaved>>` to get all the data that will need
//! loaded. From there, you can add any components necessary to the entity to fully load it in.
//!
//! See [`saving::default_save`] for an example.

use bevy::{
    prelude::{
        App, Commands, Component, CoreSet, DespawnRecursiveExt, Entity, IntoSystemConfig, Query,
        ResMut, With, Without,
    },
    reflect::Reflect,
    utils::HashSet,
};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{netty::cosmos_encoder, physics::location::Location};
use rand::{distributions::Alphanumeric, Rng};
use std::{
    fs,
    io::{self, Write},
};
use zip::{write::FileOptions, ZipWriter};

use super::{EntityId, SaveFileIdentifier, SectorsCache, SerializedData};

/// Denotes that this entity should be saved. Once this entity is saved,
/// this component will be removed.
#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsSaved;

/// This flag will denote that once this entity is saved, it will be unloaded.
/// To save this entity, make sure to also add `NeedsSaved`
#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsUnloaded;

fn check_needs_saved(
    query: Query<Entity, (With<NeedsSaved>, Without<SerializedData>)>,
    mut commands: Commands,
) {
    for ent in query.iter() {
        commands.entity(ent).insert(SerializedData::default());
    }
}

/// Make sure any systems that serialize data for saving are run after this
///
/// Make sure those systems are run before `done_saving` aswell.
pub fn begin_saving() {}

/// Make sure any systems that serialize data for saving are run before this
///
/// Make sure those systems are run after `begin_saving` aswell.
pub fn done_saving(
    query: Query<
        (
            Entity,
            &SerializedData,
            Option<&EntityId>,
            Option<&NeedsUnloaded>,
            Option<&SaveFileIdentifier>,
        ),
        With<NeedsSaved>,
    >,
    mut sectors_cache: ResMut<SectorsCache>,
    mut commands: Commands,
) {
    for (entity, sd, entity_id, needs_unloaded, save_file_identifier) in query.iter() {
        commands
            .entity(entity)
            .remove::<NeedsSaved>()
            .remove::<SerializedData>();

        if !sd.should_save() {
            continue;
        }

        let serialized = cosmos_encoder::serialize(&sd);

        let entity_id = if let Some(id) = entity_id {
            id.clone()
        } else {
            let res: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();

            let entity_id = EntityId(res);

            commands.entity(entity).insert(entity_id.clone());

            entity_id
        };

        if let Some(save_file_identifier) = save_file_identifier {
            let path = save_file_identifier.get_save_file_path();
            println!("Found @ {path}");
            if fs::try_exists(&path).unwrap_or(false) {
                println!("Removing old!");
                fs::remove_file(path).expect("Error deleting old save file!");

                if let Some(sector) = &save_file_identifier.sector {
                    sectors_cache
                        .0
                        .get_mut(sector)
                        .map(|set| set.remove(&save_file_identifier.entity_id));
                }
            }
        } else {
            println!("No previous path no delete.");
        }

        let save_identifier = SaveFileIdentifier {
            entity_id: entity_id.clone(),
            sector: sd.location.map(|l| (l.sector_x, l.sector_y, l.sector_z)),
        };

        if let Err(e) = write_file(&save_identifier, &serialized) {
            eprintln!("{e}");
            continue;
        }

        commands.entity(entity).insert(save_identifier);

        if let Some(loc) = sd.location {
            let key = (loc.sector_x, loc.sector_y, loc.sector_z);
            if !sectors_cache.0.contains_key(&key) {
                sectors_cache.0.insert(key, HashSet::new());
            }

            sectors_cache.0.get_mut(&key).unwrap().insert(entity_id);
        }

        if needs_unloaded.is_some() {
            commands.entity(entity).despawn_recursive();
        }
    }
}

fn write_file(save_identifier: &SaveFileIdentifier, serialized: &[u8]) -> io::Result<()> {
    println!("New save file ID: {save_identifier:?}");

    let path = save_identifier.get_save_file_path();

    let directory = &path[0..path.rfind('/').expect("No / found in file path!")];

    fs::create_dir_all(directory)?;

    let file = fs::File::create(&path)?;
    let mut zipped = ZipWriter::new(file);

    let options = FileOptions::default()
        .compression_method(zip::CompressionMethod::Bzip2)
        .unix_permissions(0o755);

    zipped.start_file(save_identifier.get_save_file_name(), options)?;
    zipped.write_all(serialized)?;
    zipped.finish()?;

    Ok(())
}

fn default_save(
    mut query: Query<(&mut SerializedData, Option<&Location>, Option<&Velocity>), With<NeedsSaved>>,
) {
    for (mut data, loc, vel) in query.iter_mut() {
        if let Some(loc) = loc {
            data.set_location(loc);
        }

        if let Some(vel) = vel {
            data.serialize_data("cosmos:velocity", vel);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(check_needs_saved)
        // Put all saving-related systems after this
        .add_system(begin_saving.in_base_set(CoreSet::First))
        // Put all saving-related systems before this
        .add_system(done_saving.after(begin_saving))
        // Like this:
        .add_system(default_save.after(begin_saving).before(done_saving));
}
