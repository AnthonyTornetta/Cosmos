use bevy::prelude::*;
use cosmos_core::{
    faction::{Faction, FactionId, FactionSettings, Factions},
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet},
    state::GameState,
};
use std::fs;

use crate::persistence::make_persistent::{make_persistent, DefaultPersistentComponent};

fn load_factions(mut commands: Commands) {
    let factions = if let Some(data) = fs::read("world/factions.bin").ok() {
        // We want to panic if something is corrupted
        let factions = cosmos_encoder::deserialize::<Factions>(&data).expect("Failed to deserialize faction data in world/factions.bin.");

        factions
    } else {
        info!("Generating factions!");

        let mut factions = Factions::default();

        factions.add_new_faction(Faction::new(
            "Pirate".into(),
            vec![],
            Default::default(),
            FactionSettings { default_enemy: true },
        ));

        factions.add_new_faction(Faction::new(
            "Merchant Federation".into(),
            vec![],
            Default::default(),
            FactionSettings { default_enemy: false },
        ));

        factions
    };

    commands.insert_resource(factions);
}

fn save_factions_on_change(factions: Res<Factions>) {
    fs::write("world/factions.bin", cosmos_encoder::serialize(factions.as_ref())).expect("Failed to save factions.");
}

impl DefaultPersistentComponent for FactionId {}

pub(super) fn register(app: &mut App) {
    make_persistent::<FactionId>(app);

    app.add_systems(
        Update,
        save_factions_on_change
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnEnter(GameState::PostLoading), load_factions);
}
