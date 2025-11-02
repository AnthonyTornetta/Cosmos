//! Utilities for working for structure-based events

use bevy::{
    ecs::{
        entity::Entity,
        message::{Message, MessageIterator},
    },
    platform::collections::HashMap,
};

/// An event that contains an entity referencing a single structure
pub trait StructureMessage: Message {
    /// Returns the structure entity this event is referencing
    fn structure_entity(&self) -> Entity;
}

/// Something that iterates over structure events
pub trait StructureMessageIterator<E: StructureMessage> {
    /// Groups the results of this iterator into a HashMap of <StructureEntity, Vec<&StructureMessage>>
    fn group_by_structure(&mut self) -> HashMap<Entity, Vec<&E>>;
}

impl<E: StructureMessage> StructureMessageIterator<E> for MessageIterator<'_, E> {
    fn group_by_structure(&mut self) -> HashMap<Entity, Vec<&E>> {
        let mut grouped: HashMap<Entity, Vec<&E>> = HashMap::default();
        for ev in self {
            grouped.entry(ev.structure_entity()).or_default().push(ev);
        }
        grouped
    }
}
