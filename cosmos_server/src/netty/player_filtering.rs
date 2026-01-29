//! Dictating which players can join the server

use std::fs;

use bevy::{
    platform::collections::{HashMap, HashSet},
    prelude::*,
};
use bevy_renet::steam::steamworks::SteamId;
use cosmos_core::netty::{invite::InviteFriendToServerMessage, sync::events::server_event::NettyMessageReceived};
use serde::{Deserialize, Serialize};

use crate::plugin::server_plugin::ServerType;

#[derive(Debug, Clone, Serialize, Deserialize)]
/// The reason this player was blacklisted
pub struct BlacklistedReason {
    message: String,
}

impl BlacklistedReason {
    /// Creates a reason with plaintext
    pub fn new(message: String) -> Self {
        Self { message }
    }
}

#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
/// Everyone that's allowed to connect to the server
///
/// If this resource is not present, there is assumed to be no whitelist (public access).
///
/// For public servers:
///     - A whitelist is NOT automatically created, and will only be present if the player creates
///     one (via commands or whitelist.json in root)
///
/// For private (singleplayer/friends) worlds:
///     - A whitelist is automatically created that only contains the world's owner as a temporary
///     entry.
///     - When the player invites a friend, that friend is automatically added as a temporary
///     entry to the server.
pub struct PlayerWhitelist {
    fixed: HashSet<SteamId>,
    #[serde(skip)]
    temporary: HashSet<SteamId>,
    #[serde(skip)]
    should_save: bool,
}

impl PlayerWhitelist {
    /// Adds a player to this whitelist permenantly
    pub fn add_player(&mut self, id: SteamId) {
        self.fixed.insert(id);
        self.should_save = true;
    }

    /// Adds a player to this whitelist for as long as the server is running
    pub fn add_player_temporary(&mut self, id: SteamId) {
        self.temporary.insert(id);
    }

    /// Checks if this id is on the list (permenantly or temporarily)
    pub fn contains_player(&self, id: &SteamId) -> bool {
        self.fixed.contains(id) || self.temporary.contains(id)
    }

    /// Removes a player from this whitelist (both permenantly and temporarily)
    pub fn remove_player(&mut self, id: &SteamId) {
        self.fixed.remove(id);
        self.temporary.remove(id);
    }
}

/// A list of players that are not allowed to connect to the server
///
/// This resource will always be present, even if noone is blacklisted
#[derive(Resource, Debug, Clone, Serialize, Deserialize, Default)]
pub struct PlayerBlacklist(HashMap<SteamId, BlacklistEntry>);

#[derive(Debug, Clone, Serialize, Deserialize)]
struct BlacklistEntry {
    name: String,
    reason: Option<BlacklistedReason>,
}

impl PlayerBlacklist {
    /// Marks this player as not being able to connect. Does NOT disconnect this player if they are
    /// currently connected.
    pub fn add_player(&mut self, id: SteamId, name: String, reason: Option<BlacklistedReason>) {
        self.0.insert(id, BlacklistEntry { name, reason });
    }

    /// Checks if this player should not be able to join
    pub fn contains_player(&self, id: &SteamId) -> bool {
        self.0.contains_key(id)
    }

    /// Checks if this player should not be able to join
    pub fn get_player_by_name(&self, name: &str) -> Option<SteamId> {
        let name_lower = name.to_lowercase();
        self.0.iter().find(|x| x.1.name.to_lowercase() == name_lower).map(|x| *x.0)
    }

    /// If this player is blacklisted AND a reason was given, returns that reason.
    pub fn get_reason_for_player(&self, id: &SteamId) -> Option<&BlacklistedReason> {
        self.0.get(id).and_then(|x| x.reason.as_ref())
    }

    /// Removes a player from the blacklist - they will now be able to connect again
    pub fn remove_player(&mut self, id: &SteamId) {
        self.0.remove(id);
    }
}

fn load_player_whitelist_and_blacklist(mut commands: Commands) {
    let _ = try {
        let whitelist = fs::read("whitelist.json")?;
        let mut whitelist =
            serde_json::from_slice::<PlayerWhitelist>(&whitelist).unwrap_or_else(|e| panic!("Failed to parse whitelist - {e:?}"));
        whitelist.should_save = true;
        commands.insert_resource(whitelist);
    };

    let blacklist = if let Ok(blacklist) = fs::read("blacklist.json") {
        serde_json::from_slice::<PlayerBlacklist>(&blacklist).unwrap_or_else(|e| panic!("Failed to parse blacklist - {e:?}"))
    } else {
        PlayerBlacklist::default()
    };

    commands.insert_resource(blacklist);
}

fn save_whitelist(whitelist: Res<PlayerWhitelist>) {
    if !whitelist.should_save {
        return;
    }

    if let Err(e) = fs::write("whitelist.json", serde_json::to_string_pretty(whitelist.as_ref()).unwrap()) {
        error!("Couldn't save whitelist.json - {e:?}");
    }
}

fn save_blacklist(blacklist: Res<PlayerBlacklist>) {
    if let Err(e) = fs::write("blacklist.json", serde_json::to_string_pretty(blacklist.as_ref()).unwrap()) {
        error!("Couldn't save blacklist.json - {e:?}");
    }
}

fn on_invite_friend(
    mut nevr: MessageReader<NettyMessageReceived<InviteFriendToServerMessage>>,
    server_type: Res<ServerType>,
    mut whitelist: Option<ResMut<PlayerWhitelist>>, // steam_user: Res<ServerSteamClient>,
) {
    for ev in nevr.read() {
        match &*server_type {
            ServerType::Local => {
                // do we want this?
                // if steam_user.client().user().steam_id() != SteamId::from_raw(ev.client_id) {
                //     // Only the owner can invite
                //     return;
                // }
            }
            ServerType::Dedicated { port: _ } => {
                // dedicated servers don't need to do this.
                return;
            }
        }
        if let Some(whitelist) = whitelist.as_mut() {
            whitelist.add_player_temporary(ev.friend_id);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Startup, load_player_whitelist_and_blacklist)
        .add_systems(FixedUpdate, on_invite_friend)
        .add_systems(FixedUpdate, save_whitelist.run_if(resource_exists_and_changed::<PlayerWhitelist>))
        .add_systems(FixedUpdate, save_blacklist.run_if(resource_exists_and_changed::<PlayerBlacklist>));
}
