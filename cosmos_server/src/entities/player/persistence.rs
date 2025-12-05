//! Player persistence

use std::fs;

use bevy::prelude::*;
use bevy_rapier3d::prelude::*;
use cosmos_core::{
    chat::ServerSendChatMessageMessage,
    economy::Credits,
    ecs::sets::FixedUpdateSet,
    entities::{
        EntityId,
        health::{Health, MaxHealth},
        player::{Player, creative::Creative},
    },
    inventory::{HeldItemStack, Inventory, itemstack::ItemShouldHaveData},
    item::Item,
    netty::{
        NettyChannelServer, cosmos_encoder,
        netty_rigidbody::{NettyRigidBody, NettyRigidBodyLocation},
        server::ServerLobby,
        server_reliable_messages::ServerReliableMessages,
        sync::{IdentifiableComponent, events::server_event::NettyMessageWriter, registry::server::SyncRegistriesMessage},
    },
    persistence::LoadingDistance,
    physics::location::{Location, LocationPhysicsSet, Sector, SetPosition, systems::Anchor},
    registry::Registry,
};
use renet::{ClientId, RenetServer};
use serde::{Deserialize, Serialize};

use crate::{
    commands::Operators,
    entities::player::spawn_player::find_new_player_location,
    netty::{server_events::PlayerConnectedMessage, sync::flags::SyncReason},
    persistence::{
        SaveFileIdentifier, SerializedData, WorldRoot,
        loading::{LOADING_SCHEDULE, LoadingSystemSet, NeedsLoaded},
        make_persistent::{DefaultPersistentComponent, make_persistent},
        player_loading::RecomputeNeedLoadedChildren,
        saving::{NeedsSaved, SAVING_SCHEDULE, SavingSystemSet, calculate_sfi},
    },
    plugin::server_plugin::ServerType,
    settings::ServerSettings,
    universe::UniverseSystems,
};

use super::{PlayerLooking, spawn_player::CreateNewPlayerMessage};

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

fn generate_player_file_id(player_id: u64) -> String {
    format!("{player_id}.json")
}

const PLAYER_LINK_PATH: &str = "players";

#[derive(Component, Serialize, Deserialize, Debug, Reflect)]
struct PlayerSaveLink {
    id: u64,
}

impl IdentifiableComponent for PlayerSaveLink {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:player_save_link"
    }
}

impl DefaultPersistentComponent for PlayerSaveLink {}

/// Creates a file that points the player's name to their respective data file.
fn save_player_link(
    mut commands: Commands,
    q_parent: Query<&ChildOf>,
    q_entity_id: Query<&EntityId>,
    q_player_link_needs_saved: Query<(Entity, &EntityId, &PlayerSaveLink, &Location), With<NeedsSaved>>,
    q_serialized_data: Query<(&SerializedData, &EntityId, Option<&Location>, Option<&LoadingDistance>)>,
    world_path: Res<WorldRoot>,
) {
    for (entity, e_id, player, loc) in q_player_link_needs_saved.iter() {
        let player_save_path = world_path.path_for(PLAYER_LINK_PATH);
        info!("Saving player {player:?} ({entity:?}) @ {loc}");
        let _ = fs::create_dir_all(&player_save_path);

        let mut parent = q_parent.get(entity).ok();
        while let Some(p) = parent {
            let next = q_parent.get(p.parent()).ok();
            if next.is_some() {
                parent = next;
            } else {
                break;
            }
        }
        if let Some(parent) = parent {
            // We need to load the player save link immediately after this is saved.
            commands.entity(parent.parent()).insert(RecomputeNeedLoadedChildren);
        }

        let sfi = calculate_sfi(entity, &q_parent, &q_entity_id, &q_serialized_data).expect("Missing save file identifier for player!");

        let player_identifier = PlayerIdentifier {
            sector: loc.sector(),
            entity_id: *e_id,
            sfi,
            location: *loc,
        };

        let json_data = serde_json::to_string(&player_identifier).expect("Failed to create json");

        let player_file_name = generate_player_file_id(player.id);
        fs::write(format!("{player_save_path}/{player_file_name}"), json_data).expect("Failed to save player!!!");
    }
}

fn load_player(
    mut commands: Commands,
    q_player_needs_loaded: Query<(Entity, &LoadPlayer)>,
    q_entity_ids: Query<&EntityId>,
    q_player_save_links: Query<(Entity, &PlayerSaveLink), Without<Player>>,
    world_root: Res<WorldRoot>,
) {
    for (ent, load_player) in q_player_needs_loaded.iter() {
        if let Some((already_loaded_player_link, _)) = q_player_save_links.iter().find(|(_, link)| link.id == load_player.client_id) {
            info!(
                "Player entity already exists in game for {} - adding Player component.",
                load_player.name
            );

            commands
                .entity(already_loaded_player_link)
                .insert(Player::new(load_player.name.clone(), load_player.client_id));

            // We don't need this anymore, since this player already has their link loaded.
            commands.entity(ent).despawn();

            continue;
        }

        let player_file_name = generate_player_file_id(load_player.client_id);

        info!("Attempting to load player {}", load_player.name);
        let Ok(data) = fs::read(world_root.path_for(format!("{PLAYER_LINK_PATH}/{player_file_name}").as_str())) else {
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
            let entity_id = *sfi.entity_id().expect("Missing Entity Id!");
            // Don't load already existing entities
            if q_entity_ids.iter().any(|x| x == &entity_id) {
                continue;
            }

            info!("Loading player parent ({entity_id}) ({sfi:?})");
            let mut ecmds = commands.spawn((NeedsLoaded, sfi.clone(), entity_id));
            if cur_sfi.get_parent().is_none() {
                ecmds.insert(RecomputeNeedLoadedChildren);
            }
        }

        let entity_id = *player_identifier.sfi.entity_id().expect("Missing player entity id ;(");

        let mut player_entity = commands.entity(ent);

        player_entity
            .insert((
                NeedsLoaded,
                player_identifier.sfi,
                Player::new(load_player.name.clone(), load_player.client_id),
                entity_id,
            ))
            .remove::<LoadPlayer>();
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
) -> Inventory {
    let n_slots = 9 * 7;

    let mut inventory = Inventory::new("Inventory", n_slots, Some(0..9), inventory_entity);

    fill_inventory_from_kit("starter", &mut inventory, items, commands, has_data);

    inventory
}

fn create_new_player(
    mut commands: Commands,
    items: Res<Registry<Item>>,
    needs_data: Res<ItemShouldHaveData>,
    q_player_needs_loaded: Query<(Entity, &LoadPlayer)>,
    universe_systems: Res<UniverseSystems>,
    mut evw_create_new_player: MessageWriter<CreateNewPlayerMessage>,
) {
    for (player_entity, load_player) in q_player_needs_loaded.iter() {
        let Some((location, rot)) = find_new_player_location(&universe_systems) else {
            info!("Universe not generated yet - will delay spawning player {}", load_player.name);
            continue;
        };

        info!("Creating new player for {}", load_player.name);

        let player = Player::new(load_player.name.clone(), load_player.client_id);

        let velocity = Velocity::default();
        let inventory = generate_player_inventory(player_entity, &items, &mut commands, &needs_data);

        let credits = Credits::new(5_000);

        let starting_health = MaxHealth::new(20);

        evw_create_new_player.write(CreateNewPlayerMessage::new(player_entity));

        commands
            .entity(player_entity)
            .insert((
                location,
                velocity,
                player,
                inventory,
                credits,
                starting_health,
                Health::from(starting_health),
                Transform::from_rotation(rot),
                PlayerLooking { rotation: Quat::IDENTITY },
            ))
            .with_children(|p| {
                let mut ecmds = p.spawn((Name::new("Held Item Inventory"), HeldItemStack, SyncReason::Data));
                let inventory = Inventory::new("Inventory", 1, None, ecmds.id());
                ecmds.insert(inventory);
            })
            .remove::<LoadPlayer>();
    }
}

fn finish_loading_player(
    mut commands: Commands,
    mut server: ResMut<RenetServer>,
    mut lobby: ResMut<ServerLobby>,
    mut evw_player_join: MessageWriter<PlayerConnectedMessage>,
    mut evw_sync_registries: MessageWriter<SyncRegistriesMessage>,
    server_settings: Res<ServerSettings>,
    q_player_finished_loading: Query<(Entity, &Player, &Location, &Velocity, Option<&ChildOf>, Option<&Transform>), Added<Player>>,
    mut nevw_send_chat_msg: NettyMessageWriter<ServerSendChatMessageMessage>,
    q_held_item: Query<&Inventory, With<HeldItemStack>>,
    q_children: Query<&Children>,
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

        if HeldItemStack::get_held_is_inventory(ecmds.id(), &q_children, &q_held_item).is_none() {
            error!("Missing held item inventory - inserting new one!");
            ecmds.with_children(|p| {
                let mut ecmds = p.spawn((Name::new("Held Item Inventory"), HeldItemStack, SyncReason::Data));
                let inventory = Inventory::new("Inventory", 1, None, ecmds.id());
                ecmds.insert(inventory);
            });
        }
        // If we don't remove this, it won't automatically
        // generate a new one when we save the player next
        // .remove::<SaveFileIdentifier>();

        if server_settings.creative {
            ecmds.insert(Creative);
        }

        lobby.add_player(load_player.client_id(), player_entity);

        let netty_body = NettyRigidBody::new(
            Some(*velocity),
            trans.map(|x| x.rotation).unwrap_or(Quat::IDENTITY),
            NettyRigidBodyLocation::Absolute(*location),
        );

        info!("Sending player create message for {} @ {}!", load_player.name(), *location);
        let msg = cosmos_encoder::serialize(&ServerReliableMessages::PlayerCreate {
            entity: player_entity,
            parent: maybe_parent.map(|x| x.parent()),
            id: load_player.client_id(),
            name: load_player.name().into(),
            body: netty_body,
            render_distance: None,
        });

        server.send_message(
            load_player.client_id(),
            NettyChannelServer::Reliable,
            cosmos_encoder::serialize(&ServerReliableMessages::MOTD {
                motd: "Welcome to the server!".into(),
            }),
        );

        server.broadcast_message(NettyChannelServer::Reliable, msg);

        nevw_send_chat_msg.broadcast(ServerSendChatMessageMessage {
            sender: None,
            message: format!("{} joined the game.", load_player.name()),
        });

        evw_player_join.write(PlayerConnectedMessage {
            player_entity,
            client_id: load_player.client_id(),
        });

        evw_sync_registries.write(SyncRegistriesMessage { player_entity });
    }
}

fn add_player_save_link(mut commands: Commands, q_player_needs_save_link: Query<(Entity, &Player), Without<PlayerSaveLink>>) {
    for (e, player) in q_player_needs_save_link.iter() {
        commands.entity(e).insert(PlayerSaveLink { id: player.client_id() });
    }
}

fn name_player_save_links(mut commands: Commands, q_player_save_links: Query<(Entity, &PlayerSaveLink), Without<Player>>) {
    for (e, link) in q_player_save_links.iter() {
        commands.entity(e).insert(Name::new(format!("Player Save Link ({})", link.id)));
    }
}

pub(super) fn register(app: &mut App) {
    make_persistent::<PlayerSaveLink>(app);
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
                .in_set(FixedUpdateSet::Main),
            (finish_loading_player, add_player_save_link, name_player_save_links)
                .chain()
                .in_set(FixedUpdateSet::Main)
                .after(LoadingSystemSet::DoneLoading),
        )
            .chain(),
    )
    .register_type::<PlayerSaveLink>();
}
