use bevy::{
    app::{App, Update},
    ecs::{
        event::{EventId, SendBatchIds},
        system::SystemParam,
    },
    log::{error, warn},
    prelude::{resource_exists, Deref, Event, EventReader, EventWriter, IntoSystemConfigs, Res, ResMut},
};
use renet2::RenetClient;

use crate::registry::Registry;
use crate::{
    netty::{cosmos_encoder, system_sets::NetworkingSystemsSet, NettyChannelClient},
    registry::identifiable::Identifiable,
};

use super::netty_event::{EventReceiver, NettyEvent, NettyEventMessage, RegisteredNettyEvent};

#[derive(Event)]
pub(super) struct GotNetworkEvent {
    pub component_id: u16,
    pub raw_data: Vec<u8>,
}

#[derive(Event, Default, Debug)]
/// Send this event before the [`NetworkingSystemsSet::SyncComponents`] set to automatically have
/// the inner event sent to the server.
pub struct NettyEventToSend<T: NettyEvent>(pub T);

#[derive(Deref, Event, Debug)]
/// An event received from the server.
///
/// Read this via an [`EventReader<NettyEventReceived<T>>`].
pub struct NettyEventReceived<T: NettyEvent>(pub T);

/// Send your [`NettyEvent`] via this before [`NetworkingSystemsSet::SyncComponents`] to have it
/// automatically sent to the server.
#[derive(SystemParam)]
pub struct NettyEventWriter<'w, T: NettyEvent> {
    ev_writer: EventWriter<'w, NettyEventToSend<T>>,
}

impl<E: NettyEvent> NettyEventWriter<'_, E> {
    /// Sends an `event`, which can later be read by [`EventReader`]s.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    ///
    /// See [`Events`] for details.
    pub fn send(&mut self, event: E) -> EventId<NettyEventToSend<E>> {
        self.ev_writer.send(NettyEventToSend(event))
    }

    /// Sends a list of `events` all at once, which can later be read by [`EventReader`]s.
    /// This is more efficient than sending each event individually.
    /// This method returns the [IDs](`EventId`) of the sent `events`.
    ///
    /// See [`Events`] for details.
    pub fn send_batch(&mut self, events: impl IntoIterator<Item = E>) -> SendBatchIds<NettyEventToSend<E>> {
        self.ev_writer.send_batch(events.into_iter().map(|x| NettyEventToSend(x)))
    }

    /// Sends the default value of the event. Useful when the event is an empty struct.
    /// This method returns the [ID](`EventId`) of the sent `event`.
    ///
    /// See [`Events`] for details.
    pub fn send_default(&mut self) -> EventId<NettyEventToSend<E>>
    where
        E: Default,
    {
        self.ev_writer.send_default()
    }
}

fn send_events<T: NettyEvent>(
    mut client: ResMut<RenetClient>,
    mut evr: EventReader<NettyEventToSend<T>>,
    netty_event_registry: Res<Registry<RegisteredNettyEvent>>,
) {
    for ev in evr.read() {
        let Some(registered_event) = netty_event_registry.from_id(T::unlocalized_name()) else {
            warn!(
                "Event not registered to be properly sent -- {}\n{:?}",
                T::unlocalized_name(),
                netty_event_registry
            );
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

fn receive_events(mut client: ResMut<RenetClient>, mut evw_got_event: EventWriter<GotNetworkEvent>) {
    while let Some(message) = client.receive_message(NettyChannelClient::NettyEvent) {
        let Some(msg) = cosmos_encoder::deserialize::<NettyEventMessage>(&message)
            .map(Some)
            .unwrap_or_else(|e| {
                error!("Failed to parse netty event message from server!\nBytes: {message:?}\nError: {e:?}");
                None
            })
        else {
            continue;
        };

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
    for ev in evr_need_parsed.read() {
        let Some(registered_event) = events_registry.from_id(T::unlocalized_name()) else {
            error!("Unregistered event: {}", T::unlocalized_name());
            return;
        };

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
    app.add_systems(
        Update,
        send_events::<T>
            .in_set(NetworkingSystemsSet::SyncComponents)
            .run_if(resource_exists::<RenetClient>),
    );
    app.add_event::<NettyEventToSend<T>>();
}

pub(super) fn client_receive_event<T: NettyEvent>(app: &mut App) {
    app.add_systems(
        Update,
        parse_event::<T>.in_set(NetworkingSystemsSet::ReceiveMessages).after(receive_events),
    )
    .add_event::<T>();
}

pub(super) fn register_event<T: NettyEvent>(app: &mut App) {
    app.add_event::<NettyEventReceived<T>>();

    if T::event_receiver() == EventReceiver::Client || T::event_receiver() == EventReceiver::Both {
        client_receive_event::<T>(app);
    }
    if T::event_receiver() == EventReceiver::Server || T::event_receiver() == EventReceiver::Both {
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
    .add_event::<GotNetworkEvent>();
}
