//! This handles the loading of different things in the world, such as planets & ships
//!
//! To add your own loading event, add a system after `begin_loading` and before `done_loading`.
//!
//! Use the query: `Query<(Entity, &SerializedData), With<NeedsLoaded>>` to get all the data that will need
//! loaded. From there, you can add any components necessary to the entity to fully load it in.
//!
//! See [`default_load`] for an example.

use std::fs;

use bevy::{
    ecs::schedule::{IntoSystemConfigs, IntoSystemSetConfigs, SystemSet},
    hierarchy::BuildChildren,
    log::{error, warn},
    prelude::{App, Commands, Component, Entity, Quat, Query, Update, With, Without},
    reflect::Reflect,
};
use bevy_rapier3d::prelude::Velocity;

use cosmos_core::{
    ecs::{bundles::BundleStartingRotation, NeedsDespawned},
    netty::cosmos_encoder,
    persistence::LoadingDistance,
    physics::location::Location,
    structure::loading::StructureLoadingSet,
};

use super::{EntityId, SaveFileIdentifier, SaveFileIdentifierType, SerializedData};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Put anything related to loading entities in from serialized data into this set
pub enum LoadingSystemSet {
    /// Sets up the loading entities
    BeginLoading,
    /// Put all your loading logic in here
    DoLoading,
    /// Removes all unneeded components
    DoneLoading,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// Put anything related to loading blueprinted entities in from serialized data into this set
pub enum LoadingBlueprintSystemSet {
    /// Sets up the loading entities
    BeginLoadingBlueprints,
    /// Put all your blueprint loading logic in here
    DoLoadingBlueprints,
    /// Removes all unneeded components
    DoneLoadingBlueprints,
}

#[derive(Component, Debug, Reflect)]
/// An entity that currently has this is currently in the process of being loaded
pub struct NeedsLoaded;

#[derive(Component, Debug, Reflect)]
/// An entity that currently has this is currently in the process of being loaded
pub struct NeedsBlueprintLoaded {
    /// The location this blueprint should be spawned at
    pub spawn_at: Location,
    /// The starting rotation of the blueprint
    pub rotation: Quat,
    /// The file path of the blueprint
    pub path: String,
}

fn check_needs_loaded(
    q_entity_ids: Query<(Entity, &EntityId)>,
    q_sfis: Query<(Entity, &SaveFileIdentifier), (Without<SerializedData>, With<NeedsLoaded>)>,
    mut commands: Commands,
) {
    for (ent, nl) in q_sfis.iter() {
        let path = nl.get_save_file_path();
        let Ok(data) = fs::read(&path) else {
            warn!("Error reading file at '{path}'. Is it there?");
            commands.entity(ent).insert(NeedsDespawned);
            continue;
        };

        let serialized_data: SerializedData = cosmos_encoder::deserialize(&data).expect("Error deserializing data for {path}");

        match &nl.identifier_type {
            SaveFileIdentifierType::Base(entity_id, _, _) => {
                commands.entity(ent).insert(entity_id.clone());
            }
            SaveFileIdentifierType::SubEntity(base, entity_id) => {
                if let Some(looking_for_entity) = match &base.identifier_type {
                    SaveFileIdentifierType::Base(entity_id, _, _) => Some(entity_id),
                    SaveFileIdentifierType::SubEntity(_, entity_id) => Some(entity_id),
                    SaveFileIdentifierType::BelongsTo(_, _) => None,
                } {
                    let mut parent = None;
                    // Most often the parent will also be being loaded, so we have to search through the currently being loaded.
                    for (entity, sfi) in q_sfis.iter() {
                        match &sfi.identifier_type {
                            SaveFileIdentifierType::Base(entity_id, _, _) => {
                                if entity_id == looking_for_entity {
                                    parent = Some(entity);
                                    break;
                                }
                            }
                            SaveFileIdentifierType::SubEntity(_, entity_id) => {
                                if entity_id == looking_for_entity {
                                    parent = Some(entity);
                                    break;
                                }
                            }
                            // Not managed by this system, managed by whoever this belongs to
                            SaveFileIdentifierType::BelongsTo(_, _) => {}
                        }
                    }

                    if parent.is_none() {
                        if let Some((ent, _)) = q_entity_ids.iter().find(|(_, eid)| *eid == looking_for_entity) {
                            parent = Some(ent);
                        }
                    }

                    if let Some(parent) = parent {
                        commands.entity(ent).set_parent(parent);
                    } else {
                        warn!("Unable to find parent with entity id {looking_for_entity:?} for entity {ent:?}");
                    }
                }

                commands.entity(ent).insert(entity_id.clone());
            }
            // Not managed by this system, managed by whoever this belongs to
            SaveFileIdentifierType::BelongsTo(_, _) => {}
        }

        commands.entity(ent).insert(serialized_data);
    }
}

fn check_blueprint_needs_loaded(query: Query<(Entity, &NeedsBlueprintLoaded), Without<SerializedData>>, mut commands: Commands) {
    for (ent, blueprint_needs_loaded) in query.iter() {
        let path = &blueprint_needs_loaded.path;
        let Ok(data) = fs::read(path) else {
            error!("Error reading file at '{path}'. Is it there?");
            commands.entity(ent).insert(NeedsDespawned);
            continue;
        };

        let Ok(serialized_data) = cosmos_encoder::deserialize::<SerializedData>(&data) else {
            error!("Error deserializing data for {path}");
            continue;
        };

        commands.entity(ent).insert(serialized_data);
    }
}

/// To add your own loading event, add a system after `begin_loading` and before `done_loading`.
fn done_loading_blueprint(query: Query<Entity, With<NeedsBlueprintLoaded>>, mut commands: Commands) {
    for ent in query.iter() {
        commands.entity(ent).remove::<NeedsBlueprintLoaded>().remove::<SerializedData>();
    }
}

/// To add your own loading event, add a system after `begin_loading` and before `done_loading`.
fn done_loading(query: Query<Entity, With<NeedsLoaded>>, mut commands: Commands) {
    for ent in query.iter() {
        commands.entity(ent).remove::<NeedsLoaded>().remove::<SerializedData>();
    }
}

fn default_load(query: Query<(Entity, &SerializedData), With<NeedsLoaded>>, mut commands: Commands) {
    for (ent, sd) in query.iter() {
        let mut ecmds = commands.entity(ent);

        if let Some(location) = sd.deserialize_data::<Location>("cosmos:location") {
            ecmds.insert(location);
        }
        if let Some(velocity) = sd.deserialize_data::<Velocity>("cosmos:velocity") {
            ecmds.insert(velocity);
        }
        if let Some(loading_distance) = sd.deserialize_data::<LoadingDistance>("cosmos:loading_distance") {
            ecmds.insert(loading_distance);
        }
        if let Some(rotation) = sd.deserialize_data::<Quat>("cosmos:rotation") {
            ecmds.insert(BundleStartingRotation(rotation));
        }
    }
}

/// The schedule loading takes place in - this may change in the future
pub const LOADING_SCHEDULE: Update = Update;

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        LOADING_SCHEDULE,
        (
            LoadingSystemSet::BeginLoading,
            LoadingSystemSet::DoLoading.before(StructureLoadingSet::LoadStructure),
            LoadingSystemSet::DoneLoading.after(StructureLoadingSet::StructureLoaded),
        )
            .chain(),
    )
    .add_systems(
        LOADING_SCHEDULE,
        (
            check_needs_loaded.in_set(LoadingSystemSet::BeginLoading),
            default_load.in_set(LoadingSystemSet::DoLoading),
            done_loading.in_set(LoadingSystemSet::DoneLoading),
        ),
    );

    app.configure_sets(
        LOADING_SCHEDULE,
        (
            LoadingBlueprintSystemSet::BeginLoadingBlueprints,
            LoadingBlueprintSystemSet::DoLoadingBlueprints,
            LoadingBlueprintSystemSet::DoneLoadingBlueprints,
        )
            .chain()
            .before(LoadingSystemSet::BeginLoading),
    )
    .add_systems(
        LOADING_SCHEDULE,
        (
            // Logic
            check_blueprint_needs_loaded.in_set(LoadingBlueprintSystemSet::BeginLoadingBlueprints),
            done_loading_blueprint.in_set(LoadingBlueprintSystemSet::DoneLoadingBlueprints),
        ),
    );
}
