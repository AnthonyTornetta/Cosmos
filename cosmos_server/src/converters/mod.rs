//! Updates world save files across versions

use std::fs;

use bevy::prelude::*;
use cosmos_core::{netty::cosmos_encoder, state::GameState};
use serde::{Deserialize, Serialize};

mod bincode_convert;

#[derive(PartialEq, Eq, PartialOrd, Ord, Debug, Serialize, Deserialize)]
enum SaveVersion {
    Old,
    Alpha0_0_8,
}

const LATEST_VERSION: SaveVersion = SaveVersion::Alpha0_0_8;

fn close_server_after_ticks(mut ticks: Local<u32>, mut evw_app_exit: EventWriter<AppExit>) {
    if *ticks == 10 {
        info!("Closing server after converting files.");
        evw_app_exit.write(AppExit::Success);
    } else {
        *ticks += 1;
    }
}

pub(super) fn register(app: &mut App) {
    let version = fs::read(".version").unwrap_or_default();
    if version.is_empty() {
        fs::write(".version", cosmos_encoder::serialize_uncompressed(&LATEST_VERSION)).expect("Failed to write version ID");
        return;
    }

    let version = cosmos_encoder::deserialize_uncompressed::<SaveVersion>(&version).unwrap_or(LATEST_VERSION);

    if version == LATEST_VERSION {
        return;
    }

    info!("Current save version is out of date - converting files.");

    bincode_convert::register(app, version);

    app.add_systems(Update, close_server_after_ticks.run_if(in_state(GameState::Playing)));

    fs::write(".version", cosmos_encoder::serialize_uncompressed(&LATEST_VERSION)).unwrap();
}
