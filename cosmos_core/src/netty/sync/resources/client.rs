//! Handles client-side resource  syncing logic

use crate::{
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelServer},
    state::GameState,
};
use bevy::{
    app::{App, Update},
    ecs::{
        event::{Event, EventReader, EventWriter},
        schedule::IntoSystemConfigs,
        system::{ResMut, Resource},
    },
    log::{error, info},
    prelude::{Commands, Condition, IntoSystemSetConfigs, SystemSet},
    state::condition::in_state,
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

        if left_to_sync.0.unwrap_or(0) != 0 {
            let new_amt = left_to_sync.0.expect("This should never happen") - 1;

            left_to_sync.0 = Some(new_amt);

            info!("Got resource from server: {}! Need {} more.", ev.resource_name, new_amt);
        }

        let Ok(new_resource) = bincode::deserialize::<T>(&ev.serialized_data) else {
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

pub(super) fn register(app: &mut App) {
    let condition = in_state(GameState::LoadingData).or(in_state(GameState::LoadingWorld).or(in_state(GameState::Playing)));

    app.configure_sets(
        Update,
        LoadingResourcesSet::LoadResourcesFromServer
            .run_if(condition.clone())
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );

    app.configure_sets(Update, TransitionStateSet::TransitionState);

    app.add_systems(
        Update,
        (resources_listen_netty.before(LoadingResourcesSet::LoadResourcesFromServer),)
            .chain()
            .run_if(condition),
    )
    .add_event::<ReceivedResourceEvent>();

    add_multi_statebound_resource::<ResourcesLeftToSync, GameState>(app, GameState::Connecting, GameState::LoadingData);
}
