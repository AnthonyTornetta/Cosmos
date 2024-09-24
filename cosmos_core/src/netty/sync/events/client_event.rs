use bevy::{
    app::{App, Update},
    log::error,
    prelude::{resource_exists, Event, EventReader, EventWriter, IntoSystemConfigs, Res, ResMut},
};
use renet2::RenetClient;

use crate::{netty::NettyChannelServer, registry::Registry};
use crate::{
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelClient},
    registry::identifiable::Identifiable,
};

use super::netty_event::{GotNetworkEvent, NettyEvent, NettyEventMessage, RegisteredNettyEvent};

#[derive(Event)]
/// Send this event before the [`NetworkingSystemsSet::SyncComponents`] set to automatically have
/// the inner event sent to the server.
pub struct NettyEventToSend<T: NettyEvent>(pub T);

/// Send your [`NettyEvent`] via this before [`NetworkingSystemsSet::SyncComponents`] to have it
/// automatically sent to the server.
pub type NettyEventWriter<'w, T> = EventWriter<'w, NettyEventToSend<T>>;

fn send_events<T: NettyEvent>(
    mut client: ResMut<RenetClient>,
    mut evr: EventReader<NettyEventToSend<T>>,
    netty_event_registry: Res<Registry<RegisteredNettyEvent>>,
) {
    for ev in evr.read() {
        let Some(registered_event) = netty_event_registry.from_id(T::unlocalized_name()) else {
            continue;
        };

        let serialized = bincode::serialize(&ev.0).unwrap();
        client.send_message(
            NettyChannelClient::NettyEvent,
            cosmos_encoder::serialize(&NettyEventMessage::SendNettyEvent {
                component_id: registered_event.id(),
                raw_data: serialized,
            }),
        );
    }
}

fn receive_events(mut server: ResMut<RenetClient>, mut evw_got_event: EventWriter<GotNetworkEvent>) {
    while let Some(message) = server.receive_message(NettyChannelServer::NettyEvent) {
        let msg: NettyEventMessage = cosmos_encoder::deserialize(&message).unwrap_or_else(|e| {
            panic!("Failed to parse component replication message from server!\nError: {e:?}");
        });

        match msg {
            NettyEventMessage::SendNettyEvent { component_id, raw_data } => {
                evw_got_event.send(GotNetworkEvent { component_id, raw_data });
            }
        }
    }
}

fn parse_event<T: NettyEvent>(
    events_registry: Res<Registry<RegisteredNettyEvent>>,
    mut evw_custom_event: EventWriter<T>,
    mut evr_need_parsed: EventReader<GotNetworkEvent>,
) {
    let Some(registered_event) = events_registry.from_id(T::unlocalized_name()) else {
        return;
    };

    for ev in evr_need_parsed.read() {
        if ev.component_id != registered_event.id() {
            continue;
        }

        let Ok(event) = bincode::deserialize::<T>(&ev.raw_data) else {
            error!("Got invalid event from server!");
            continue;
        };

        evw_custom_event.send(event);
    }
}

pub(super) fn client_send_event<T: NettyEvent>(app: &mut App) {
    app.add_systems(Update, send_events::<T>);
    app.add_event::<NettyEventToSend<T>>();
}

pub(super) fn client_receive_event<T: NettyEvent>(app: &mut App) {
    app.add_systems(Update, parse_event::<T>);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        receive_events
            .run_if(resource_exists::<RenetClient>)
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );
}
