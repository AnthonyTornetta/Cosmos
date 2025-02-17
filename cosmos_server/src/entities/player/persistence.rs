//! Player persistence

use std::{
    fs,
    hash::{DefaultHasher, Hash, Hasher},
};

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use cosmos_core::{
    chat::ServerSendChatMessageEvent,
    economy::Credits,
    entities::player::{creative::Creative, Player},
    inventory::{itemstack::ItemShouldHaveData, Inventory},
    item::Item,
    netty::{
        cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server::ServerLobby,
        server_reliable_messages::ServerReliableMessages,
        sync::{events::server_event::NettyEventWriter, registry::server::SyncRegistriesEvent, ComponentSyncingSet},
        system_sets::NetworkingSystemsSet,
        NettyChannelServer,
    },
    persistence::LoadingDistance,
    physics::{
        location::{systems::Anchor, Location, LocationPhysicsSet, Sector, SetPosition},
        player_world::WorldWithin,
    },
    registry::{identifiable::Identifiable, Registry},
};
use renet2::{ClientId, RenetServer};
use serde::{Deserialize, Serialize};

use crate::{
    entities::player::spawn_player::find_new_player_location,
    netty::server_events::PlayerConnectedEvent,
    persistence::{
        loading::{LoadingSystemSet, NeedsLoaded, LOADING_SCHEDULE},
        saving::{calculate_sfi, NeedsSaved, SavingSystemSet, SAVING_SCHEDULE},
        EntityId, SaveFileIdentifier, SerializedData,
    },
    physics::assign_player_world,
    settings::ServerSettings,
    universe::generation::UniverseSystems,
};

use super::PlayerLooking;

#[derive(Debug, Serialize, Deserialize)]
struct PlayerIdentifier {
    location: Location,
    entity_id: EntityId,
    sector: Sector,
    sfi: SaveFileIdentifier,
}

#[derive(Component)]
/// Used to load a player into the game. If this player has joined the server before, their saved
/// data will be loaded. Otherwise, a new player with this information will be created.
pub struct LoadPlayer {
    /// The name of the player. This must be unique from all other players.
    pub name: String,
    /// The networking client id of the player. This is NOT used to identify their save data.
    pub client_id: ClientId,
}

fn generate_player_file_id(player_name: &str) -> String {
    let mut hasher = DefaultHasher::default();
    player_name.hash(&mut hasher);
    let hash = hasher.finish();
    format!("{hash}.json")
}

const PLAYER_LINK_PATH: &str = "world/players";

/// Creates a file that points the player's name to their respective data file.
fn save_player_link(
    q_parent: Query<&Parent>,
    q_entity_id: Query<&EntityId>,
    q_player_needs_saved: Query<(Entity, &EntityId, &Player, &Location), With<NeedsSaved>>,
    q_serialized_data: Query<(&SerializedData, &EntityId, Option<&LoadingDistance>)>,
) {
    for (entity, e_id, player, loc) in q_player_needs_saved.iter() {
        info!("Saving player {player:?} ({entity:?}) @ {loc}");
        let _ = fs::create_dir_all(PLAYER_LINK_PATH);

        let sfi = calculate_sfi(entity, &q_parent, &q_entity_id, &q_serialized_data).expect("Missing save file identifier for player!");

        let player_identifier = PlayerIdentifier {
            sector: loc.sector(),
            entity_id: e_id.clone(),
            sfi,
            location: *loc,
        };

        let json_data = serde_json::to_string(&player_identifier).expect("Failed to create json");

        let player_file_name = generate_player_file_id(player.name());
        fs::write(format!("{PLAYER_LINK_PATH}/{player_file_name}"), json_data).expect("Failed to save player!!!");
    }
}

fn load_player(
    mut commands: Commands,
    q_player_needs_loaded: Query<(Entity, &LoadPlayer)>,
    player_worlds: Query<(&Location, &WorldWithin, &RapierContextEntityLink), (With<Player>, Without<Parent>)>,
) {
    for (ent, load_player) in q_player_needs_loaded.iter() {
        let player_file_name = generate_player_file_id(&load_player.name);

        info!("Attempting to load player {}", load_player.name);
        let Ok(data) = fs::read(format!("{PLAYER_LINK_PATH}/{player_file_name}")) else {
            info!("No data found for {}", load_player.name);
            continue;
        };
        info!("Found data for {}. Loading now", load_player.name);

        let player_identifier = serde_json::from_slice::<PlayerIdentifier>(&data)
            .unwrap_or_else(|e| panic!("Invalid json data for player {player_file_name}\n{e:?}"));

        // Ensure the player's parents are also being loaded
        let mut cur_sfi = &player_identifier.sfi;
        while let Some(sfi) = cur_sfi.get_parent() {
            cur_sfi = sfi;
            commands.spawn((NeedsLoaded, sfi.clone(), sfi.entity_id().expect("Missing Entity Id!").clone()));
        }

        let player_entity = commands
            .entity(ent)
            .insert((
                NeedsLoaded,
                player_identifier.sfi,
                Player::new(load_player.name.clone(), load_player.client_id),
            ))
            .remove::<LoadPlayer>()
            .id();

        assign_player_world(&player_worlds, player_entity, &player_identifier.location, &mut commands);
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct KitEntry {
    slot: u32,
    item: String,
    quantity: u16,
}

fn fill_inventory_from_kit(
    kit_name: &str,
    inventory: &mut Inventory,
    items: &Registry<Item>,
    commands: &mut Commands,
    needs_data: &ItemShouldHaveData,
) {
    let Ok(kit) = fs::read_to_string(format!("assets/cosmos/kits/{kit_name}.json")) else {
        error!("Missing kit - {kit_name}");
        return;
    };

    let kit = serde_json::from_str::<Vec<KitEntry>>(&kit).map(Some).unwrap_or_else(|e| {
        error!("{e}");
        None
    });

    let Some(kit) = kit else {
        error!("Invalid kit file - {kit_name}");
        return;
    };

    for entry in kit {
        let Some(item) = items.from_id(&entry.item) else {
            error!("Missing item {} in kit {kit_name}", entry.item);
            continue;
        };

        if entry.slot as usize >= inventory.len() {
            error!("Slot {} in kit {kit_name} out of inventory bounds!", entry.slot);
            continue;
        }

        inventory.insert_item_at(entry.slot as usize, item, entry.quantity, commands, needs_data);
    }
}

fn generate_player_inventory(
    inventory_entity: Entity,
    items: &Registry<Item>,
    commands: &mut Commands,
    has_data: &ItemShouldHaveData,
    creative: bool,
) -> Inventory {
    let mut inventory = Inventory::new("Inventory", 9 * 16, Some(0..9), inventory_entity);

    if creative {
        for item in items.iter().rev().filter(|item| item.unlocalized_name() != "cosmos:air") {
            inventory.insert_item(item, item.max_stack_size(), commands, has_data);
        }
    } else {
        fill_inventory_from_kit("starter", &mut inventory, items, commands, has_data);
    }

    inventory
}

fn create_new_player(
    mut commands: Commands,
    player_worlds: Query<(&Location, &WorldWithin, &RapierContextEntityLink), (With<Player>, Without<Parent>)>,
    items: Res<Registry<Item>>,
    needs_data: Res<ItemShouldHaveData>,
    server_settings: Res<ServerSettings>,
    q_player_needs_loaded: Query<(Entity, &LoadPlayer)>,
    universe_systems: Res<UniverseSystems>,
) {
    for (player_entity, load_player) in q_player_needs_loaded.iter() {
        info!("Creating new player for {}", load_player.name);

        let player = Player::new(load_player.name.clone(), load_player.client_id);

        let (location, rot) = find_new_player_location(&universe_systems);
        let velocity = Velocity::default();
        let inventory = generate_player_inventory(player_entity, &items, &mut commands, &needs_data, server_settings.creative);

        let credits = Credits::new(25_000);

        commands
            .entity(player_entity)
            .insert((
                location,
                velocity,
                player,
                inventory,
                credits,
                Transform::from_rotation(rot),
                PlayerLooking { rotation: Quat::IDENTITY },
            ))
            .remove::<LoadPlayer>();

        assign_player_world(&player_worlds, player_entity, &location, &mut commands);
    }
}

fn finish_loading_player(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut lobby: ResMut<ServerLobby>,
    mut evw_player_join: EventWriter<PlayerConnectedEvent>,
    mut evw_sync_registries: EventWriter<SyncRegistriesEvent>,
    server_settings: Res<ServerSettings>,
    q_player_finished_loading: Query<(Entity, &Player, &Location, &Velocity, Option<&Parent>, Option<&Transform>), Added<Player>>,
    mut nevw_send_chat_msg: NettyEventWriter<ServerSendChatMessageEvent>,
) {
    for (player_entity, load_player, location, velocity, maybe_parent, trans) in q_player_finished_loading.iter() {
        info!("Completing player load for {}", load_player.name());
        let mut ecmds = commands.entity(player_entity);

        ecmds.insert((
            LockedAxes::ROTATION_LOCKED,
            RigidBody::Dynamic,
            Collider::capsule_y(0.65, 0.25),
            Friction {
                coefficient: 0.0,
                combine_rule: CoefficientCombineRule::Min,
            },
            ReadMassProperties::default(),
            LoadingDistance::new(2, 9999),
            ActiveEvents::COLLISION_EVENTS,
            Name::new(format!("Player ({})", load_player.name())),
            Anchor,
            SetPosition::Transform,
        ));
        // If we don't remove this, it won't automatically
        // generate a new one when we save the player next
        // .remove::<SaveFileIdentifier>();

        if server_settings.creative {
            ecmds.insert(Creative);
        }

        lobby.add_player(load_player.id(), player_entity);

        let netty_body = NettyRigidBody::new(
            Some(*velocity),
            trans.map(|x| x.rotation).unwrap_or(Quat::IDENTITY),
            NettyRigidBodyLocation::Absolute(*location),
        );

        info!("Sending player create message for {} @ {}!", load_player.name(), *location);
        let msg = cosmos_encoder::serialize(&ServerReliableMessages::PlayerCreate {
            entity: player_entity,
            parent: maybe_parent.map(|x| x.get()),
            id: load_player.id(),
            name: load_player.name().into(),
            body: netty_body,
            render_distance: None,
        });

        server.send_message(
            load_player.id(),
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::MOTD {
                motd: "Welcome to the server!".into(),
            }),
        );

        server.broadcast_message(NettyChannelServer::Reliable, msg);

        nevw_send_chat_msg.broadcast(ServerSendChatMessageEvent {
            sender: None,
            message: format!("{} joined the game.", load_player.name()),
        });

        evw_player_join.send(PlayerConnectedEvent {
            player_entity,
            client_id: load_player.id(),
        });

        evw_sync_registries.send(SyncRegistriesEvent { player_entity });
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        SAVING_SCHEDULE,
        save_player_link
            .after(SavingSystemSet::CreateEntityIds)
            .before(SavingSystemSet::DoneSaving),
    );
    app.add_systems(
        LOADING_SCHEDULE,
        (
            (load_player, create_new_player)
                .chain()
                .before(LoadingSystemSet::BeginLoading)
                .before(LocationPhysicsSet::DoPhysics)
                .in_set(NetworkingSystemsSet::Between),
            finish_loading_player
                .in_set(NetworkingSystemsSet::SyncComponents)
                .before(ComponentSyncingSet::PreComponentSyncing)
                .after(LoadingSystemSet::DoneLoading),
        )
            .chain(),
    );
}
