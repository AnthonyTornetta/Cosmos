use bevy::{
    ecs::{
        event::{MessageId, SendBatchIds},
        system::SystemParam,
    },
    prelude::*,
};
use renet::{ClientId, RenetServer};

use crate::{
    netty::{NettyChannelClient, NettyChannelServer, cosmos_encoder, system_sets::NetworkingSystemsSet},
    registry::identifiable::Identifiable,
};
use crate::{registry::Registry, state::GameState};

use super::netty_event::{MessageReceiver, NettyMessage, NettyMessageMessage, RegisteredNettyMessage};

#[derive(Message)]
pub(super) struct GotNetworkMessage {
    pub component_id: u16,
    pub raw_data: Vec<u8>,
    pub client_id: renet::ClientId,
}

#[derive(Message, Debug)]
/// Send this event before the [`NetworkingSystemsSet::SyncComponents`] set to automatically have
/// the inner event sent to the client.
pub struct NettyMessageToSend<T: NettyMessage> {
    /// The event to send
    pub event: T,
    /// The clients to send this to or [`None`] to broadcast this to everyone.
    pub client_ids: Option<Vec<ClientId>>,
}

#[derive(Deref, Message, Debug)]
/// An event received from a client.
///
/// Read via [`MessageReader<NettyMessageReceived<T>>`]
pub struct NettyMessageReceived<T: NettyMessage> {
    #[deref]
    /// The actual event received from the client
    pub event: T,
    /// The client that sent this event
    pub client_id: ClientId,
}

/// Send your [`NettyMessage`] via this before [`NetworkingSystemsSet::SyncComponents`] to have it
/// automatically sent to the client.
#[derive(SystemParam)]
pub struct NettyMessageWriter<'w, T: NettyMessage> {
    ev_writer: MessageWriter<'w, NettyMessageToSend<T>>,
}

impl<E: NettyMessage> NettyMessageWriter<'_, E> {
    /// Sends an `event`, which can later be read by [`MessageReader`]s.
    /// This method returns the [ID](`MessageId`) of the sent `event`.
    ///
    /// See [`bevy::prelude::Messages`] for details.
    ///
    /// If you wish to send this event to all clients, see [`Self::broadcast`].
    pub fn write(&mut self, event: E, client_id: ClientId) -> MessageId<NettyMessageToSend<E>> {
        self.ev_writer.write(NettyMessageToSend {
            event,
            client_ids: Some(vec![client_id]),
        })
    }

    /// Sends an `event`, which can later be read by [`MessageReader`]s.
    /// This method returns the [ID](`MessageId`) of the sent `event`.
    ///
    /// See [`bevy::prelude::Messages`] for details.
    ///
    /// If you wish to send this event to all clients, see [`Self::broadcast`].
    pub fn write_to_many(&mut self, event: E, client_ids: impl Iterator<Item = ClientId>) -> MessageId<NettyMessageToSend<E>> {
        self.ev_writer.write(NettyMessageToSend {
            event,
            client_ids: Some(client_ids.collect::<Vec<_>>()),
        })
    }

    /// Sends an `event`, which can later be read by [`MessageReader`]s.
    /// This method returns the [ID](`MessageId`) of the sent `event`.
    ///
    /// See [`bevy::prelude::Messages`] for details.
    pub fn broadcast(&mut self, event: E) -> MessageId<NettyMessageToSend<E>> {
        self.ev_writer.write(NettyMessageToSend { event, client_ids: None })
    }

    /// Sends a list of `events` all at once, which can later be read by [`MessageReader`]s.
    /// This is more efficient than sending each event individually.
    /// This method returns the [IDs](`MessageId`) of the sent `events`.
    ///
    /// See [`bevy::prelude::Messages`] for details.
    pub fn write_batch(
        &mut self,
        events: impl IntoIterator<Item = E>,
        client_ids: Option<Vec<ClientId>>,
    ) -> SendBatchIds<NettyMessageToSend<E>> {
        self.ev_writer.write_batch(events.into_iter().map(|event| NettyMessageToSend {
            event,
            client_ids: client_ids.clone(),
        }))
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    /// This method returns the [ID](`MessageId`) of the sent `event`.
    ///
    /// See [`bevy::prelude::Messages`] for details.
    pub fn write_default(&mut self, client_ids: Option<Vec<ClientId>>) -> MessageId<NettyMessageToSend<E>>
    where
        E: Default,
    {
        self.ev_writer.write(NettyMessageToSend {
            event: E::default(),
            client_ids,
        })
    }
}

fn receive_event(mut server: ResMut<RenetServer>, mut evw_got_event: MessageWriter<GotNetworkMessage>) {
    for client_id in server.clients_id().into_iter() {
        while let Some(message) = server.receive_message(client_id, NettyChannelClient::NettyMessage) {
            let msg: NettyMessageMessage = cosmos_encoder::deserialize(&message).unwrap_or_else(|e| {
                panic!("Failed to parse component replication message from client ({client_id})!\nError: {e:?}");
            });

            match msg {
                NettyMessageMessage::SendNettyMessage { component_id, raw_data } => {
                    evw_got_event.write(GotNetworkMessage {
                        component_id,
                        raw_data,
                        client_id,
                    });
                }
            }
        }
    }
}

fn parse_event<T: NettyMessage>(
    events_registry: Res<Registry<RegisteredNettyMessage>>,
    mut evw_custom_event: MessageWriter<NettyMessageReceived<T>>,
    mut evr_need_parsed: MessageReader<GotNetworkMessage>,
) {
    let Some(registered_event) = events_registry.from_id(T::unlocalized_name()) else {
        return;
    };

    for ev in evr_need_parsed.read() {
        if ev.component_id != registered_event.id() {
            continue;
        }

        let Ok(event) = cosmos_encoder::deserialize_uncompressed::<T>(&ev.raw_data).map_err(|e| {
            error!(
                "Got invalid event from client! ({}) {e:?} - {:?}",
                T::unlocalized_name(),
                ev.raw_data
            )
        }) else {
            continue;
        };

        evw_custom_event.write(NettyMessageReceived {
            event,
            client_id: ev.client_id,
        });
    }
}

fn send_events<T: NettyMessage>(
    mut server: ResMut<RenetServer>,
    mut evr: MessageReader<NettyMessageToSend<T>>,
    netty_event_registry: Res<Registry<RegisteredNettyMessage>>,
) {
    for ev in evr.read() {
        let Some(registered_event) = netty_event_registry.from_id(T::unlocalized_name()) else {
            error!("Message {} not regstered!\n{:?}", T::unlocalized_name(), netty_event_registry);
            continue;
        };

        let serialized = cosmos_encoder::serialize_uncompressed(&ev.event);

        if let Some(client_id) = &ev.client_ids {
            for client_id in client_id.iter().skip(1) {
                server.send_message(
                    *client_id,
                    NettyChannelServer::NettyMessage,
                    cosmos_encoder::serialize(&NettyMessageMessage::SendNettyMessage {
                        component_id: registered_event.id(),
                        raw_data: serialized.clone(),
                    }),
                );
            }

            if let Some(client_id) = client_id.first() {
                server.send_message(
                    *client_id,
                    NettyChannelServer::NettyMessage,
                    cosmos_encoder::serialize(&NettyMessageMessage::SendNettyMessage {
                        component_id: registered_event.id(),
                        raw_data: serialized,
                    }),
                );
            }
        } else {
            server.broadcast_message(
                NettyChannelServer::NettyMessage,
                cosmos_encoder::serialize(&NettyMessageMessage::SendNettyMessage {
                    component_id: registered_event.id(),
                    raw_data: serialized,
                }),
            );
        }
    }
}

fn server_receive_event<T: NettyMessage>(app: &mut App) {
    app.add_systems(Update, parse_event::<T>.in_set(NetworkingSystemsSet::ReceiveMessages))
        .add_event::<NettyMessageReceived<T>>();
}

fn server_send_event<T: NettyMessage>(app: &mut App) {
    app.add_systems(Update, send_events::<T>.in_set(NetworkingSystemsSet::SyncComponents))
        .add_event::<NettyMessageToSend<T>>();
}

fn register_event_type_impl<T: NettyMessage>(mut registry: ResMut<Registry<RegisteredNettyMessage>>) {
    registry.register(RegisteredNettyMessage {
        id: 0,
        unlocalized_name: T::unlocalized_name().into(),
    });
}

fn register_event_type<T: NettyMessage>(app: &mut App) {
    app.add_systems(OnEnter(GameState::Loading), register_event_type_impl::<T>);
}

pub(super) fn register_event<T: NettyMessage>(app: &mut App) {
    register_event_type::<T>(app);

    if T::event_receiver() == MessageReceiver::Server || T::event_receiver() == MessageReceiver::Both {
        server_receive_event::<T>(app);
    }
    if T::event_receiver() == MessageReceiver::Client || T::event_receiver() == MessageReceiver::Both {
        server_send_event::<T>(app);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        Update,
        receive_event
            .run_if(resource_exists::<RenetServer>)
            .in_set(NetworkingSystemsSet::ReceiveMessages),
    )
    .add_event::<GotNetworkMessage>();
}
