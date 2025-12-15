//! Performs regular autosaves of the world

use std::time::Duration;

use bevy::{prelude::*, time::common_conditions::on_timer};
use cosmos_core::entities::player::Player;

use crate::persistence::{
    backup::BackupSystemSet,
    saving::{SAVING_SCHEDULE, SavingSystemSet, ShouldBeSaved},
};

use super::{backup::CreateWorldBackup, saving::NeedsSaved};

const AUTOSAVE_INTERVAL: Duration = Duration::from_mins(10);

#[derive(Message, Default)]
/// Send this event to save every savable entity in the game
pub struct SaveEverything;

fn backup_before_saving(mut evw_create_backup: MessageWriter<CreateWorldBackup>, mut evr_save_everything: MessageReader<SaveEverything>) {
    if evr_save_everything.is_empty() {
        return;
    }
    evr_save_everything.clear();
    evw_create_backup.write_default();
}

fn save_everything(
    mut commands: Commands,
    q_needs_saved: Query<Entity, With<ShouldBeSaved>>,
    mut evr_save_everything: MessageReader<SaveEverything>,
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

fn trigger_autosave(mut evw_create_backup: MessageWriter<SaveEverything>, q_players: Query<(), With<Player>>) {
    if q_players.is_empty() {
        return;
    }

    info!("Triggering autosave");
    evw_create_backup.write_default();
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        SAVING_SCHEDULE,
        (
            (trigger_autosave.run_if(on_timer(AUTOSAVE_INTERVAL)), backup_before_saving)
                .chain()
                .before(BackupSystemSet::PerformBackup),
            save_everything
                .after(SavingSystemSet::MarkSavable)
                .before(SavingSystemSet::BeginSaving)
                .after(BackupSystemSet::PerformBackup)
                .chain(),
        )
            .chain(),
    )
    .add_message::<SaveEverything>();
}
