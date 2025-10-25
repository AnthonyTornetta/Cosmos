//! The flags an entity can have that dictate its syncing.
//!
//! Notably: [`SyncTo`] and [`SyncReason`]

use bevy::{platform::collections::HashSet, prelude::*};
use cosmos_core::{
    block::data::BlockData,
    ecs::sets::FixedUpdateSet,
    entities::player::Player,
    inventory::itemstack::ItemStackData,
    netty::{
        NettyChannelServer, NoSendEntity, cosmos_encoder,
        server_reliable_messages::ServerReliableMessages,
        sync::{
            ComponentEntityIdentifier,
            server_entity_syncing::RequestedEntityEvent,
            server_syncing::{ReadyForSyncing, SyncTo},
        },
        system_sets::NetworkingSystemsSet,
    },
    persistence::LoadingDistance,
    physics::location::Location,
    prelude::{Structure, StructureSystem},
};
use renet::RenetServer;

use crate::persistence::loading::{NeedsBlueprintLoaded, NeedsLoaded};

#[derive(Component, Debug, Reflect, Clone, Default)]
/// The reasons this entity will be synced.
///
/// If an entity has a [`Location`] component, this will be assumed to be [`SyncReason::Location`],
/// even if this component is not on this entity. Otherwise, this component can be manually added
/// to an entity to have it be synced for other reasons.
pub enum SyncReason {
    /// This will only be synced if this entity has a parent that should be synced
    ///
    /// This should be used when this entity is data that describes its parent
    Data,
    /// This should follow the default block data syncing rules
    ///
    /// This WILL be used as the default for entities with the [`BlockData`] component.
    ///
    /// 1. The player is within 1 sector
    /// 2. The structure is being synced w/ the player
    BlockData,
    /// This will only be synced if the location and the player are within a specific distance of
    /// each other
    #[default]
    Location,
}

fn add_structure_systems_sync_flag(
    q_structure_systems: Query<Entity, (With<StructureSystem>, Without<SyncReason>)>,
    mut commands: Commands,
) {
    for ent in q_structure_systems.iter() {
        commands.entity(ent).insert(SyncReason::Data);
    }
}

fn add_item_data_sync_flag(q_item_data: Query<Entity, (With<ItemStackData>, Without<SyncReason>)>, mut commands: Commands) {
    for ent in q_item_data.iter() {
        commands.entity(ent).insert(SyncReason::Data);
    }
}

/// MegaFalse represents this failing for all players, not just one.
enum MegaBool {
    True,
    False,
    MegaFalse,
}

fn should_sync(
    this_ent: Entity,
    q_parent: &Query<&ChildOf>,
    q_sync_to: &Query<
        (
            Entity,
            Option<&SyncReason>,
            Option<&Location>,
            Option<&LoadingDistance>,
            Option<&ChildOf>,
            Option<&Structure>,
            Has<BlockData>,
        ),
        (
            Without<NoSendEntity>,
            With<SyncTo>,
            Without<NeedsBlueprintLoaded>,
            Without<NeedsLoaded>,
        ),
    >,
    player_loc: &Location,
) -> MegaBool {
    let Ok((_, sync_reason, location, loading_distance, parent, structure, block_data)) = q_sync_to.get(this_ent) else {
        return MegaBool::MegaFalse;
    };

    // TODO: This structure-specific check should be moved in the future, and an `unloaded` component
    // should be created.
    if structure
        .map(|s| match s {
            Structure::Full(f) => !f.is_loaded(),
            Structure::Dynamic(_) => false,
        })
        .unwrap_or(false)
    {
        info!("Rejected because of structure unloaded!");
        return MegaBool::MegaFalse;
    }

    let sync_reason = sync_reason
        .cloned()
        .unwrap_or(if block_data { SyncReason::BlockData } else { Default::default() });

    match sync_reason {
        SyncReason::Data => {
            let Some(parent) = parent else {
                return MegaBool::MegaFalse;
            };

            should_sync(parent.parent(), q_parent, q_sync_to, player_loc)
        }
        SyncReason::BlockData => {
            let Some(parent) = parent else {
                return MegaBool::MegaFalse;
            };

            let Ok(parent) = q_parent.get(parent.parent()) else {
                return MegaBool::MegaFalse;
            };

            let Ok(Some(location)) = q_sync_to.get(parent.parent()).map(|(_, _, location, _, _, _, _)| location) else {
                return MegaBool::MegaFalse;
            };

            match should_sync(parent.parent(), q_parent, q_sync_to, player_loc) {
                MegaBool::MegaFalse => MegaBool::MegaFalse,
                MegaBool::False => MegaBool::False,
                MegaBool::True => {
                    if LoadingDistance::new(1, 1).should_load(player_loc, location) {
                        MegaBool::True
                    } else {
                        MegaBool::False
                    }
                }
            }
        }
        SyncReason::Location => {
            let (Some(location), Some(loading_distance)) = (location, loading_distance) else {
                return MegaBool::MegaFalse;
            };

            if loading_distance.should_load(player_loc, location) {
                MegaBool::True
            } else {
                MegaBool::False
            }
        }
    }
}

fn update_sync_players(
    q_sync_to: Query<
        (
            Entity,
            Option<&SyncReason>,
            Option<&Location>,
            Option<&LoadingDistance>,
            Option<&ChildOf>,
            Option<&Structure>,
            Has<BlockData>,
        ),
        (
            Without<NoSendEntity>,
            With<SyncTo>,
            Without<NeedsBlueprintLoaded>,
            Without<NeedsLoaded>,
        ),
    >,
    q_parent: Query<&ChildOf>,
    mut q_mut_sync_to: Query<&mut SyncTo>,
    q_players: Query<(&Player, &Location), With<ReadyForSyncing>>,
) {
    for (ent, sync_reason, this_loc, loading_distance, parent, _, block_data) in q_sync_to.iter() {
        let sync_reason = sync_reason
            .cloned()
            .unwrap_or(if block_data { SyncReason::BlockData } else { Default::default() });

        let mut to_send_to = HashSet::default();

        for (player, player_loc) in q_players.iter() {
            let should_sync = match sync_reason {
                SyncReason::Data => {
                    let Some(parent) = parent else {
                        break;
                    };

                    should_sync(parent.parent(), &q_parent, &q_sync_to, player_loc)
                }
                SyncReason::BlockData => {
                    let Some(parent) = parent else {
                        break;
                    };

                    let Ok(parent) = q_parent.get(parent.parent()) else {
                        break;
                    };

                    let Ok(Some(location)) = q_sync_to.get(parent.parent()).map(|(_, _, location, _, _, _, _)| location) else {
                        break;
                    };

                    match should_sync(parent.parent(), &q_parent, &q_sync_to, player_loc) {
                        MegaBool::MegaFalse => break,
                        MegaBool::False => MegaBool::False,
                        MegaBool::True => {
                            if LoadingDistance::new(1, 1).should_load(player_loc, location) {
                                MegaBool::True
                            } else {
                                MegaBool::False
                            }
                        }
                    }
                }
                SyncReason::Location => {
                    let (Some(location), Some(loading_distance)) = (this_loc, loading_distance) else {
                        break;
                    };

                    if loading_distance.should_load(player_loc, location) {
                        MegaBool::True
                    } else {
                        MegaBool::False
                    }
                }
            };

            let id = match should_sync {
                MegaBool::MegaFalse => break,
                MegaBool::False => continue,
                MegaBool::True => player.client_id(),
            };

            to_send_to.insert(id);
        }

        let mut sync_to = q_mut_sync_to.get_mut(ent).expect("Invalid state");

        *sync_to = SyncTo::new(to_send_to);
    }
}

#[derive(Debug, Component, Reflect, Default)]
struct PreviousSyncTo(SyncTo);

fn generate_request_entity_events_for_new_sync_tos(
    mut evr_request_entity: EventWriter<RequestedEntityEvent>,
    mut q_sync_to: Query<(Entity, &SyncTo, &mut PreviousSyncTo)>,
    mut server: ResMut<RenetServer>,
) {
    for (ent, sync_to, mut prev) in q_sync_to.iter_mut() {
        let mut not_found = vec![];
        let mut no_longer_synced_to = vec![];

        for &id in sync_to.iter() {
            if !prev.0.should_sync_to(id) {
                not_found.push(id);
            }
        }

        for &id in prev.0.iter() {
            if !sync_to.should_sync_to(id) {
                no_longer_synced_to.push(id);
            }
        }

        if not_found.is_empty() && no_longer_synced_to.is_empty() {
            continue;
        }

        prev.0 = sync_to.clone();

        for id in not_found {
            evr_request_entity.write(RequestedEntityEvent {
                entity: ent,
                client_id: id,
            });
        }

        for id in no_longer_synced_to {
            server.send_message(
                id,
                NettyChannelServer::Reliable,
                cosmos_encoder::serialize(&ServerReliableMessages::EntityDespawn {
                    // We only care about normal entities for this - block data and stuff doesn't
                    // really matter
                    entity: ComponentEntityIdentifier::Entity(ent),
                }),
            );
        }
    }
}

fn on_needs_sync_data(
    mut commands: Commands,
    q_added_sync_data: Query<
        Entity,
        (
            Without<SyncTo>,
            Or<(With<Location>, With<BlockData>, With<SyncReason>)>,
            Without<NoSendEntity>,
        ),
    >,
) {
    for ent in q_added_sync_data.iter() {
        commands.entity(ent).insert((SyncTo::default(), PreviousSyncTo::default()));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        FixedUpdate,
        (
            add_item_data_sync_flag,
            add_structure_systems_sync_flag,
            on_needs_sync_data,
            update_sync_players,
            generate_request_entity_events_for_new_sync_tos,
        )
            .chain()
            .in_set(FixedUpdateSet::NettySend)
            .before(NetworkingSystemsSet::SyncComponents),
    )
    .register_type::<SyncTo>()
    .register_type::<SyncReason>();
}
