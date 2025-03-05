//! Used for syncing of registries from server -> client

use crate::{
    entities::player::Player,
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelServer},
    state::GameState,
};
use bevy::{
    app::{App, Startup, Update},
    ecs::{
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Query, Res, ResMut, Resource},
    },
    log::{info, warn},
    prelude::{resource_exists_and_changed, Deref, Entity, Event, IntoSystemSetConfigs, SystemSet},
    state::condition::in_state,
};
use bevy_renet2::renet2::RenetServer;

use super::{ResourceSyncingMessage, SyncableResource};

#[derive(Resource, Deref, Debug, Default)]
/// Keeps track of the number of registries a client must be sent to be considered done loading registries.
struct NumResourcesToSync(u64);

#[derive(Event)]
/// This event signifies that this player needs to have their registries mapped to the server's
/// registries. This should be sent whenever the player initially joins.
pub struct SyncResourceEvent {
    /// The player's entity
    pub player_entity: Entity,
}

fn sync<T: SyncableResource>(
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<SyncResourceEvent>,
    resource: Res<T>,
) {
    for ev in ev_reader.read() {
        let Ok(player) = q_player.get(ev.player_entity) else {
            warn!("Missing player entity from player join event!");
            continue;
        };

        server.send_message(
            player.client_id(),
            NettyChannelServer::Resource,
            cosmos_encoder::serialize(&ResourceSyncingMessage::Resource {
                data: bincode::serialize(resource.as_ref()).expect("Failed to serialize :("),
                unlocalized_name: T::unlocalized_name().into(),
            }),
        );
    }
}

fn sync_on_change<T: SyncableResource>(mut server: ResMut<RenetServer>, resource: Res<T>) {
    server.broadcast_message(
        NettyChannelServer::Resource,
        cosmos_encoder::serialize(&ResourceSyncingMessage::Resource {
            data: bincode::serialize(resource.as_ref()).expect("Failed to serialize :("),
            unlocalized_name: T::unlocalized_name().into(),
        }),
    );
}

fn incr_resources_to_sync(mut n_resources: ResMut<NumResourcesToSync>) {
    n_resources.0 += 1;
}

fn send_number_of_resources(
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<SyncResourceEvent>,
    n_resources: Res<NumResourcesToSync>,
) {
    for ev in ev_reader.read() {
        let Ok(player) = q_player.get(ev.player_entity) else {
            warn!("Missing player entity from player join event!");
            continue;
        };

        info!("Sending {n_resources:?} resources.");

        server.send_message(
            player.client_id(),
            NettyChannelServer::Resource,
            cosmos_encoder::serialize(&ResourceSyncingMessage::ResourceCount(n_resources.0)),
        );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum IncrementResourcesSet {
    Increment,
}

/// Call this function on the server-side to signal that this resources should be synced with the client
pub(super) fn sync_resource<T: SyncableResource>(app: &mut App) {
    app.add_systems(Startup, incr_resources_to_sync.in_set(IncrementResourcesSet::Increment))
        .add_systems(
            Update,
            (sync::<T>, sync_on_change::<T>.run_if(resource_exists_and_changed::<T>))
                .after(send_number_of_resources)
                .chain(),
        );
}

pub(super) fn register(app: &mut App) {
    app.add_event::<SyncResourceEvent>();
    app.configure_sets(
        Startup,
        IncrementResourcesSet::Increment.ambiguous_with(IncrementResourcesSet::Increment),
    );

    app.add_systems(
        Update,
        send_number_of_resources
            .run_if(in_state(GameState::Playing))
            .after(NetworkingSystemsSet::ProcessReceivedMessages),
    )
    .init_resource::<NumResourcesToSync>();
}
