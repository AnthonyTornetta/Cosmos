//! Handles client-side registry syncing logic

use bevy::{
    app::{App, Update},
    ecs::{
        event::{Event, EventReader, EventWriter},
        schedule::{common_conditions::resource_exists, IntoSystemConfigs},
        system::{Res, ResMut, Resource},
    },
    log::{error, info},
    prelude::{IntoSystemSetConfigs, SystemSet},
    reflect::erased_serde::Serialize,
    state::{condition::in_state, state::NextState},
};
use bevy_renet2::renet2::RenetClient;
use cosmos_core::{
    netty::{cosmos_encoder, server_registry::RegistrySyncing, system_sets::NetworkingSystemsSet, NettyChannelServer},
    registry::{identifiable::Identifiable, Registry},
};
use serde::de::DeserializeOwned;

use crate::{ecs::add_multi_statebound_resource, state::game_state::GameState};

#[derive(Event)]
struct ReceivedRegistryEvent {
    serialized_data: Vec<u8>,
    registry_name: String,
}

#[derive(Debug, Default, Resource)]
struct RegistriesLeftToSync(Option<i64>);

fn sync<T: Identifiable + Serialize + DeserializeOwned + std::fmt::Debug>(
    mut registry: ResMut<Registry<T>>,
    mut ev_reader: EventReader<ReceivedRegistryEvent>,
    mut left_to_sync: ResMut<RegistriesLeftToSync>,
) {
    for ev in ev_reader.read() {
        if ev.registry_name != registry.name() {
            continue;
        }

        let new_amt = left_to_sync.0.unwrap_or(0) - 1;

        left_to_sync.0 = Some(new_amt);

        info!("Got registry from server: {}! Need {} more.", ev.registry_name, new_amt);

        let Ok(new_registry) = cosmos_encoder::deserialize::<Registry<T>>(&ev.serialized_data) else {
            error!("Got bad registry data from server - {}!", ev.registry_name);
            continue;
        };

        *registry = new_registry;
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum LoadingRegistriesSet {
    LoadRegistriesFromServer,
}

/// Call this function on the client-side to signal that this registry should be synced with the server
pub fn sync_registry<T: Identifiable + Serialize + DeserializeOwned + std::fmt::Debug>(app: &mut App) {
    app.add_systems(
        Update,
        sync::<T>
            .before(transition_state)
            .in_set(LoadingRegistriesSet::LoadRegistriesFromServer)
            .ambiguous_with(LoadingRegistriesSet::LoadRegistriesFromServer)
            .run_if(in_state(GameState::LoadingData)),
    );
}

fn registry_listen_netty(
    mut client: ResMut<RenetClient>,
    mut ev_writer: EventWriter<ReceivedRegistryEvent>,
    mut registry_count: ResMut<RegistriesLeftToSync>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Registry) {
        let msg: RegistrySyncing = cosmos_encoder::deserialize(&message).expect("Unable to parse registry sync from server");

        match msg {
            RegistrySyncing::RegistryCount(count) => {
                info!("Need to load {count} registries from server.");
                registry_count.0 = Some(count as i64 + registry_count.0.unwrap_or(0));
            }
            RegistrySyncing::Registry { serialized, registry_name } => {
                ev_writer.send(ReceivedRegistryEvent {
                    serialized_data: serialized,
                    registry_name,
                });
            }
        }
    }
}

fn transition_state(mut state_changer: ResMut<NextState<GameState>>, loading_registries: Res<RegistriesLeftToSync>) {
    if loading_registries.0.is_some_and(|x| x == 0) {
        info!("Got all registries from server - loading world!");
        state_changer.set(GameState::LoadingWorld);
    }
}

pub(super) fn register(app: &mut App) {
    app.configure_sets(
        Update,
        LoadingRegistriesSet::LoadRegistriesFromServer
            .run_if(in_state(GameState::LoadingData))
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );

    app.add_systems(
        Update,
        (
            registry_listen_netty.before(LoadingRegistriesSet::LoadRegistriesFromServer),
            transition_state,
        )
            .run_if(resource_exists::<RegistriesLeftToSync>)
            .chain()
            .run_if(in_state(GameState::LoadingData)),
    )
    .add_event::<ReceivedRegistryEvent>();

    add_multi_statebound_resource::<RegistriesLeftToSync>(app, GameState::Connecting, GameState::LoadingData);
}
