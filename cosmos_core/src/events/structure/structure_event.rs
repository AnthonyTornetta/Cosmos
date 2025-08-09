//! Utilities for working for structure-based events

use bevy::{
    ecs::{
        entity::Entity,
        event::{Event, EventIterator},
    },
    platform::collections::HashMap,
};

/// An event that contains an entity referencing a single structure
pub trait StructureEvent: Event {
    /// Returns the structure entity this event is referencing
    fn structure_entity(&self) -> Entity;
}

/// Something that iterates over structure events
pub trait StructureEventIterator<E: StructureEvent> {
    /// Groups the results of this iterator into a HashMap of <StructureEntity, Vec<&StructureEvent>>
    fn group_by_structure(&mut self) -> HashMap<Entity, Vec<&E>>;
}

impl<E: StructureEvent> StructureEventIterator<E> for EventIterator<'_, E> {
    fn group_by_structure(&mut self) -> HashMap<Entity, Vec<&E>> {
        let mut grouped: HashMap<Entity, Vec<&E>> = HashMap::default();
        for ev in self {
            grouped.entry(ev.structure_entity()).or_default().push(ev);
        }
        grouped
    }
}
