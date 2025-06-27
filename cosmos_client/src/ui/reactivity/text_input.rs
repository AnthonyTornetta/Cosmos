//! Reactivity for text inputs

use bevy::prelude::*;

use crate::ui::components::text_input::{InputType, InputValue, TextInput};

use super::{BindValues, NeedsValueFetched, ReactableFields, ReactableValue, ReactiveUiSystemSet};

fn on_update_bound_values<T: ReactableValue>(
    q_react_value: Query<&T>,
    mut ev_reader: EventReader<NeedsValueFetched>,
    mut q_changed_value: Query<(&mut InputValue, &mut TextInput, &BindValues<T>)>,
) {
    for ev in ev_reader.read() {
        let Ok((mut input_value, mut text_input, bind_values)) = q_changed_value.get_mut(ev.0) else {
            continue;
        };

        for bind_value in bind_values.iter() {
            let Ok(react_value) = q_react_value.get(bind_value.bound_entity) else {
                warn!("Missing bound value for text entity.");
                continue;
            };

            match bind_value.field {
                ReactableFields::Value => {
                    let react_value = react_value.as_value();

                    if input_value.value() != react_value && !(input_value.value().is_empty() && react_value == "0") {
                        input_value.set_value(react_value);
                    }
                }
                ReactableFields::Max => match &mut text_input.input_type {
                    InputType::Decimal { min: _, max } => {
                        let Ok(val) = react_value.as_value().parse::<f64>() else {
                            error!("Invalid f64 value: {}", react_value.as_value());
                            continue;
                        };

                        *max = val;
                    }
                    InputType::Integer { min: _, max } => {
                        let Ok(val) = react_value.as_value().parse::<i64>() else {
                            error!("Invalid i64 value: {}", react_value.as_value());
                            continue;
                        };

                        *max = val;
                    }
                    _ => {
                        error!("Cannot set Max field on TextInput that isn't of type `InputType::Integer` or `InputType::Decimal`!");
                    }
                },
                ReactableFields::Min => match &mut text_input.input_type {
                    InputType::Decimal { min, max: _ } => {
                        let Ok(val) = react_value.as_value().parse::<f64>() else {
                            error!("Invalid f64 value: {}", react_value.as_value());
                            continue;
                        };

                        *min = val;
                    }
                    InputType::Integer { min, max: _ } => {
                        let Ok(val) = react_value.as_value().parse::<i64>() else {
                            error!("Invalid i64 value: {}", react_value.as_value());
                            continue;
                        };

                        *min = val;
                    }
                    _ => {
                        error!("Cannot set Min field on TextInput that isn't of type `InputType::Integer` or `InputType::Decimal`!");
                    }
                },
                _ => {}
            }
        }
    }
}

fn on_update_text_value<T: ReactableValue>(
    mut q_react_value: Query<&mut T>,
    q_changed_value: Query<(&InputValue, &BindValues<T>), Changed<InputValue>>,
) {
    for (text_input_value, bind_values) in q_changed_value.iter() {
        for bind_value in bind_values.iter() {
            if matches!(bind_value.field, ReactableFields::Value) {
                let Ok(mut react_value) = q_react_value.get_mut(bind_value.bound_entity) else {
                    warn!("Missing bound value for text entity.");
                    continue;
                };

                let num_as_str = text_input_value.value().to_string();

                if react_value.as_value() != num_as_str {
                    react_value.set_from_value(&num_as_str);
                }
            }
        }
    }
}

pub(super) fn register<T: ReactableValue>(app: &mut App) {
    app.add_systems(
        Update,
        (on_update_bound_values::<T>, on_update_text_value::<T>)
            .in_set(ReactiveUiSystemSet::ProcessTextValueChanges)
            .ambiguous_with(ReactiveUiSystemSet::ProcessTextValueChanges),
    );
}
