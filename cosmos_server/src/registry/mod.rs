//! Used for syncing of registries from server -> client

use bevy::{
    app::{App, Startup, Update},
    ecs::{
        event::EventReader,
        schedule::{common_conditions::in_state, IntoSystemConfigs},
        system::{Query, Res, ResMut, Resource},
    },
    log::{info, warn},
    prelude::Deref,
};
use bevy_renet2::renet2::RenetServer;
use cosmos_core::{
    entities::player::Player,
    netty::{cosmos_encoder, server_registry::RegistrySyncing, system_sets::NetworkingSystemsSet, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
};
use serde::{Deserialize, Serialize};

use crate::{events::netty::netty_events::PlayerConnectedEvent, state::GameState};

#[derive(Resource, Deref, Debug, Default)]
/// Keeps track of the number of registries a client must be sent to be considered done loading registries.
struct NumRegistriesToSync(u64);

fn sync<'a, T: Identifiable + Serialize + Deserialize<'a>>(
    q_player: Query<&Player>,
    mut server: ResMut<RenetServer>,
    mut ev_reader: EventReader<PlayerConnectedEvent>,
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
    mut ev_reader: EventReader<PlayerConnectedEvent>,
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

/// Call this function on the server-side to signal that this registry should be synced with the client
pub fn sync_registry<'a, T: Identifiable + Serialize + Deserialize<'a>>(app: &mut App) {
    app.add_systems(Startup, incr_registries_to_sync).add_systems(
        Update,
        sync::<T>.run_if(in_state(GameState::Playing)).after(send_number_of_registries),
    );
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        send_number_of_registries
            .run_if(in_state(GameState::Playing))
            .after(NetworkingSystemsSet::ProcessReceivedMessages),
    )
    .init_resource::<NumRegistriesToSync>();
}
