// use bevy::{
//     prelude::Resource,
//     reflect::{FromReflect, Reflect},
//     platform::collections::HashSet,
// };

// pub struct CancellableMessage<T> {
//     pub event: T,
//     id: u64,
// }

// impl<T> CancellableMessage<T> {
//     pub fn new(event: T, event_manager: &mut CancellableMessageManager) -> Self {
//         Self {
//             event,
//             id: event_manager.new_event_entry(),
//         }
//     }

//     pub fn cancel(&self, event_manager: &mut CancellableMessageManager) {
//         event_manager.cancel_event(self.id);
//     }

//     pub fn is_active(&self, event_manager: &mut CancellableMessageManager) -> bool {
//         event_manager.is_event_active(self.id)
//     }

//     pub fn unwrap_and_send()
// }

// #[derive(Resource, Reflect, FromReflect, Debug, Default)]
// pub struct CancellableMessageManager {
//     active_events: HashSet<u64>,
//     next_id: u64,
// }

// impl CancellableMessageManager {
//     fn new_event_entry(&mut self) -> u64 {
//         self.next_id += 1;

//         self.active_events.insert(self.next_id);

//         self.next_id
//     }

//     /// Marks an event as inactive
//     pub fn finish_event(&mut self, id: u64) {
//         // It's fine if this removes nothing
//         self.active_events.remove(&id);
//     }

//     /// Marks an event as inactive
//     pub fn cancel_event(&mut self, id: u64) {
//         // It's fine if this removes nothing
//         self.active_events.remove(&id);
//     }

//     /// Returns true if an event with this id exists & has not been cancelled.
//     pub fn is_event_active(&mut self, id: u64) -> bool {
//         self.active_events.contains(&id)
//     }
// }
