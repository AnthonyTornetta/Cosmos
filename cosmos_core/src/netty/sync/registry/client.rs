//! Handles client-side registry syncing logic

use crate::{
    netty::{
        NettyChannelClient, NettyChannelServer, cosmos_encoder, server_registry::RegistrySyncing,
        sync::resources::client::ResourcesLeftToSync, system_sets::NetworkingSystemsSet,
    },
    registry::{Registry, identifiable::Identifiable},
};
use bevy::{prelude::*, state::state::FreelyMutableState};
use bevy_renet::renet::RenetClient;
use serde::{Serialize, de::DeserializeOwned};

use crate::ecs::add_multi_statebound_resource;

#[derive(Message)]
struct ReceivedRegistryMessage {
    serialized_data: Vec<u8>,
    registry_name: String,
}

#[derive(Debug, Default, Resource)]
struct RegistriesLeftToSync(Option<i64>);

fn sync<T: Identifiable + Serialize + DeserializeOwned + std::fmt::Debug>(
    mut registry: ResMut<Registry<T>>,
    mut ev_reader: MessageReader<ReceivedRegistryMessage>,
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum TransitionStateSet {
    TransitionState,
}

/// Call this function on the client-side to signal that this registry should be synced with the server
pub(super) fn sync_registry<T: Identifiable + Serialize + DeserializeOwned + std::fmt::Debug>(app: &mut App) {
    app.add_systems(
        Update,
        sync::<T>
            .before(TransitionStateSet::TransitionState)
            .in_set(LoadingRegistriesSet::LoadRegistriesFromServer)
            .ambiguous_with(LoadingRegistriesSet::LoadRegistriesFromServer),
    );
}

fn registry_listen_netty(
    mut client: ResMut<RenetClient>,
    mut ev_writer: MessageWriter<ReceivedRegistryMessage>,
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
                ev_writer.write(ReceivedRegistryMessage {
                    serialized_data: serialized,
                    registry_name,
                });
            }
        }
    }
}

#[allow(unused)] // LSP assumes this function is never used, even though it's just feature flagged
pub(super) fn register<T: States + FreelyMutableState + Clone + Copy>(
    app: &mut App,
    connecting_state: T,
    loading_data_state: T,
    loading_world_state: T,
) {
    app.configure_sets(
        Update,
        LoadingRegistriesSet::LoadRegistriesFromServer
            .run_if(in_state(loading_data_state))
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );

    app.configure_sets(Update, TransitionStateSet::TransitionState);

    let transition_state = move |mut client: ResMut<RenetClient>,
                                 mut state_changer: ResMut<NextState<T>>,
                                 loading_registries: Res<RegistriesLeftToSync>,
                                 // TODO: This is very sphegetti, please have a better way of doing this.
                                 loading_resources: Res<ResourcesLeftToSync>| {
        if loading_registries.0.is_some_and(|x| x == 0) && loading_resources.as_ref().0.is_some_and(|x| x == 0) {
            info!("Got all registries & resources from server - loading world!");
            state_changer.set(loading_world_state);
            client.send_message(
                NettyChannelClient::Registry,
                cosmos_encoder::serialize(&crate::netty::client_registry::RegistrySyncing::FinishedReceivingRegistries),
            )
        }
    };

    app.add_systems(
        Update,
        (
            registry_listen_netty.before(LoadingRegistriesSet::LoadRegistriesFromServer),
            transition_state.in_set(TransitionStateSet::TransitionState),
        )
            .run_if(resource_exists::<RegistriesLeftToSync>)
            .run_if(resource_exists::<ResourcesLeftToSync>)
            .chain()
            .run_if(in_state(loading_data_state)),
    )
    .add_event::<ReceivedRegistryMessage>();

    add_multi_statebound_resource::<RegistriesLeftToSync, T>(app, connecting_state, loading_data_state);
}
