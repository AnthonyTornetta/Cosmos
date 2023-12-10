//! This handles the saving of different things in the world, such as planets & ships
//!
//! To add your own saving event, add a system after `begin_saving` and before `done_saving`.
//!
//! Use the query: `Query<(Entity, &SerializedData), With<NeedsSaved>>` to get all the data that will need
//! loaded. From there, you can add any components necessary to the entity to fully load it in.
//!
//! See [`saving::default_save`] for an example.

use bevy::{
    ecs::schedule::{apply_deferred, IntoSystemSetConfigs, SystemSet},
    log::warn,
    prelude::{App, Commands, Component, Entity, First, IntoSystemConfigs, Query, ResMut, With, Without},
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This system set is for when entities are being saved normally - NOT FOR A BLUEPRINT (use [`BlueprintingSystemSet`] for that.)
pub enum SavingSystemSet {
    /// Adds the `SerializedData` component to any entities that have the `NeedsSaved` component.
    BeginSaving,
    /// apply_deferred
    FlushBeginSaving,
    /// Put all your saving logic in here
    DoSaving,
    /// apply_deferred
    FlushDoSaving,
    /// This writes the save data to the disk and removes the `SerializedData` and `NeedsSaved` components.
    DoneSaving,
    /// apply_deferred
    FlushDoneSaving,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This system set is for when entities are being blueprinted - NOT FOR A NORMAL SAVE (use [`SavingSystemSet`] for that.)
pub enum BlueprintingSystemSet {
    /// Adds the `SerializedData` component to any entities that have the `NeedsBlueprinted` component.
    BeginBlueprinting,
    /// apply_deferred
    FlushBeginBlueprinting,
    /// Put all your blueprinting logic in here
    DoBlueprinting,
    /// apply_deferred
    FlushDoBlueprinting,
    /// This writes the save data to the disk and removes the `SerializedData` and `NeedsBlueprinted` components.
    DoneBlueprinting,
    /// apply_deferred
    FlushDoneBlueprinting,
}

/// Denotes that this entity should be saved. Once this entity is saved,
/// this component will be removed.
#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsSaved;

/// Denotes that this entity should be saved as a blueprint. Once this entity is saved,
/// this component will be removed.
#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsBlueprinted {
    /// The blueprint file's name (without .bp or the path to it)
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

/// Saves the given structure.
///
/// This is NOT how the structures are saved in the world, but rather used to get structure
/// files that can be loaded through commands.
fn save_blueprint(data: &SerializedData, needs_blueprinted: &NeedsBlueprinted) -> std::io::Result<()> {
    if let Err(e) = fs::create_dir("saves") {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => return Err(e),
        }
    }

    if let Err(e) = fs::create_dir(format!("blueprints/{}", needs_blueprinted.subdir_name)) {
        match e.kind() {
            ErrorKind::AlreadyExists => {}
            _ => return Err(e),
        }
    }

    fs::write(
        format!(
            "blueprints/{}/{}.bp",
            needs_blueprinted.subdir_name, needs_blueprinted.blueprint_name
        ),
        cosmos_encoder::serialize(&data),
    )?;

    Ok(())
}

/// Put all systems that add data to blueprinted entities before this and after `begin_blueprinting`
fn done_blueprinting(mut query: Query<(Entity, &mut SerializedData, &NeedsBlueprinted, Option<&NeedsSaved>)>, mut commands: Commands) {
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

/// Make sure any systems that serialize data for saving are run before this
fn done_saving(
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

/// The schedule saving takes place in - this may change in the future
pub const SAVING_SCHEDULE: First = First;

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        SAVING_SCHEDULE,
        (
            SavingSystemSet::BeginSaving,
            SavingSystemSet::FlushBeginSaving,
            SavingSystemSet::DoSaving,
            SavingSystemSet::FlushDoSaving,
            SavingSystemSet::DoneSaving,
            SavingSystemSet::FlushDoneSaving,
        )
            .chain()
            .before(despawn_needed),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (
            // Defers
            apply_deferred.in_set(SavingSystemSet::FlushBeginSaving),
            apply_deferred.in_set(SavingSystemSet::FlushDoSaving),
            apply_deferred.in_set(SavingSystemSet::FlushDoneSaving),
            // Logic
            check_needs_saved.in_set(SavingSystemSet::BeginSaving),
            default_save.in_set(SavingSystemSet::DoSaving),
            done_saving.in_set(SavingSystemSet::DoneSaving),
        ),
    );

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            BlueprintingSystemSet::BeginBlueprinting,
            BlueprintingSystemSet::FlushBeginBlueprinting,
            BlueprintingSystemSet::DoBlueprinting,
            BlueprintingSystemSet::FlushDoBlueprinting,
            BlueprintingSystemSet::DoneBlueprinting,
            BlueprintingSystemSet::FlushDoneBlueprinting,
        )
            .chain()
            .before(SavingSystemSet::BeginSaving),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (
            // Defers
            apply_deferred.in_set(BlueprintingSystemSet::FlushBeginBlueprinting),
            apply_deferred.in_set(BlueprintingSystemSet::FlushDoBlueprinting),
            apply_deferred.in_set(BlueprintingSystemSet::FlushDoneBlueprinting),
            // Logic
            check_needs_blueprinted.in_set(BlueprintingSystemSet::BeginBlueprinting),
            done_blueprinting.in_set(BlueprintingSystemSet::DoneBlueprinting),
        ),
    );
}
