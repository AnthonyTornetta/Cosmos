//! The flags an entity can have that dictate its syncing.
//!
//! Notably: [`SyncTo`] and [`SyncReason`]

use bevy::{prelude::*, utils::HashSet};
use cosmos_core::{
    block::data::BlockData,
    entities::player::Player,
    netty::{
        NoSendEntity,
        sync::{server_entity_syncing::RequestedEntityEvent, server_syncing::ReadyForSyncing},
        system_sets::NetworkingSystemsSet,
    },
    persistence::LoadingDistance,
    physics::location::Location,
    prelude::Structure,
};
use renet::ClientId;

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

#[derive(Component, Debug, Reflect, Clone, Default)]
/// Contains the list of entities this component should be synced to
pub struct SyncTo(HashSet<ClientId>);

impl SyncTo {
    /// Returns if this should be synced to this client id.
    pub fn should_sync_to(&self, client_id: ClientId) -> bool {
        self.0.contains(&client_id)
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
    q_parent: &Query<&Parent>,
    q_sync_to: &Query<
        (
            Entity,
            Option<&SyncReason>,
            Option<&Location>,
            Option<&LoadingDistance>,
            Option<&Parent>,
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

            should_sync(parent.get(), q_parent, q_sync_to, player_loc)
        }
        SyncReason::BlockData => {
            let Some(parent) = parent else {
                return MegaBool::MegaFalse;
            };

            let Ok(parent) = q_parent.get(parent.get()) else {
                return MegaBool::MegaFalse;
            };

            let Ok(Some(location)) = q_sync_to.get(parent.get()).map(|(_, _, location, _, _, _, _)| location) else {
                return MegaBool::MegaFalse;
            };

            match should_sync(parent.get(), q_parent, q_sync_to, player_loc) {
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
            Option<&Parent>,
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
    q_parent: Query<&Parent>,
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

                    should_sync(parent.get(), &q_parent, &q_sync_to, player_loc)
                }
                SyncReason::BlockData => {
                    let Some(parent) = parent else {
                        break;
                    };

                    let Ok(parent) = q_parent.get(parent.get()) else {
                        break;
                    };

                    let Ok(Some(location)) = q_sync_to.get(parent.get()).map(|(_, _, location, _, _, _, _)| location) else {
                        break;
                    };

                    match should_sync(parent.get(), &q_parent, &q_sync_to, player_loc) {
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

        sync_to.0 = to_send_to;
    }
}

#[derive(Debug, Component, Reflect, Default)]
struct PreviousSyncTo(SyncTo);

fn generate_request_entity_events_for_new_sync_tos(
    mut evr_request_entity: EventWriter<RequestedEntityEvent>,
    mut q_sync_to: Query<(Entity, &SyncTo, &mut PreviousSyncTo)>,
    mut commands: Commands,
) {
    for (ent, sync_to, mut prev) in q_sync_to.iter_mut() {
        let mut not_found = vec![];

        for id in sync_to.0.iter() {
            if !prev.0.0.contains(id) {
                not_found.push(*id);
            }
        }

        if not_found.is_empty() {
            continue;
        }

        prev.0 = sync_to.clone();

        for id in not_found {
            info!("Send {ent:?} to player id {id}");
            commands.entity(ent).log_components();
            evr_request_entity.send(RequestedEntityEvent {
                entity: ent,
                client_id: id,
            });
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
        Update,
        (
            on_needs_sync_data,
            update_sync_players,
            generate_request_entity_events_for_new_sync_tos,
        )
            .chain()
            .after(NetworkingSystemsSet::Between)
            .before(NetworkingSystemsSet::SyncComponents),
    )
    .register_type::<SyncTo>()
    .register_type::<SyncReason>();
}
