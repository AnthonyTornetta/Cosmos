//! This handles the saving of different things in the world, such as planets & ships
//!
//! To add your own saving event, add a system after `begin_saving` and before `done_saving`.
//!
//! Use the query: `Query<(Entity, &SerializedData), With<NeedsSaved>>` to get all the data that will need
//! loaded. From there, you can add any components necessary to the entity to fully load it in.
//!
//! See [`saving::default_save`] for an example.

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    ecs::{
        NeedsDespawned,
        data::{DataEntities, DataFor},
        despawn_needed,
    },
    entities::player::Player,
    netty::cosmos_encoder,
    persistence::LoadingDistance,
    physics::location::Location,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    io::{self, ErrorKind},
};

use crate::persistence::make_persistent::{PersistentComponent, make_persistent};

use super::{EntityId, PreviousSaveFileIdentifier, SaveFileIdentifier, SaveFileIdentifierType, SectorsCache, SerializedData};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
/// This system set is for when entities are being saved normally - NOT FOR A BLUEPRINT (use [`BlueprintingSystemSet`] for that.)
pub enum SavingSystemSet {
    /// Marks all entities that can be saved with [`ShouldBeSaved`].
    MarkSavable,
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

/// Denotes that this entity should not be saved, even if marked to has the [`NeedsSaved`] component.
#[derive(Component, Debug, Default, Reflect)]
pub struct NeverSave;

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
    q_parent: Query<&ChildOf, Or<(Without<SerializedData>, Without<NeedsSaved>)>>,
    q_needs_serialized_data: Query<(Entity, Option<&ChildOf>), (With<NeedsSaved>, Without<NeverSave>, Without<SerializedData>)>,
    mut commands: Commands,
) {
    for (ent, mut parent) in q_needs_serialized_data.iter() {
        commands.entity(ent).insert(SerializedData::default());

        // If something that needs saved has parents, we must propagate it up to work properly.
        while let Some(p) = parent {
            let ent = p.parent();
            commands.entity(ent).insert((SerializedData::default(), NeedsSaved));
            parent = q_parent.get(ent).ok();
        }
    }
}

#[derive(Serialize, Deserialize)]
/// Internal component (Serialized version of [`DataFor`])
///
/// Only public because the interface requires it to be.
pub struct DataFlag(EntityId);

impl PersistentComponent for DataFor {
    type SaveType = DataFlag;

    fn convert_to_save_type<'a>(
        &'a self,
        q_entity_ids: &Query<&EntityId>,
    ) -> Option<cosmos_core::utils::ownership::MaybeOwned<'a, Self::SaveType>> {
        Some(DataFlag(q_entity_ids.get(self.0).ok().copied()?).into())
    }

    fn convert_from_save_type(saved_type: Self::SaveType, entity_id_manager: &super::make_persistent::EntityIdManager) -> Option<Self> {
        entity_id_manager.entity_from_entity_id(&saved_type.0).map(Self)
    }
}

fn save_data_entities(
    mut commands: Commands,
    q_data_ents: Query<&DataEntities>,
    q_data_ents_need_saved: Query<&DataEntities, With<NeedsSaved>>,
) {
    for de in q_data_ents_need_saved.iter() {
        for ent in de.iter() {
            rec_save_des(ent, &q_data_ents, &mut commands);
        }
    }
}

fn rec_save_des(ent: Entity, q_data_ents: &Query<&DataEntities>, commands: &mut Commands) {
    commands.entity(ent).insert(NeedsSaved);
    if let Ok(data_ents) = q_data_ents.get(ent) {
        for de in data_ents.iter() {
            rec_save_des(de, q_data_ents, commands);
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
fn save_blueprint(data: &SerializedData, needs_blueprinted: &NeedsBlueprinted, log_name: &str) -> std::io::Result<()> {
    if let Err(e) = fs::create_dir("blueprints") {
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

    info!("Finished blueprinting {log_name}");

    Ok(())
}

/// Put all systems that add data to blueprinted entities before this and after `begin_blueprinting`
fn done_blueprinting(
    mut query: Query<(Entity, &mut SerializedData, &NeedsBlueprinted, Option<&NeedsSaved>, Option<&Name>)>,
    mut commands: Commands,
) {
    for (entity, mut serialized_data, needs_blueprinted, needs_saved, name) in query.iter_mut() {
        let bp_name = name.map(|n| format!("{n} ({entity:?})")).unwrap_or(format!("{entity:?}"));
        save_blueprint(&serialized_data, needs_blueprinted, &bp_name)
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

fn ensure_data_entities_have_correct_parents(
    q_data_ent: Query<(Entity, &DataFor, Option<&ChildOf>), Changed<DataFor>>,
    mut commands: Commands,
) {
    for (ent, data_for, child_of) in q_data_ent.iter() {
        if Some(data_for.0) != child_of.map(|x| x.parent()) {
            commands.entity(ent).insert(ChildOf(data_for.0));
        }
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
        (With<NeedsSaved>, Without<NeverSave>),
    >,
    q_parent: Query<&ChildOf>,
    q_entity_id: Query<&EntityId>,
    q_serialized_data: Query<(&SerializedData, &EntityId, Option<&LoadingDistance>)>,
    dead_saves_query: Query<&PreviousSaveFileIdentifier, (With<NeedsDespawned>, Without<NeedsSaved>)>,
    mut sectors_cache: ResMut<SectorsCache>,
    mut commands: Commands,
) {
    for dead_save in dead_saves_query.iter() {
        let path = dead_save.0.get_save_file_path();
        if fs::exists(&path).unwrap_or(false) {
            if let Err(e) = fs::remove_file(&path) {
                error!("Error deleting old save file @ {path}! - {e:?}");
            }

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

        if matches!(&save_file_identifier.identifier_type, SaveFileIdentifierType::Base(_, _, _))
            && let Some(loc) = sd.location
        {
            sectors_cache.insert(loc.sector(), *entity_id, loading_distance.map(|ld| ld.load_distance()));
        }
    }
}

/// This is in a bad spot, and should be moved.
pub(crate) fn calculate_sfi(
    entity: Entity,
    q_parent: &Query<&ChildOf>,
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
            *entity_id,
            loading_distance.map(|ld| ld.load_distance()),
        ));
    };

    let Ok(entity_id) = q_entity_id.get(entity) else {
        error!("Missing entity id for {entity:?} - cannot generate save file identifier.");
        return None;
    };

    let Some(parent_sfi) = calculate_sfi(parent.parent(), q_parent, q_entity_id, q_serialized_data) else {
        error!("Could not calculate parent save file identifier - not saving {entity:?}");
        return None;
    };

    Some(SaveFileIdentifier::sub_entity(parent_sfi, *entity_id))
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

#[derive(Component)]
struct ShouldBeSaved;

fn mark_savable_entities(
    mut commands: Commands,
    q_savable: Query<Entity, (Without<ShouldBeSaved>, Or<((With<Location>, With<LoadingDistance>), With<DataFor>)>)>,
) {
    for ent in q_savable.iter() {
        commands.entity(ent).insert(ShouldBeSaved);
    }
}

/// The schedule saving takes place in - this may change in the future
pub const SAVING_SCHEDULE: First = First;

pub(super) fn register(app: &mut App) {
    make_persistent::<DataFor>(app);

    app.configure_sets(
        SAVING_SCHEDULE,
        (
            SavingSystemSet::MarkSavable,
            SavingSystemSet::BeginSaving,
            SavingSystemSet::CreateEntityIds,
            SavingSystemSet::DoSaving,
            SavingSystemSet::DoneSaving,
        )
            .chain()
            .before(despawn_needed),
    )
    .add_systems(
        SAVING_SCHEDULE,
        (
            mark_savable_entities.in_set(SavingSystemSet::MarkSavable),
            (save_data_entities, check_needs_saved).chain().in_set(SavingSystemSet::BeginSaving),
            default_save.in_set(SavingSystemSet::DoSaving),
            create_entity_ids.in_set(SavingSystemSet::CreateEntityIds),
            (ensure_data_entities_have_correct_parents, done_saving).in_set(SavingSystemSet::DoneSaving),
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
