use bevy::{prelude::*, utils::HashSet};
use cosmos_core::{
    entities::player::Player,
    netty::{
        NoSendEntity,
        sync::{server_entity_syncing::RequestedEntityEvent, server_syncing::should_be_sent_to},
    },
    persistence::LoadingDistance,
    physics::location::Location,
};
use renet::ClientId;

#[derive(Component, Debug, Reflect, Clone, Default)]
pub enum SyncReason {
    /// This will only be synced if this entity has a parent that should be synced
    ///
    /// This should be used when this entity is data that describes its parent
    Data,
    /// This will only be synced if the location and the player are within a specific distance of
    /// each other
    #[default]
    Location,
}

#[derive(Component, Debug, Reflect, Clone, Default)]
/// Contains the list of entities this component should be synced to
pub struct SyncTo(HashSet<ClientId>);

enum MegaBool {
    True,
    False,
    MegaFalse,
}

fn should_sync(
    this_ent: Entity,
    q_sync_to: &Query<
        (
            Entity,
            Option<&SyncReason>,
            Option<&Location>,
            Option<&LoadingDistance>,
            Option<&Parent>,
        ),
        (Without<NoSendEntity>, With<SyncTo>),
    >,
    player_loc: &Location,
) -> MegaBool {
    let Ok((_, sync_reason, location, loading_distance, parent)) = q_sync_to.get(this_ent) else {
        return MegaBool::MegaFalse;
    };

    let sync_reason = sync_reason.cloned().unwrap_or_default();

    match sync_reason {
        SyncReason::Data => {
            let Some(parent) = parent else {
                return MegaBool::MegaFalse;
            };

            should_sync(parent.get(), q_sync_to, player_loc)
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
        ),
        (Without<NoSendEntity>, With<SyncTo>),
    >,
    mut q_mut_sync_to: Query<&mut SyncTo>,
    q_players: Query<(&Player, &Location)>,
) {
    for (ent, sync_reason, this_loc, loading_distance, parent) in q_sync_to.iter() {
        let sync_reason = sync_reason.cloned().unwrap_or_default();

        let mut to_send_to = vec![];

        for (player, player_loc) in q_players.iter() {
            let should_sync = match sync_reason {
                SyncReason::Data => {
                    let Some(parent) = parent else {
                        break;
                    };

                    should_sync(parent.get(), &q_sync_to, player_loc)
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

            to_send_to.push(id);
        }

        let mut sync_to = q_mut_sync_to.get_mut(ent).expect("Invalid state");

        sync_to.0 = to_send_to;
        sync_to.0.sort();
    }
}

#[derive(Debug, Component, Reflect)]
struct PreviousSyncTo(SyncTo);

fn send_to_sync_tos(
    mut evr_request_entity: EventWriter<RequestedEntityEvent>,
    mut q_sync_to: Query<(Entity, &SyncTo, &mut PreviousSyncTo)>,
) {
    for (ent, sync_to, mut prev) in q_sync_to.iter_mut() {
        let mut not_found = vec![];

        // let mut prev_i = 0;
        // for &item in sync_to.0.iter() {
        //     while prev_i < prev.0.0.len() {
        //         if item < prev.0.0[prev_i] {
        //             not_found.push(item);
        //         } else {
        //             prev_i += 1;
        //         }
        //     }
        //
        //     if prev_i >= prev.0.0.len() {
        //         not_found.push(item);
        //         continue;
        //     }
        // }
        //
        // prev.0 = sync_to.clone();
    }
}

fn on_add_sync_data(mut commands: Commands, q_added_sync_data: Query<Entity, (Without<SyncTo>, With<Location>, Without<NoSendEntity>)>) {
    for ent in q_added_sync_data.iter() {
        commands.entity(ent).insert(SyncTo::default());
    }
}

pub(super) fn register(app: &mut App) {}
