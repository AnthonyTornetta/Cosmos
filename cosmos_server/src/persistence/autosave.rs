//! Performs regular autosaves of the world

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::{
    ecs::NeedsDespawned, entities::player::Player, netty::system_sets::NetworkingSystemsSet, persistence::LoadingDistance,
    physics::location::Location,
};

use super::{backup::CreateWorldBackup, saving::NeedsSaved};

const AUTOSAVE_INTERVAL: Duration = Duration::from_mins(5);

#[derive(Event, Default)]
/// Send this event to save every savable entity in the game
pub struct SaveEverything;

fn backup_before_saving(mut evw_create_backup: EventWriter<CreateWorldBackup>, mut evr_save_everything: EventReader<SaveEverything>) {
    if evr_save_everything.is_empty() {
        return;
    }
    evr_save_everything.clear();
    evw_create_backup.send_default();
}

fn save_everything(
    mut commands: Commands,
    q_needs_saved: Query<Entity, (Without<NeedsDespawned>, With<Location>, With<LoadingDistance>)>,
    mut evr_save_everything: EventReader<SaveEverything>,
) {
    if evr_save_everything.is_empty() {
        return;
    };
    evr_save_everything.clear();

    info!("Saving all entities! Expect some lag.");
    for entity in q_needs_saved.iter() {
        commands.entity(entity).insert(NeedsSaved);
    }
}

fn trigger_autosave(mut evw_create_backup: EventWriter<SaveEverything>, q_players: Query<(), With<Player>>) {
    if q_players.is_empty() {
        return;
    }

    info!("Triggering autosave");
    evw_create_backup.send_default();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Last,
        (
            trigger_autosave.run_if(on_timer(AUTOSAVE_INTERVAL)),
            backup_before_saving,
            save_everything,
        )
            .in_set(NetworkingSystemsSet::SyncComponents)
            .chain(),
    )
    .add_event::<SaveEverything>();
}
