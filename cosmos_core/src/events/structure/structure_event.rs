use bevy::{
    ecs::{
        entity::Entity,
        event::{Event, EventIterator},
    },
    platform::collections::HashMap,
};

pub trait StructureEvent: Event {
    fn structure_entity(&self) -> Entity;
}

pub trait StructureEventIterator<E: StructureEvent> {
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
