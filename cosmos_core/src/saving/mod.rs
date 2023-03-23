use std::fs;

use bevy::{
    prelude::{
        App, Commands, Component, CoreSet, Entity, IntoSystemConfig, Query, Transform, With,
        Without,
    },
    reflect::Reflect,
    utils::HashMap,
};
use bevy_rapier3d::prelude::Velocity;
use serde::{Deserialize, Serialize};

use crate::{physics::location::Location, structure::Structure};

#[derive(Component, Debug, Reflect, Serialize, Deserialize, PartialEq, Eq, Clone, Copy)]
pub struct EntityId(u128);

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
    pub fn save(&mut self, data_id: String, data: Vec<u8>) {
        self.save_data.insert(data_id, data);
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

/// Only used as a label - probably a better way
pub fn begin_saving() {}

pub fn done_saving(
    query: Query<(Entity, &SerializedData), With<NeedsSaved>>,
    mut commands: Commands,
) {
    for (entity, sd) in query.iter() {
        let serialized = bincode::serialize(&sd.save_data).unwrap();

        let directory = sd
            .location
            .map(|loc| format!("{}_{}_{}/", loc.sector_x, loc.sector_y, loc.sector_z))
            .unwrap_or("nowhere/".into());

        commands
            .entity(entity)
            .remove::<NeedsSaved>()
            .remove::<SerializedData>();
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(check_needs_saved)
        .add_system(begin_saving.in_base_set(CoreSet::Last));
}

fn default_save(mut query: Query<(&mut SerializedData, Option<&Location>, Option<&Velocity>)>) {}
