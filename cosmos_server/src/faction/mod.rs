//! Server faction logic

use bevy::prelude::*;
use cosmos_core::{
    ecs::sets::FixedUpdateSet,
    faction::{Faction, FactionId, FactionSettings, Factions},
    netty::cosmos_encoder,
    state::GameState,
};
use std::fs;

use crate::persistence::{
    WorldRoot,
    make_persistent::{DefaultPersistentComponent, make_persistent},
};

mod events;

fn load_factions(mut commands: Commands, world_root: Res<WorldRoot>) {
    let path = world_root.path_for("factions.bin");
    let factions = if let Ok(data) = fs::read(&path) {
        // We want to panic if something is corrupted
        cosmos_encoder::deserialize::<Factions>(&data).unwrap_or_else(|e| panic!("Failed to deserialize faction data in {path}. {e:?}"))
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

fn save_factions_on_change(factions: Res<Factions>, world_root: Res<WorldRoot>) {
    fs::write(world_root.path_for("factions.bin"), cosmos_encoder::serialize(factions.as_ref())).expect("Failed to save factions.");
}

impl DefaultPersistentComponent for FactionId {}

pub(super) fn register(app: &mut App) {
    make_persistent::<FactionId>(app);
    events::register(app);

    app.add_systems(
        FixedUpdate,
        save_factions_on_change
            .in_set(FixedUpdateSet::Main)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnEnter(GameState::PostLoading), load_factions);
}
