//! Provides a wrapper around a default bevy event that makes it thread-safe and mutable

use std::sync::{Arc, RwLock, RwLockReadGuard, RwLockWriteGuard};

use bevy::prelude::{App, Event};

#[derive(Event)]
/// Same as a bevy Event, but you can read & write to it
pub struct MutEvent<E: Event>(Arc<RwLock<E>>);

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

trait MutEventsCommand {
    fn add_mut_event<E: Event>(&mut self);
}

impl MutEventsCommand for App {
    fn add_mut_event<E: Event>(&mut self) {
        self.add_event::<MutEvent<E>>();
    }
}
