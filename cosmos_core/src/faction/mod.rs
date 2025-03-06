use std::fs;

use bevy::{prelude::*, utils::HashMap};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{
    entities::EntityId,
    netty::{
        cosmos_encoder,
        sync::resources::{sync_resource, SyncableResource},
        system_sets::NetworkingSystemsSet,
    },
    state::GameState,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
pub enum FactionRelation {
    Ally,
    #[default]
    Neutral,
    Enemy,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct FactionSettings {
    /// If this is true, this faction will automatically be at war with any neutral faction.
    pub default_enemy: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Reflect)]
pub struct Faction {
    id: FactionId,
    name: String,
    players: Vec<EntityId>,
    relationships: HashMap<FactionId, FactionRelation>,
    settings: FactionSettings,
}

impl Faction {
    pub fn new(
        name: String,
        players: Vec<EntityId>,
        relationships: HashMap<FactionId, FactionRelation>,
        settings: FactionSettings,
    ) -> Self {
        Self {
            id: FactionId::generate_new(),
            name,
            players,
            relationships,
            settings,
        }
    }
}

#[derive(Clone, Copy, Component, Debug, PartialEq, Eq, Hash, Serialize, Deserialize, Reflect, Default)]
/// Links this entity to the faction its apart of.
pub struct FactionId(Uuid);

impl FactionId {
    pub fn generate_new() -> Self {
        Self(Uuid::new_v4())
    }
}

#[derive(Resource, Reflect, Clone, Serialize, Deserialize, Debug, Default)]
pub struct Factions(HashMap<FactionId, Faction>);

impl Factions {
    pub fn add_new_faction(&mut self, faction: Faction) {
        self.0.insert(faction.id, faction);
    }
}

impl SyncableResource for Factions {
    fn unlocalized_name() -> &'static str {
        "cosmos:factions"
    }
}

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

pub(super) fn register(app: &mut App) {
    sync_resource::<Factions>(app);

    app.add_systems(
        Update,
        save_factions_on_change
            .in_set(NetworkingSystemsSet::Between)
            .run_if(in_state(GameState::Playing)),
    )
    .add_systems(OnExit(GameState::PreLoading), load_factions);

    app.register_type::<FactionRelation>()
        .register_type::<Faction>()
        .register_type::<FactionId>();
}
