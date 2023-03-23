use bevy::{
    prelude::{App, Commands, Component, CoreSet, Entity, IntoSystemConfig, Query, With, Without},
    reflect::Reflect,
    utils::HashMap,
};
use bevy_rapier3d::prelude::Velocity;
use rand::{distributions::Alphanumeric, Rng};
use serde::{Deserialize, Serialize};
use std::fs;

use crate::physics::location::Location;

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone)]
pub struct EntityId(String);

#[derive(Component, Debug, Default, Reflect, Serialize, Deserialize)]
pub struct SerializedData {
    save_data: HashMap<String, Vec<u8>>,

    location: Option<Location>,
}

#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsSaved;

#[derive(Component, Debug, Default, Reflect)]
pub struct NeedsLoaded;

impl SerializedData {
    /// Saves the data to that data id. Will overwrite any existing data at that id.
    pub fn save(&mut self, data_id: impl Into<String>, data: Vec<u8>) {
        self.save_data.insert(data_id.into(), data);
    }

    /// Calls `bincode::serialize` on the passed in data.
    /// Then sends that data into the `save` method, with the given data id.
    pub fn serialize_data(&mut self, data_id: impl Into<String>, data: &impl Serialize) {
        self.save(
            data_id,
            bincode::serialize(data).expect("Error serializing data!"),
        );
    }
}

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
    query: Query<(Entity, &SerializedData, Option<&EntityId>), With<NeedsSaved>>,
    mut commands: Commands,
) {
    for (entity, sd, entity_id) in query.iter() {
        commands
            .entity(entity)
            .remove::<NeedsSaved>()
            .remove::<SerializedData>();

        println!("GOT AMIGO!");

        let serialized = bincode::serialize(&sd.save_data).unwrap();

        let directory = sd
            .location
            .map(|loc| format!("{}_{}_{}/", loc.sector_x, loc.sector_y, loc.sector_z))
            .unwrap_or("nowhere/".into());

        let full_directory = format!("world/{directory}");

        if let Err(e) = fs::create_dir_all(&full_directory) {
            eprintln!("{e}");
            continue;
        }

        let file_name: String = if let Some(id) = entity_id {
            id.0.clone()
        } else {
            let res: String = rand::thread_rng()
                .sample_iter(&Alphanumeric)
                .take(64)
                .map(char::from)
                .collect();

            commands.entity(entity).insert(EntityId(res.clone()));

            res
        };

        println!("WRITING FILE!");
        if let Err(e) = fs::write(format!("{full_directory}{file_name}.cent"), serialized) {
            eprintln!("{e}");
            continue;
        }
    }
}

fn default_save(
    mut query: Query<(&mut SerializedData, Option<&Location>, Option<&Velocity>), With<NeedsSaved>>,
) {
    for (mut data, loc, vel) in query.iter_mut() {
        println!("Saving base data!");
        if let Some(loc) = loc {
            data.location = Some(*loc);
        }

        if let Some(vel) = vel {
            data.serialize_data("cosmos:velocity", vel);
        }
    }
}

// fn add_entity_ids(
//     query: Query<Entity, (With<Location>, Without<EntityId>)>,
//     mut commands: Commands,
// ) {
//     for entity in query.iter() {
//         commands.entity(entity).insert(EntityId(5));
//     }
// }

pub(crate) fn register(app: &mut App) {
    app.add_system(check_needs_saved)
        // Put all saving-related systems after this
        .add_system(begin_saving.in_base_set(CoreSet::First))
        // Put all saving-related systems before this
        .add_system(done_saving.after(begin_saving))
        // Like this:
        .add_system(default_save.after(begin_saving).before(done_saving));
}
