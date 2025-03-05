//! Handles client-side resource  syncing logic

use crate::netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelServer};
use bevy::{
    app::{App, Update},
    ecs::{
        event::{Event, EventReader, EventWriter},
        schedule::{common_conditions::resource_exists, IntoSystemConfigs},
        system::{ResMut, Resource},
    },
    log::{error, info},
    prelude::{Commands, IntoSystemSetConfigs, States, SystemSet},
    state::{condition::in_state, state::FreelyMutableState},
};
use bevy_renet2::renet2::RenetClient;

use crate::ecs::add_multi_statebound_resource;

use super::{ResourceSyncingMessage, SyncableResource};

#[derive(Event)]
struct ReceivedResourceEvent {
    serialized_data: Vec<u8>,
    resource_name: String,
}

#[derive(Debug, Default, Resource)]
pub(crate) struct ResourcesLeftToSync(pub Option<i64>);

fn sync<T: SyncableResource>(
    mut commands: Commands,
    mut ev_reader: EventReader<ReceivedResourceEvent>,
    mut left_to_sync: ResMut<ResourcesLeftToSync>,
) {
    for ev in ev_reader.read() {
        if ev.resource_name != T::unlocalized_name() {
            continue;
        }

        let new_amt = left_to_sync.0.unwrap_or(0) - 1;

        left_to_sync.0 = Some(new_amt);

        info!("Got resource from server: {}! Need {} more.", ev.resource_name, new_amt);

        let Ok(new_resource) = cosmos_encoder::deserialize::<T>(&ev.serialized_data) else {
            error!("Got bad resource data from server - {}!", ev.resource_name);
            continue;
        };

        commands.insert_resource(new_resource);
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum LoadingResourcesSet {
    LoadResourcesFromServer,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
enum TransitionStateSet {
    TransitionState,
}

/// Call this function on the client-side to signal that this resource should be synced with the server
pub(super) fn sync_resource<T: SyncableResource>(app: &mut App) {
    app.add_systems(
        Update,
        sync::<T>
            .before(TransitionStateSet::TransitionState)
            .in_set(LoadingResourcesSet::LoadResourcesFromServer)
            .ambiguous_with(LoadingResourcesSet::LoadResourcesFromServer),
    );
}

fn resources_listen_netty(
    mut client: ResMut<RenetClient>,
    mut ev_writer: EventWriter<ReceivedResourceEvent>,
    mut resource_count: ResMut<ResourcesLeftToSync>,
) {
    while let Some(message) = client.receive_message(NettyChannelServer::Resource) {
        let msg: ResourceSyncingMessage = cosmos_encoder::deserialize(&message).expect("Unable to parse resource sync from server");

        match msg {
            ResourceSyncingMessage::ResourceCount(count) => {
                info!("Need to load {count} resources from server.");
                resource_count.0 = Some(count as i64 + resource_count.0.unwrap_or(0));
            }
            ResourceSyncingMessage::Resource { data, unlocalized_name } => {
                ev_writer.send(ReceivedResourceEvent {
                    serialized_data: data,
                    resource_name: unlocalized_name,
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
        LoadingResourcesSet::LoadResourcesFromServer
            .run_if(in_state(loading_data_state))
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );

    app.configure_sets(Update, TransitionStateSet::TransitionState);

    app.add_systems(
        Update,
        (resources_listen_netty.before(LoadingResourcesSet::LoadResourcesFromServer),)
            .run_if(resource_exists::<ResourcesLeftToSync>)
            .chain()
            .run_if(in_state(loading_data_state)),
    )
    .add_event::<ReceivedResourceEvent>();

    add_multi_statebound_resource::<ResourcesLeftToSync, T>(app, connecting_state, loading_data_state);
}
