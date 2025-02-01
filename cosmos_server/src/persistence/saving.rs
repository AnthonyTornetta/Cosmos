//! This handles the saving of different things in the world, such as planets & ships
//!
//! To add your own saving event, add a system after `begin_saving` and before `done_saving`.
//!
//! Use the query: `Query<(Entity, &SerializedData), With<NeedsSaved>>` to get all the data that will need
//! loaded. From there, you can add any components necessary to the entity to fully load it in.
//!
//! See [`saving::default_save`] for an example.

use bevy::{
    core::Name,
    ecs::schedule::{IntoSystemSetConfigs, SystemSet},
    hierarchy::Parent,
    log::{error, info, warn},
    prelude::{App, Commands, Component, Entity, First, IntoSystemConfigs, Or, Query, ResMut, Transform, With, Without},
    reflect::Reflect,
};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    ecs::{despawn_needed, NeedsDespawned},
    entities::player::Player,
    netty::cosmos_encoder,
    persistence::LoadingDistance,
    physics::location::Location,
};
use std::{
    fs,
    io::{self, ErrorKind},
};

use super::{EntityId, PreviousSaveFileIdentifier, SaveFileIdentifier, SaveFileIdentifierType, SectorsCache, SerializedData};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This system set is for when entities are being saved normally - NOT FOR A BLUEPRINT (use [`BlueprintingSystemSet`] for that.)
pub enum SavingSystemSet {
    /// Adds the `SerializedData` component to any entities that have the `NeedsSaved` component.
    BeginSaving,
    /// Put all your saving logic in here
    DoSaving,
    /// Creates any entity ids that need to be created for the saved entities.
    CreateEntityIds,
    /// This writes the save data to the disk and removes the `SerializedData` and `NeedsSaved` components.
    DoneSaving,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This system set is for when entities are being blueprinted - NOT FOR A NORMAL SAVE (use [`SavingSystemSet`] for that.)
pub enum BlueprintingSystemSet {
    /// Adds the `SerializedData` component to any entities that have the `NeedsBlueprinted` component.
    BeginBlueprinting,
    /// Put all your blueprinting logic in here
    DoBlueprinting,
    /// This writes the save data to the disk and removes the `SerializedData` and `NeedsBlueprinted` components.
    DoneBlueprinting,
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

fn check_needs_saved(
    q_parent: Query<&Parent, Or<(Without<SerializedData>, Without<NeedsSaved>)>>,
    q_needs_serialized_data: Query<(Entity, Option<&Parent>), (With<NeedsSaved>, Without<SerializedData>)>,
    mut commands: Commands,
) {
    for (ent, mut parent) in q_needs_serialized_data.iter() {
        commands.entity(ent).insert(SerializedData::default());

        // If something that needs saved has parents, we must propagate it up to work properly.
        while let Some(p) = parent {
            let ent = p.get();
            commands.entity(ent).insert((SerializedData::default(), NeedsSaved));
            parent = q_parent.get(ent).ok();
        }
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

fn create_entity_ids(mut commands: Commands, q_without_id: Query<(Entity, &SerializedData), (Without<EntityId>, With<NeedsSaved>)>) {
    for (ent, sd) in q_without_id.iter() {
        if !sd.should_save() {
            continue;
        }

        commands.entity(ent).insert(EntityId::generate());
    }
}

/// Make sure any systems that serialize data for saving are run before this
fn done_saving(
    q_needs_saved: Query<
        (
            Entity,
            Option<&Name>,
            &SerializedData,
            &EntityId,
            Option<&LoadingDistance>,
            Option<&SaveFileIdentifier>,
            Option<&PreviousSaveFileIdentifier>,
            Option<&Player>,
        ),
        With<NeedsSaved>,
    >,
    q_parent: Query<&Parent>,
    q_entity_id: Query<&EntityId>,
    q_serialized_data: Query<(&SerializedData, &EntityId, Option<&LoadingDistance>)>,
    dead_saves_query: Query<&PreviousSaveFileIdentifier, (With<NeedsDespawned>, Without<NeedsSaved>)>,
    mut sectors_cache: ResMut<SectorsCache>,
    mut commands: Commands,
) {
    for dead_save in dead_saves_query.iter() {
        let path = dead_save.0.get_save_file_path();
        if fs::exists(&path).unwrap_or(false) {
            fs::remove_file(path).expect("Error deleting old save file!");

            if let SaveFileIdentifierType::Base(entity_id, Some(sector), load_distance) = &dead_save.0.identifier_type {
                sectors_cache.remove(entity_id, *sector, *load_distance);
            }
        }
    }

    for (entity, name, sd, entity_id, loading_distance, mut save_file_identifier, previous_sfi, player) in q_needs_saved.iter() {
        commands.entity(entity).remove::<NeedsSaved>().remove::<SerializedData>();

        if !sd.should_save() {
            continue;
        }

        if matches!(
            save_file_identifier,
            Some(SaveFileIdentifier {
                identifier_type: SaveFileIdentifierType::Base(_, _, _)
            })
        ) && loading_distance.is_none()
        {
            if let Some(name) = name {
                error!("Missing load distance for {name} {entity:?} w/ base savefileidentifier type!");
            } else {
                error!("Missing load distance for {entity:?} w/ base savefileidentifier type!");
            }

            commands.entity(entity).log_components();
        }

        // Required to be in the outer scope so the reference is still valid
        let sfi: Option<SaveFileIdentifier>;
        if save_file_identifier.is_none() {
            sfi = calculate_sfi(entity, &q_parent, &q_entity_id, &q_serialized_data);
            save_file_identifier = sfi.as_ref();
        } else {
            info!("Save file component already on entity ({entity:?})- {save_file_identifier:?}");
        }

        let Some(save_file_identifier) = save_file_identifier else {
            error!("Could not calculate save file identifier for {entity:?} - loggin components");
            commands.entity(entity).log_components();
            continue;
        };

        if let Some(previous_sfi) = previous_sfi {
            let path = previous_sfi.0.get_save_file_path();
            if fs::exists(&path).unwrap_or(false) {
                if fs::remove_file(&path).is_err() {
                    warn!("Error deleting old save file at {path}!");
                }

                if let SaveFileIdentifierType::Base(entity_id, Some(sector), load_distance) = &previous_sfi.0.identifier_type {
                    sectors_cache.remove(entity_id, *sector, *load_distance);
                }
            }
        }

        commands
            .entity(entity)
            .insert(PreviousSaveFileIdentifier(save_file_identifier.clone()));

        let serialized: Vec<u8> = cosmos_encoder::serialize(&sd);

        info!("WRITING TO DISK - {save_file_identifier:?}");

        if let Err(e) = write_file(save_file_identifier, &serialized) {
            error!("Unable to save {entity:?}\n{e}");
        }

        if let Some(player) = player {
            info!("Saving player data for {player:?} to disk.");
        }

        if matches!(&save_file_identifier.identifier_type, SaveFileIdentifierType::Base(_, _, _)) {
            if let Some(loc) = sd.location {
                sectors_cache.insert(loc.sector(), entity_id.clone(), loading_distance.map(|ld| ld.load_distance()));
            }
        }
    }
}

/// This is in a bad spot, and should be moved.
pub(crate) fn calculate_sfi(
    entity: Entity,
    q_parent: &Query<&Parent>,
    q_entity_id: &Query<&EntityId>,
    q_serialized_data: &Query<(&SerializedData, &EntityId, Option<&LoadingDistance>)>,
) -> Option<SaveFileIdentifier> {
    let Ok(parent) = q_parent.get(entity) else {
        let Ok((sd, entity_id, loading_distance)) = q_serialized_data.get(entity) else {
            error!("Entity {entity:?} missing entity serialized data. Cannot save {entity:?}.");
            return None;
        };

        return Some(SaveFileIdentifier::new(
            sd.location.map(|l| l.sector()),
            entity_id.clone(),
            loading_distance.map(|ld| ld.load_distance()),
        ));
    };

    let Ok(entity_id) = q_entity_id.get(entity) else {
        error!("Missing entity id for {entity:?} - cannot generate save file identifier.");
        return None;
    };

    let Some(parent_sfi) = calculate_sfi(parent.get(), q_parent, q_entity_id, q_serialized_data) else {
        error!("Could not calculate parent save file identifier - not saving {entity:?}");
        return None;
    };

    Some(SaveFileIdentifier::sub_entity(parent_sfi, entity_id.clone()))
}

fn write_file(save_identifier: &SaveFileIdentifier, serialized: &[u8]) -> io::Result<()> {
    let path = save_identifier.get_save_file_path();

    let directory = &path[0..path.rfind('/').expect("No / found in file path!")];

    fs::create_dir_all(directory)?;

    fs::write(&path, serialized)?;

    Ok(())
}

fn default_save(
    mut query: Query<
        (
            &mut SerializedData,
            Option<&Location>,
            Option<&Velocity>,
            Option<&LoadingDistance>,
            Option<&Transform>,
        ),
        With<NeedsSaved>,
    >,
) {
    for (mut data, loc, vel, loading_distance, transform) in query.iter_mut() {
        if let Some(loc) = loc {
            data.set_location(loc);
        }

        if let Some(vel) = vel {
            data.serialize_data("cosmos:velocity", vel);
        }

        if let Some(val) = loading_distance {
            data.serialize_data("cosmos:loading_distance", val);
        }

        if let Some(trans) = transform {
            data.serialize_data("cosmos:rotation", &trans.rotation);
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
            SavingSystemSet::DoSaving,
            SavingSystemSet::CreateEntityIds,
            SavingSystemSet::DoneSaving,
        )
            .chain()
            .before(despawn_needed),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (
            check_needs_saved.in_set(SavingSystemSet::BeginSaving),
            default_save.in_set(SavingSystemSet::DoSaving),
            create_entity_ids.in_set(SavingSystemSet::CreateEntityIds),
            done_saving.in_set(SavingSystemSet::DoneSaving),
        ),
    );

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            BlueprintingSystemSet::BeginBlueprinting,
            BlueprintingSystemSet::DoBlueprinting,
            BlueprintingSystemSet::DoneBlueprinting,
        )
            .chain()
            .before(SavingSystemSet::BeginSaving),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (
            // Logic
            check_needs_blueprinted.in_set(BlueprintingSystemSet::BeginBlueprinting),
            done_blueprinting.in_set(BlueprintingSystemSet::DoneBlueprinting),
        ),
    );
}
