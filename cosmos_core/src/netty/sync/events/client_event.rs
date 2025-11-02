use bevy::{
    ecs::{
        event::{MessageId, SendBatchIds},
        system::SystemParam,
    },
    prelude::*,
};
use renet::RenetClient;

use crate::{
    netty::{
        NettyChannelClient, NettyChannelServer,
        cosmos_encoder::{self, serialize_uncompressed},
        sync::mapping::NetworkMapping,
        system_sets::NetworkingSystemsSet,
    },
    registry::Registry,
    registry::identifiable::Identifiable,
    state::GameState,
};

use super::netty_event::{MessageReceiver, NettyMessage, NettyMessageMessage, RegisteredNettyMessage};

#[derive(Message)]
pub(super) struct GotNetworkMessage {
    pub component_id: u16,
    pub raw_data: Vec<u8>,
}

#[derive(Message, Default, Debug)]
/// Send this event before the [`NetworkingSystemsSet::SyncComponents`] set to automatically have
/// the inner event sent to the server.
pub struct NettyMessageToSend<T: NettyMessage>(pub T);

/// An event received from the server.
///
/// Read this via an [`MessageReader<NettyMessageReceived<T>>`].
pub type NettyMessageReceived<T> = T;

/// Send your [`NettyMessage`] via this before [`NetworkingSystemsSet::SyncComponents`] to have it
/// automatically sent to the server.
#[derive(SystemParam)]
pub struct NettyMessageWriter<'w, T: NettyMessage> {
    ev_writer: MessageWriter<'w, NettyMessageToSend<T>>,
}

impl<E: NettyMessage> NettyMessageWriter<'_, E> {
    /// Sends an `event`, which can later be read by [`MessageReader`]s.
    /// This method returns the [ID](`MessageId`) of the sent `event`.
    ///
    /// See [`Messages`] for details.
    pub fn write(&mut self, event: E) -> MessageId<NettyMessageToSend<E>> {
        self.ev_writer.write(NettyMessageToSend(event))
    }

    /// Sends a list of `events` all at once, which can later be read by [`MessageReader`]s.
    /// This is more efficient than sending each event individually.
    /// This method returns the [IDs](`MessageId`) of the sent `events`.
    ///
    /// See [`Messages`] for details.
    pub fn write_batch(&mut self, events: impl IntoIterator<Item = E>) -> SendBatchIds<NettyMessageToSend<E>> {
        self.ev_writer.write_batch(events.into_iter().map(|x| NettyMessageToSend(x)))
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    /// This method returns the [ID](`MessageId`) of the sent `event`.
    ///
    /// See [`Messages`] for details.
    pub fn write_default(&mut self) -> MessageId<NettyMessageToSend<E>>
    where
        E: Default,
    {
        self.ev_writer.write_default()
    }
}

fn send_events<T: NettyMessage>(
    mut client: ResMut<RenetClient>,
    mut evr: MessageReader<NettyMessageToSend<T>>,
    netty_event_registry: Res<Registry<RegisteredNettyMessage>>,
    mapping: Res<NetworkMapping>,
) {
    for ev in evr.read() {
        let Some(registered_event) = netty_event_registry.from_id(T::unlocalized_name()) else {
            warn!(
                "Message not registered to be properly sent -- {}\n{:?}",
                T::unlocalized_name(),
                netty_event_registry
            );
            continue;
        };

        let serialized = if T::needs_entity_conversion() {
            let Some(x) = ev.0.clone().convert_entities_client_to_server(&mapping) else {
                warn!(
                    "Unable to convert entity to server entity for {}! \n {:?}",
                    T::unlocalized_name(),
                    ev.0
                );
                continue;
            };

            serialize_uncompressed(&x)
        } else {
            serialize_uncompressed(&ev.0)
        };

        client.send_message(
            NettyChannelClient::NettyMessage,
            cosmos_encoder::serialize(&NettyMessageMessage::SendNettyMessage {
                component_id: registered_event.id(),
                raw_data: serialized,
            }),
        );
    }
}

fn receive_events(mut client: ResMut<RenetClient>, mut evw_got_event: MessageWriter<GotNetworkMessage>) {
    while let Some(message) = client.receive_message(NettyChannelServer::NettyMessage) {
        let Some(msg) = cosmos_encoder::deserialize::<NettyMessageMessage>(&message)
            .map(Some)
            .unwrap_or_else(|e| {
                error!("Failed to parse netty event message from server!\nBytes: {message:?}\nError: {e:?}");
                None
            })
        else {
            error!("Error deserializing message into `NettyMessageMessage`");
            continue;
        };

        match msg {
            NettyMessageMessage::SendNettyMessage { component_id, raw_data } => {
                evw_got_event.write(GotNetworkMessage { component_id, raw_data });
            }
        }
    }
}

fn parse_event<T: NettyMessage>(
    events_registry: Res<Registry<RegisteredNettyMessage>>,
    mut evw_custom_event: MessageWriter<T>,
    mut evr_need_parsed: MessageReader<GotNetworkMessage>,
    netty_mapping: Res<NetworkMapping>,
) {
    for ev in evr_need_parsed.read() {
        let Some(registered_event) = events_registry.from_id(T::unlocalized_name()) else {
            error!("Unregistered event: {}", T::unlocalized_name());
            return;
        };

        if ev.component_id != registered_event.id() {
            continue;
        }

        let event = match cosmos_encoder::deserialize_uncompressed::<T>(&ev.raw_data) {
            Err(e) => {
                error!("Got invalid event from server (parsing error)! {e:?}");
                continue;
            }
            Ok(event) => event,
        };

        let event = if T::needs_entity_conversion() {
            let Some(event) = event.convert_entities_server_to_client(&netty_mapping) else {
                error!("Unable to convert event to client entity event!");
                continue;
            };
            event
        } else {
            event
        };

        evw_custom_event.write(event);
    }
}

pub(super) fn client_send_event<T: NettyMessage>(app: &mut App) {
    app.add_systems(
        Update,
        send_events::<T>
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(resource_exists::<RenetClient>),
    );
    app.add_event::<NettyMessageToSend<T>>();
}

pub(super) fn client_receive_event<T: NettyMessage>(app: &mut App) {
    app.add_systems(
        Update,
        parse_event::<T>
            .in_set(NetworkingSystemsSet::ReceiveMessages)
            .after(receive_events)
            .run_if(in_state(GameState::Playing).or(in_state(GameState::LoadingWorld))),
    )
    .add_event::<T>();
}

pub(super) fn register_event<T: NettyMessage>(app: &mut App) {
    if T::event_receiver() == MessageReceiver::Client || T::event_receiver() == MessageReceiver::Both {
        client_receive_event::<T>(app);
    }
    if T::event_receiver() == MessageReceiver::Server || T::event_receiver() == MessageReceiver::Both {
        client_send_event::<T>(app);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        receive_events
            .run_if(resource_exists::<RenetClient>)
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    )
    .add_event::<GotNetworkMessage>();
}
