//! Provides a wrapper around a default bevy event that makes it thread-safe and mutable

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bevy::{
    ecs::event::EventWriter,
    prelude::{App, Event},
};

#[derive(Event)]
/// Same as a bevy Event, but you can read & write to it
pub struct MutEvent<E: Event + Send + Sync + 'static>(Arc<RwLock<E>>);

impl<E: Event> MutEvent<E> {
    /// Reads the contents of this event
    pub fn read(&self) -> RwLockReadGuard<E> {
        self.0.read().unwrap()
    }

    /// Writes to the contents of this event
    pub fn write(&self) -> RwLockWriteGuard<E> {
        self.0.write().unwrap()
    }
}

impl<E: Event> From<E> for MutEvent<E> {
    fn from(value: E) -> Self {
        Self(Arc::new(RwLock::new(value)))
    }
}

/// Custom send function for bevy `EventWriter`s that will automatically call `into` for you.
pub trait EventWriterCustomSend<E: Event> {
    /// Custom send function for bevy `EventWriter`s that will automatically call `into` for you.
    ///
    /// ```rs
    /// event_writer.send_mut(e);
    /// // is the same as
    /// event_writer.write(e.into());
    /// // is the same as
    /// event_writer.write(MutEvent::from(e));
    /// ```
    fn send_mut(&mut self, e: impl Into<MutEvent<E>>);
}

impl<E: Event + Send + Sync + 'static> EventWriterCustomSend<E> for EventWriter<'_, MutEvent<E>> {
    fn send_mut(&mut self, e: impl Into<MutEvent<E>>) {
        self.write(e.into());
    }
}

/// Adds a mutable event that can be used via an EventReader & Writer
///
/// Add your own mutable event via `App::add_mut_event(&mut self, event: Event)`
pub trait MutEventsCommand {
    /// Adds a mutable event that can be used via an EventReader & Writer
    ///
    /// Example usage:
    /// ```rs
    /// fn read_system(mut event_reader: EventReader<MutEvent<EventType>>) {
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
    /// fn send_system(mut event_writer: EventWriter<MutEvent<EventType>>) {
    ///     event_writer.write(EventType::default().into());
    /// }
    /// ```
    fn add_mut_event<E: Event>(&mut self) -> &mut Self;
}

impl MutEventsCommand for App {
    fn add_mut_event<E: Event>(&mut self) -> &mut Self {
        self.add_event::<MutEvent<E>>();

        self
    }
}
