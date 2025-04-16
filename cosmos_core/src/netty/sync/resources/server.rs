//! Used for syncing of registries from server -> client

use crate::{
    entities::player::Player,
    netty::{NettyChannelServer, cosmos_encoder, sync::registry::server::SyncRegistriesEvent, system_sets::NetworkingSystemsSet},
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
    prelude::{Deref, IntoSystemSetConfigs, SystemSet, resource_exists_and_changed},
    state::condition::in_state,
};
use bevy_renet::renet::RenetServer;

use super::{ResourceSyncingMessage, SyncableResource};

#[derive(Resource, Deref, Debug, Default)]
/// Keeps track of the number of registries a client must be sent to be considered done loading registries.
struct NumResourcesToSync(u64);

fn sync<T: SyncableResource>(
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<SyncRegistriesEvent>,
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
                data: cosmos_encoder::serialize_uncompressed(resource.as_ref()),
                unlocalized_name: T::unlocalized_name().into(),
            }),
        );
    }
}

fn sync_on_change<T: SyncableResource>(mut server: ResMut<RenetServer>, resource: Res<T>) {
    server.broadcast_message(
        NettyChannelServer::Resource,
        cosmos_encoder::serialize(&ResourceSyncingMessage::Resource {
            data: cosmos_encoder::serialize_uncompressed(resource.as_ref()),
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
    mut ev_reader: EventReader<SyncRegistriesEvent>,
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
                .run_if(in_state(GameState::Playing))
                .chain(),
        );
}

pub(super) fn register(app: &mut App) {
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
