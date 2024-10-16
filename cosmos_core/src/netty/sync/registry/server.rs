//! Used for syncing of registries from server -> client

use crate::{
    entities::player::Player,
    netty::{cosmos_encoder, server_registry::RegistrySyncing, system_sets::NetworkingSystemsSet, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
};
use bevy::{
    app::{App, Startup, Update},
    ecs::{
        event::EventReader,
        schedule::IntoSystemConfigs,
        system::{Query, Res, ResMut, Resource},
    },
    log::{info, warn},
    prelude::{Deref, Entity, Event, IntoSystemSetConfigs, States, SystemSet},
    state::condition::in_state,
};
use bevy_renet2::renet2::RenetServer;
use serde::{Deserialize, Serialize};

#[derive(Resource, Deref, Debug, Default)]
/// Keeps track of the number of registries a client must be sent to be considered done loading registries.
struct NumRegistriesToSync(u64);

#[derive(Event)]
/// This event signifies that this player needs to have their registries mapped to the server's
/// registries. This should be sent whenever the player initially joins.
pub struct SyncRegistriesEvent {
    /// The player's entity
    pub player_entity: Entity,
}

fn sync<'a, T: Identifiable + Serialize + Deserialize<'a>>(
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<SyncRegistriesEvent>,
    registry: Res<Registry<T>>,
) {
    for ev in ev_reader.read() {
        let Ok(player) = q_player.get(ev.player_entity) else {
            warn!("Missing player entity from player join event!");
            continue;
        };

        server.send_message(
            player.id(),
            NettyChannelServer::Registry,
            cosmos_encoder::serialize(&RegistrySyncing::Registry {
                serialized: cosmos_encoder::serialize(registry.as_ref()),
                registry_name: registry.name().into(),
            }),
        );
    }
}

fn incr_registries_to_sync(mut n_registries: ResMut<NumRegistriesToSync>) {
    n_registries.0 += 1;
}

fn send_number_of_registries(
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<SyncRegistriesEvent>,
    n_registries: Res<NumRegistriesToSync>,
) {
    for ev in ev_reader.read() {
        let Ok(player) = q_player.get(ev.player_entity) else {
            warn!("Missing player entity from player join event!");
            continue;
        };

        info!("Sending {n_registries:?}");

        server.send_message(
            player.id(),
            NettyChannelServer::Registry,
            cosmos_encoder::serialize(&RegistrySyncing::RegistryCount(n_registries.0)),
        );
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum IncrementSet {
    Increment,
}

/// Call this function on the server-side to signal that this registry should be synced with the client
pub(super) fn sync_registry<'a, T: Identifiable + Serialize + Deserialize<'a>>(app: &mut App) {
    app.add_systems(Startup, incr_registries_to_sync.in_set(IncrementSet::Increment))
        .add_systems(Update, sync::<T>.after(send_number_of_registries));
}

#[allow(unused)] // LSP assumes this function is never used, even though it's just feature flagged
pub(super) fn register<T: States>(app: &mut App, playing_state: T) {
    app.add_event::<SyncRegistriesEvent>();
    app.configure_sets(Startup, IncrementSet::Increment.ambiguous_with(IncrementSet::Increment));

    app.add_systems(
        Update,
        send_number_of_registries
            .run_if(in_state(playing_state))
            .after(NetworkingSystemsSet::ProcessReceivedMessages),
    )
    .init_resource::<NumRegistriesToSync>();
}
