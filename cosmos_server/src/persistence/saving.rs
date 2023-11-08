//! This handles the saving of different things in the world, such as planets & ships
//!
//! To add your own saving event, add a system after `begin_saving` and before `done_saving`.
//!
//! Use the query: `Query<(Entity, &SerializedData), With<NeedsSaved>>` to get all the data that will need
//! loaded. From there, you can add any components necessary to the entity to fully load it in.
//!
//! See [`saving::default_save`] for an example.

use bevy::{
    prelude::{warn, App, Commands, Component, Entity, First, IntoSystemConfigs, PostUpdate, Query, ResMut, With, Without},
    reflect::Reflect,
};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    ecs::{despawn_needed, NeedsDespawned},
    netty::cosmos_encoder,
    persistence::LoadingDistance,
    physics::location::Location,
};
use std::{
    fs,
    io::{self, ErrorKind},
};

use super::{EntityId, SaveFileIdentifier, SaveFileIdentifierType, SectorsCache, SerializedData};

/// Denotes that this entity should be saved. Once this entity is saved,
/// this component will be removed.
#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsSaved;

/// Denotes that this entity should be saved as a blueprint. Once this entity is saved,
/// this component will be removed.
#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsBlueprinted {
    /// The blueprint file's name (without .blueprint or the path to it)
    pub blueprint_name: String,
    /// The subdirectory the blueprint resides in (same as the blueprint type)
    pub subdir_name: String,
}

fn check_needs_saved(query: Query<Entity, (With<NeedsSaved>, Without<SerializedData>)>, mut commands: Commands) {
    for ent in query.iter() {
        commands.entity(ent).insert(SerializedData::default());
    }
}

fn check_needs_blueprinted(query: Query<Entity, (With<NeedsBlueprinted>, Without<SerializedData>)>, mut commands: Commands) {
    for ent in query.iter() {
        commands.entity(ent).insert(SerializedData::default());
    }
}

/// Put all systems that add data to blueprinted entities after this and before `done_blueprinting`
pub fn begin_blueprinting() {}

/// Saves the given structure.
///
/// This is NOT how the structures are saved in the world, but rather used to get structure
/// files that can be loaded through commands.
pub fn save_blueprint(data: &SerializedData, needs_blueprinted: &NeedsBlueprinted) -> std::io::Result<()> {
    if let Err(e) = fs::create_dir("saves") {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => return Err(e),
        }
    }

    if let Err(e) = fs::create_dir(format!("saves/{}", needs_blueprinted.subdir_name)) {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => return Err(e),
        }
    }

    fs::write(
        format!(
            "saves/{}/{}.blueprint",
            needs_blueprinted.subdir_name, needs_blueprinted.blueprint_name
        ),
        cosmos_encoder::serialize(&data),
    )?;

    Ok(())
}

/// Put all systems that add data to blueprinted entities before this and after `begin_blueprinting`
pub fn done_blueprinting(mut query: Query<(Entity, &mut SerializedData, &NeedsBlueprinted, Option<&NeedsSaved>)>, mut commands: Commands) {
    for (entity, mut serialized_data, needs_blueprinted, needs_saved) in query.iter_mut() {
        save_blueprint(&serialized_data, needs_blueprinted)
            .unwrap_or_else(|e| warn!("Failed to save blueprint for {entity:?} \n\n{e}\n\n"));

        commands.entity(entity).remove::<NeedsBlueprinted>();

        if needs_saved.is_none() {
            commands.entity(entity).remove::<SerializedData>();
        } else {
            // Clear out any blueprint data for the actual saving coming up
            *serialized_data = SerializedData::default();
        }
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
            Option<&LoadingDistance>,
            Option<&SaveFileIdentifier>,
        ),
        With<NeedsSaved>,
    >,
    dead_saves_query: Query<&SaveFileIdentifier, (With<NeedsDespawned>, Without<NeedsSaved>)>,
    mut sectors_cache: ResMut<SectorsCache>,
    mut commands: Commands,
) {
    for dead_save in dead_saves_query.iter() {
        let path = dead_save.get_save_file_path();
        if fs::try_exists(&path).unwrap_or(false) {
            fs::remove_file(path).expect("Error deleting old save file!");

            if let SaveFileIdentifierType::Base((entity_id, Some(sector), load_distance)) = &dead_save.identifier_type {
                sectors_cache.remove(entity_id, *sector, *load_distance);
            }
        }
    }

    for (entity, sd, entity_id, loading_distance, save_file_identifier) in query.iter() {
        commands.entity(entity).remove::<NeedsSaved>().remove::<SerializedData>();

        if !sd.should_save() {
            continue;
        }

        let entity_id = if let Some(id) = entity_id {
            id.clone()
        } else {
            let entity_id = EntityId::generate();

            commands.entity(entity).insert(entity_id.clone());

            entity_id
        };

        if let Some(save_file_identifier) = save_file_identifier {
            let path = save_file_identifier.get_save_file_path();
            if fs::try_exists(&path).unwrap_or(false) {
                fs::remove_file(path).expect("Error deleting old save file!");

                if let SaveFileIdentifierType::Base((entity_id, Some(sector), load_distance)) = &save_file_identifier.identifier_type {
                    sectors_cache.remove(entity_id, *sector, *load_distance);
                }
            }
        }

        let serialized: Vec<u8> = cosmos_encoder::serialize(&sd);

        let save_identifier = save_file_identifier.cloned().unwrap_or_else(|| {
            let sfi = SaveFileIdentifier::new(
                sd.location.map(|l| l.sector()),
                entity_id.clone(),
                loading_distance.map(|ld| ld.load_distance()),
            );

            commands.entity(entity).insert(sfi.clone());

            sfi
        });

        if let Err(e) = write_file(&save_identifier, &serialized) {
            warn!("{e}");
            continue;
        }

        if matches!(&save_identifier.identifier_type, SaveFileIdentifierType::Base(_)) {
            if let Some(loc) = sd.location {
                sectors_cache.insert(loc.sector(), entity_id, loading_distance.map(|ld| ld.load_distance()));
            }
        }
    }
}

fn write_file(save_identifier: &SaveFileIdentifier, serialized: &[u8]) -> io::Result<()> {
    let path = save_identifier.get_save_file_path();

    let directory = &path[0..path.rfind('/').expect("No / found in file path!")];

    fs::create_dir_all(directory)?;

    fs::write(&path, serialized)?;

    Ok(())
}

fn default_save(mut query: Query<(&mut SerializedData, Option<&Location>, Option<&Velocity>, Option<&LoadingDistance>), With<NeedsSaved>>) {
    for (mut data, loc, vel, loading_distance) in query.iter_mut() {
        if let Some(loc) = loc {
            data.set_location(loc);
        }

        if let Some(vel) = vel {
            data.serialize_data("cosmos:velocity", vel);
        }

        if let Some(val) = loading_distance {
            data.serialize_data("cosmos:loading_distance", val);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(PostUpdate, (check_needs_saved, check_needs_blueprinted))
        // Add all blueprint-related systems between these systems
        .add_systems(First, (begin_blueprinting, done_blueprinting).chain().before(begin_saving))
        // Put all saving-related systems after this
        .add_systems(First, begin_saving.before(despawn_needed))
        // Put all saving-related systems before this
        .add_systems(First, done_saving.after(begin_saving).before(despawn_needed))
        // Like this:
        .add_systems(First, default_save.after(begin_saving).before(done_saving));
}
