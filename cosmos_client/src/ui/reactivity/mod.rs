use std::marker::PhantomData;

use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter},
        query::{Changed, With, Without},
        schedule::IntoSystemConfigs,
        system::{Commands, Query},
    },
};

pub mod slider;
pub mod text;

#[derive(Component)]
pub struct ReactValueAsString(String);

pub enum ReactionType {}

pub struct ValueReactor {
    pub reaction_types: Vec<ReactionType>,
    pub data_entity: Entity,
}

pub trait ReactableValue: Send + Sync + 'static + PartialEq + Component {
    fn as_value(&self) -> String;
    fn set_from_value(&mut self, new_value: &str);
}

// trait Register {
//     fn register(app: &mut App);
// }

// impl<T: Send + Sync + 'static> Register for React<T> {
//     default fn register(app: &mut App) {
//         // app.add_systems(Update, (()));
//     }
// }

// impl<T: BindableValue> React<T> {
//     fn register(app: &mut App) {}
// }

#[derive(Component)]
pub struct BindValue<K: Send + Sync + 'static> {
    bound_entity: Entity,
    _phantom: PhantomData<K>,
}

impl<K: Send + Sync + 'static> BindValue<K> {
    pub fn new(bound_entity: Entity) -> Self {
        Self {
            bound_entity,
            _phantom: Default::default(),
        }
    }
}

#[derive(Component)]
pub struct NeedsValueFetched;

#[derive(Event)]
struct ReactValueUpdated {
    value_entity: Entity,
}

fn notify_listeners<K: Send + Sync + 'static>(
    mut ev_listeners: EventReader<ReactValueUpdated>,
    q_bound_listeners: Query<(Entity, &BindValue<K>)>,
    mut commands: Commands,
) {
    for ev in ev_listeners.read() {
        for (bound_ent, bound_value) in q_bound_listeners.iter() {
            if bound_value.bound_entity == ev.value_entity {
                commands.entity(bound_ent).insert(NeedsValueFetched);
            }
        }
    }
}

fn listen_changes<T: ReactableValue>(
    // mut commands: Commands,
    // q_bound_listeners: Query<(Entity, &BindValue<K>)>,
    mut q_changed_reactors: Query<(Entity, &mut T, &ReactValueAsString), Changed<ReactValueAsString>>,
    mut ev_writer: EventWriter<ReactValueUpdated>,
) {
    for (ent, mut react, value_as_string) in q_changed_reactors.iter_mut() {
        if react.as_value() == value_as_string.0 {
            continue;
        }

        react.set_from_value(&value_as_string.0);

        ev_writer.send(ReactValueUpdated { value_entity: ent });

        // for (bound_ent, bound_value) in q_bound_listeners.iter() {
        //     if bound_value.bound_entity == ent {
        //         // commands.entity(bound_ent).insert(NeedsValueFetched { storage_entity: ent });
        //     }
        // }
    }
}

fn on_change_react_value<T: ReactableValue>(mut q_changed_react: Query<(&T, &mut ReactValueAsString), Changed<T>>) {
    for (react, mut changed_value) in q_changed_react.iter_mut() {
        changed_value.0 = react.as_value();
    }
}

fn monitor_needs_react_value<T: ReactableValue>(
    mut commands: Commands,
    q_needs_react_value: Query<(Entity, &T), (With<T>, Without<ReactValueAsString>)>,
) {
    for (ent, t) in &q_needs_react_value {
        commands.entity(ent).insert(ReactValueAsString(t.as_value()));
    }
}

pub(crate) fn add_reactable_type<T: ReactableValue>(app: &mut App) {
    app.add_systems(
        Update,
        (monitor_needs_react_value::<T>, listen_changes::<T>, on_change_react_value::<T>).chain(),
    );
}

pub(crate) fn add_reactable_component_type<K: Send + Sync + 'static>(app: &mut App) {
    app.add_systems(Update, notify_listeners::<K>);
}

pub(super) fn register(app: &mut App) {
    app.add_event::<ReactValueUpdated>();

    text::register(app);
    slider::register(app);
}
