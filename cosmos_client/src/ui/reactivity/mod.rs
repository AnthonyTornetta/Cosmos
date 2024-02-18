use std::marker::PhantomData;

use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventWriter},
        query::Changed,
        schedule::IntoSystemConfigs,
        system::Query,
    },
    prelude::Deref,
};

pub mod slider;
pub mod text;
pub mod text_input;

/// Use this to signify which field type should be reacted to for this variable
///
/// This is turbo ugly, please come up with a better solution that isn't all shoved into one enum.
pub enum ReactableFields {
    /// A value that is set or constantly displayed
    Value,
    /// A text field (aka label)
    Text { section: usize },
    /// A min field - generally a numeric value
    Min,
    /// A max field - generally a numeric value
    Max,
}

pub trait ReactableValue: Send + Sync + 'static + PartialEq + Component {
    fn as_value(&self) -> String;
    fn set_from_value(&mut self, new_value: &str);
}

#[derive(Component, Deref)]
pub struct BindValues<T: ReactableValue>(Vec<BindValue<T>>);

pub struct BindValue<T: ReactableValue> {
    bound_entity: Entity,
    field: ReactableFields,
    _phantom: PhantomData<T>,
}

impl<T: ReactableValue> BindValue<T> {
    pub fn new(bound_entity: Entity, field: ReactableFields) -> Self {
        Self {
            bound_entity,
            field,
            _phantom: Default::default(),
        }
    }
}

impl<T: ReactableValue> BindValues<T> {
    pub fn new(items: Vec<BindValue<T>>) -> Self {
        Self(items)
    }
}

#[derive(Event)]
pub struct NeedsValueFetched(Entity);

fn listen_changes_to_reactors<T: ReactableValue>(
    q_bound_listeners: Query<(Entity, &BindValues<T>)>,
    mut q_changed_reactors: Query<Entity, Changed<T>>,
    mut ev_writer: EventWriter<NeedsValueFetched>,
) {
    for ent in q_changed_reactors.iter_mut() {
        for (bound_ent, bound_value) in q_bound_listeners.iter() {
            if bound_value.iter().any(|x| x.bound_entity == ent) {
                // commands.entity(bound_ent).insert(NeedsValueFetched);
                ev_writer.send(NeedsValueFetched(bound_ent));
            }
        }
    }
}

pub(crate) fn add_reactable_type<T: ReactableValue>(app: &mut App) {
    app.add_systems(Update, (listen_changes_to_reactors::<T>,).chain());

    slider::register::<T>(app);
    text::register::<T>(app);
    text_input::register::<T>(app);
}

pub(super) fn register(app: &mut App) {
    app.add_event::<NeedsValueFetched>();
}
