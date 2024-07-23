//! A framework for adding UI items that will react to variables.
//!
//! Use `BindValues` to bind specific values to this component.

use std::marker::PhantomData;

use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventWriter},
        query::Changed,
        schedule::IntoSystemConfigs,
        schedule::IntoSystemSetConfigs,
        system::Query,
    },
    prelude::{Deref, SystemSet},
};
use cosmos_core::netty::system_sets::NetworkingSystemsSet;

use super::{components::scollable_container::SliderUiSystemSet, UiSystemSet};

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
    Text {
        /// When you make a Text component, that text is composed of sections
        ///
        /// This is the section you want change's index. A new section will NOT be made for this
        /// index if one does not exist, so make sure to create your needed sections first.
        section: usize,
    },
    /// A min field - generally a numeric value
    Min,
    /// A max field - generally a numeric value
    Max,
}

/// A value that can be reacted to
pub trait ReactableValue: Send + Sync + 'static + PartialEq + Component {
    /// Convert whatever value this is into a string representation
    fn as_value(&self) -> String;
    /// Parse this value back from the string representation
    ///
    /// The `new_value` is NOT guarenteed to be a valid representation, so if
    /// you do any parsing be sure to have a default for an invalid form.
    fn set_from_value(&mut self, new_value: &str);
}

#[derive(Component, Deref)]
/// Binds different values to this component.
///
/// You can use multiple BindValues on the same struct as long as they
/// have different generic types.
pub struct BindValues<T: ReactableValue>(Vec<BindValue<T>>);

/// Binds a value to the proper field.
///
/// The `bound_entity` field should corruspond to the entity that has the `T` component on it to read from/write to.
pub struct BindValue<T: ReactableValue> {
    bound_entity: Entity,
    field: ReactableFields,
    _phantom: PhantomData<T>,
}

impl<T: ReactableValue> BindValue<T> {
    /// - The `bound_entity` field should corruspond to the entity that has the `T` component on it to read from/write to.
    /// - The `field` is the marker of the field you want to link to. Be sure to check the component's documentation to see what
    ///   fields you can bind to.
    pub fn new(bound_entity: Entity, field: ReactableFields) -> Self {
        Self {
            bound_entity,
            field,
            _phantom: Default::default(),
        }
    }
}

impl<T: ReactableValue> BindValues<T> {
    /// Binds different values to this component.
    ///
    /// Specify the values to bind in this vec
    pub fn new(items: Vec<BindValue<T>>) -> Self {
        Self(items)
    }

    /// Binds a value to this component.
    ///
    /// Specify the values to bind in this vec
    pub fn single(items: BindValue<T>) -> Self {
        Self::new(vec![items])
    }
}

#[derive(Event)]
/// If this component is on a UI component, then it needs its values fetched from the variable entities it is bound to.
///
/// The entity stored is the entity that holds the values that need fetched.
pub struct NeedsValueFetched(pub Entity);

fn listen_changes_to_reactors<T: ReactableValue>(
    q_bound_listeners: Query<(Entity, &BindValues<T>)>,
    mut q_changed_reactors: Query<Entity, Changed<T>>,
    mut ev_writer: EventWriter<NeedsValueFetched>,
) {
    for ent in q_changed_reactors.iter_mut() {
        for (bound_ent, bound_value) in q_bound_listeners.iter() {
            if bound_value.iter().any(|x| x.bound_entity == ent) {
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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum ReactiveUiSystemSet {
    ProcessTextValueChanges,
    ProcessSliderValueChanges,

    ProcessChanges,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<NeedsValueFetched>();

    app.configure_sets(
        Update,
        (
            ReactiveUiSystemSet::ProcessTextValueChanges,
            ReactiveUiSystemSet::ProcessSliderValueChanges,
            ReactiveUiSystemSet::ProcessChanges,
        )
            .after(SliderUiSystemSet::AddSliderBundle)
            .before(SliderUiSystemSet::SliderInteraction)
            .chain()
            .in_set(NetworkingSystemsSet::Between)
            .in_set(UiSystemSet::DoUi),
    );
}
