use bevy::{
    app::{App, Update},
    prelude::{Event, EventReader, EventWriter, IntoSystemConfigs, Res, ResMut},
};
use renet2::RenetServer;

use crate::registry::Registry;
use crate::{
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelClient},
    registry::identifiable::Identifiable,
};

use super::netty_event::{NettyEvent, NettyEventMessage, RegisteredNettyEvent};

#[derive(Event)]
struct GotNetworkEvent {
    component_id: u16,
    raw_data: Vec<u8>,
}

fn receive_event(mut server: ResMut<RenetServer>, mut evw_got_event: EventWriter<GotNetworkEvent>) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::NettyEvent) {
            let msg: NettyEventMessage = cosmos_encoder::deserialize(&message).unwrap_or_else(|e| {
                panic!("Failed to parse component replication message from client ({client_id})!\nError: {e:?}");
            });

            match msg {
                NettyEventMessage::SendNettyEvent { component_id, raw_data } => {
                    evw_got_event.send(GotNetworkEvent { component_id, raw_data });
                }
            }
        }
    }
}

fn parse_event<T: NettyEvent>(
    events_registry: Registry<RegisteredNettyEvent>,
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
    }
}

pub(super) fn handle_event<T: NettyEvent>(app: &mut App) {}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, receive_event.in_set(NetworkingSystemsSet::ReceiveMessages));
}
