use std::fs;

use bevy::{
    prelude::{App, Commands, Component, CoreSet, Entity, IntoSystemConfig, Query, With, Without},
    reflect::Reflect,
};
use bevy_rapier3d::prelude::Velocity;

use cosmos_core::physics::location::Location;

use super::{get_save_file_path, EntityId, SerializedData};

#[derive(Debug, Component)]
pub struct SaveFileIdentifier {
    pub sector: Option<(i64, i64, i64)>,
    pub entity_id: EntityId,
}

#[derive(Component, Debug, Reflect)]
pub struct NeedsLoaded;

fn check_needs_loaded(
    query: Query<(Entity, &SaveFileIdentifier), (Without<SerializedData>, With<NeedsLoaded>)>,
    mut commands: Commands,
) {
    for (ent, nl) in query.iter() {
        let path = get_save_file_path(nl.sector.clone(), &nl.entity_id);
        let Ok(data) = fs::read(&path) else {
            eprintln!("Error reading file at {path}");
            continue;
        };

        let serialized_data: SerializedData =
            bincode::deserialize(&data).expect(&format!("Error deserializing data for {path}"));

        commands.entity(ent).insert(serialized_data);
    }
}

pub fn begin_loading() {}

pub fn done_loading(query: Query<Entity, With<NeedsLoaded>>, mut commands: Commands) {
    for ent in query.iter() {
        commands
            .entity(ent)
            .remove::<NeedsLoaded>()
            .remove::<SerializedData>()
            .remove::<SaveFileIdentifier>();
    }
}

fn default_load(
    query: Query<(Entity, &SerializedData), With<NeedsLoaded>>,
    mut commands: Commands,
) {
    for (ent, sd) in query.iter() {
        let mut ecmds = commands.entity(ent);

        if let Some(location) = sd.deserialize_data::<Location>("cosmos:location") {
            ecmds.insert(location);
        }
        if let Some(velocity) = sd.deserialize_data::<Velocity>("cosmos:velocity") {
            ecmds.insert(velocity);
        }
    }
}

pub(crate) fn register(app: &mut App) {
    app.add_system(check_needs_loaded)
        // Put all loading-related systems after this
        .add_system(begin_loading.in_base_set(CoreSet::First))
        // Put all loading-related systems before this
        .add_system(done_loading.after(begin_loading))
        // Like this:
        .add_system(default_load.after(begin_loading).before(done_loading));
}
