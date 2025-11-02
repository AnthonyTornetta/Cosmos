//! Provides a wrapper around a default bevy event that makes it thread-safe and mutable

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bevy::{
    ecs::message::MessageWriter,
    prelude::{App, Message},
};

#[derive(Message)]
/// Same as a bevy Message, but you can read & write to it
pub struct MutMessage<E: Message + Send + Sync + 'static>(Arc<RwLock<E>>);

impl<E: Message> MutMessage<E> {
    /// Reads the contents of this event
    pub fn read(&self) -> RwLockReadGuard<E> {
        self.0.read().unwrap()
    }

    /// Writes to the contents of this event
    pub fn write(&self) -> RwLockWriteGuard<E> {
        self.0.write().unwrap()
    }
}

impl<E: Message> From<E> for MutMessage<E> {
    fn from(value: E) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

/// Custom send function for bevy `MessageWriter`s that will automatically call `into` for you.
pub trait MessageWriterCustomSend<E: Message> {
    /// Custom send function for bevy `MessageWriter`s that will automatically call `into` for you.
    ///
    /// ```rs
    /// event_writer.send_mut(e);
    /// // is the same as
    /// event_writer.write(e.into());
    /// // is the same as
    /// event_writer.write(MutMessage::from(e));
    /// ```
    fn send_mut(&mut self, e: impl Into<MutMessage<E>>);
}

impl<E: Message + Send + Sync + 'static> MessageWriterCustomSend<E> for MessageWriter<'_, MutMessage<E>> {
    fn send_mut(&mut self, e: impl Into<MutMessage<E>>) {
        self.write(e.into());
    }
}

/// Adds a mutable event that can be used via an MessageReader & Writer
///
/// Add your own mutable event via `App::add_mut_event(&mut self, event: Message)`
pub trait MutMessagesCommand {
    /// Adds a mutable event that can be used via an MessageReader & Writer
    ///
    /// Example usage:
    /// ```rs
    /// fn read_system(mut event_reader: MessageReader<MutMessage<MessageType>>) {
    ///     for ev in event_reader.iter() {
    ///         // Read:
    ///         {
    ///             let event = ev.read();
    ///             info!("{event:?}");
    ///         }
    ///         // Or write:
    ///         {
    ///             let event = ev.write();
    ///             event.mutable_thing();
    ///         }
    ///     }
    /// }
    ///
    /// fn send_system(mut event_writer: MessageWriter<MutMessage<MessageType>>) {
    ///     event_writer.write(MessageType::default().into());
    /// }
    /// ```
    fn add_mut_event<E: Message>(&mut self) -> &mut Self;
}

impl MutMessagesCommand for App {
    fn add_mut_event<E: Message>(&mut self) -> &mut Self {
        self.add_message::<MutMessage<E>>();

        self
    }
}
