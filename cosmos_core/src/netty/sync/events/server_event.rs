use bevy::{
    app::{App, Update},
    log::error,
    prelude::{resource_exists, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Res, ResMut, States},
};
use renet2::{ClientId, RenetServer};

use crate::registry::Registry;
use crate::{
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelClient},
    registry::identifiable::Identifiable,
};

use super::netty_event::{GotNetworkEvent, NettyEvent, NettyEventMessage, RegisteredNettyEvent};

fn receive_event(mut server: ResMut<RenetServer>, mut evw_got_event: EventWriter<GotNetworkEvent>) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::NettyEvent) {
            let msg: NettyEventMessage = cosmos_encoder::deserialize(&message).unwrap_or_else(|e| {
                panic!("Failed to parse component replication message from client ({client_id})!\nError: {e:?}");
            });

            match msg {
                NettyEventMessage::SendNettyEvent { component_id, raw_data } => {
                    println!("Sending comp id: {component_id}");
                    evw_got_event.send(GotNetworkEvent { component_id, raw_data });
                }
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
            error!("Got invalid event from client!");
            continue;
        };

        println!("Received: {event:?}");

        evw_custom_event.send(event);
    }
}

#[derive(Event)]
/// Send this event before the [`NetworkingSystemsSet::SyncComponents`] set to automatically have
/// the inner event sent to the server.
pub struct NettyEventToSend<T: NettyEvent> {
    /// The event to send
    pub event: T,
    /// The client to send this to or [`None`] to broadcast this to everyone.
    pub client_id: Option<ClientId>,
}

fn send_events<T: NettyEvent>(
    mut server: ResMut<RenetServer>,
    mut evr: EventReader<NettyEventToSend<T>>,
    netty_event_registry: Res<Registry<RegisteredNettyEvent>>,
) {
    for ev in evr.read() {
        let Some(registered_event) = netty_event_registry.from_id(T::unlocalized_name()) else {
            continue;
        };

        let serialized = bincode::serialize(&ev.event).unwrap();

        if let Some(client_id) = ev.client_id {
            server.send_message(
                client_id,
                NettyChannelClient::NettyEvent,
                cosmos_encoder::serialize(&NettyEventMessage::SendNettyEvent {
                    component_id: registered_event.id(),
                    raw_data: serialized,
                }),
            );
        } else {
            server.broadcast_message(
                NettyChannelClient::NettyEvent,
                cosmos_encoder::serialize(&NettyEventMessage::SendNettyEvent {
                    component_id: registered_event.id(),
                    raw_data: serialized,
                }),
            );
        }
    }
}

pub(super) fn server_receive_event<T: NettyEvent>(app: &mut App) {
    app.add_systems(Update, parse_event::<T>.in_set(NetworkingSystemsSet::ReceiveMessages))
        .add_event::<NettyEventToSend<T>>();
}

pub(super) fn server_send_event<T: NettyEvent>(app: &mut App) {
    app.add_systems(Update, send_events::<T>.in_set(NetworkingSystemsSet::SyncComponents))
        .add_event::<NettyEventToSend<T>>();
}

fn register_event_type_impl<T: NettyEvent>(mut registry: ResMut<Registry<RegisteredNettyEvent>>) {
    registry.register(RegisteredNettyEvent {
        id: 0,
        unlocalized_name: T::unlocalized_name().into(),
    });
}

pub(super) fn register_event_type<T: NettyEvent, S: States>(app: &mut App, loading_state: S) {
    app.add_systems(OnEnter(loading_state), register_event_type_impl::<T>);
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        receive_event
            .run_if(resource_exists::<RenetServer>)
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    );
}
