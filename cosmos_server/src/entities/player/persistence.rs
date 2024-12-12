use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy::prelude::*;
use cosmos_core::{
    entities::player::Player,
    persistence::LoadingDistance,
    physics::location::{Location, Sector},
};
use renet2::ClientId;
use serde::{Deserialize, Serialize};

use crate::persistence::{
    loading::{LoadingSystemSet, NeedsLoaded, LOADING_SCHEDULE},
    saving::{calculate_sfi, NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
    EntityId, SaveFileIdentifier, SerializedData,
};

#[derive(Debug, Serialize, Deserialize)]
struct PlayerIdentifier {
    entity_id: EntityId,
    sector: Sector,
    sfi: SaveFileIdentifier,
}

#[derive(Component)]
pub struct LoadPlayer {
    name: String,
    client_id: ClientId,
}

fn generate_player_file_id(player_name: &str) -> String {
    let mut hasher = DefaultHasher::default();
    player_name.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{hash}.json")
}

const PLAYER_LINK_PATH: &'static str = "world/players";

/// Creates a file that points the player's name to their respective data file.
fn save_player_link(
    q_parent: Query<&Parent>,
    q_entity_id: Query<&EntityId>,
    q_player_needs_saved: Query<(Entity, &EntityId, &Player, &Location), With<NeedsSaved>>,
    q_serialized_data: Query<(&SerializedData, &EntityId, Option<&LoadingDistance>)>,
) {
    for (entity, e_id, player, loc) in q_player_needs_saved.iter() {
        info!("Saving player {player:?}");
        let _ = fs::create_dir_all(&PLAYER_LINK_PATH);

        let sfi = calculate_sfi(entity, &q_parent, &q_entity_id, &q_serialized_data).expect("Missing save file identifier for player!");

        let player_identifier = PlayerIdentifier {
            sector: loc.sector(),
            entity_id: e_id.clone(),
            sfi,
        };

        let json_data = serde_json::to_string(&player_identifier).expect("Failed to create json");

        let player_file_name = generate_player_file_id(player.name());
        fs::write(format!("{PLAYER_LINK_PATH}/{player_file_name}"), json_data).expect("Failed to save player!!!");
    }
}

fn load_player(mut commands: Commands, q_player_needs_loaded: Query<(Entity, &LoadPlayer)>) {
    for (ent, load_player) in q_player_needs_loaded.iter() {
        let player_file_name = generate_player_file_id(&load_player.name);

        let Ok(data) = fs::read(format!("{PLAYER_LINK_PATH}/{player_file_name}")) else {
            continue;
        };

        let player_identifier = serde_json::from_slice::<PlayerIdentifier>(&data)
            .unwrap_or_else(|e| panic!("Invalid json data for player {player_file_name}\n{e:?}"));

        // Ensure the player's parents are also being loaded
        let mut cur_sfi = &player_identifier.sfi;
        while let Some(sfi) = cur_sfi.get_parent() {
            cur_sfi = sfi;
            commands.spawn((NeedsLoaded, sfi.clone(), sfi.entity_id().expect("Missing Entity Id!").clone()));
        }

        commands.entity(ent).insert((
            NeedsLoaded,
            player_identifier.entity_id,
            player_identifier.sfi,
            Player::new(load_player.name.clone(), load_player.client_id),
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        SAVING_SCHEDULE,
        save_player_link
            .after(SavingSystemSet::CreateEntityIds)
            .before(SavingSystemSet::DoneSaving),
    );
    app.add_systems(LOADING_SCHEDULE, load_player.in_set(LoadingSystemSet::BeginLoading));
}
