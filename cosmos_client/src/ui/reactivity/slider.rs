//! Reactivity for sliders

use super::{BindValues, NeedsValueFetched, ReactableFields, ReactableValue, ReactiveUiSystemSet};
use crate::ui::components::slider::{Slider, SliderValue};
use bevy::prelude::*;

fn on_update_bound_values<T: ReactableValue>(
    q_react_value: Query<&T>,
    mut ev_reader: EventReader<NeedsValueFetched>,
    mut q_changed_value: Query<(&mut SliderValue, &mut Slider, &BindValues<T>)>,
) {
    for ev in ev_reader.read() {
        let Ok((mut numeric_value, mut slider, bind_values)) = q_changed_value.get_mut(ev.0) else {
            continue;
        };

        for bind_value in bind_values.iter() {
            let Ok(react_value) = q_react_value.get(bind_value.bound_entity) else {
                warn!("Missing bound value for text entity.");
                continue;
            };

            match bind_value.field {
                ReactableFields::Value => {
                    let Ok(val) = react_value.as_value().parse::<i64>() else {
                        error!("Invalid i64 value: {}", react_value.as_value());
                        continue;
                    };

                    if numeric_value.value() != val {
                        numeric_value.set_value(val);
                    }
                }
                ReactableFields::Min => {
                    let Ok(val) = react_value.as_value().parse::<i64>() else {
                        error!("Invalid i64 value: {}", react_value.as_value());
                        continue;
                    };

                    if slider.min != val {
                        slider.min = val;
                    }
                }
                ReactableFields::Max => {
                    let Ok(val) = react_value.as_value().parse::<i64>() else {
                        error!("Invalid i64 value: {}", react_value.as_value());
                        continue;
                    };

                    if slider.max != val {
                        slider.max = val;
                    }
                }
                _ => {}
            }
        }
    }
}

fn on_update_slider_value<T: ReactableValue>(
    mut q_react_value: Query<&mut T>,
    q_changed_value: Query<(&SliderValue, &BindValues<T>), Changed<SliderValue>>,
) {
    for (slider_value, bind_values) in q_changed_value.iter() {
        for bind_value in bind_values.iter() {
            if matches!(bind_value.field, ReactableFields::Value) {
                let Ok(mut react_value) = q_react_value.get_mut(bind_value.bound_entity) else {
                    warn!("Missing bound value for text entity.");
                    continue;
                };

                let num_as_str = format!("{}", slider_value.value());

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
        (on_update_bound_values::<T>, on_update_slider_value::<T>)
            .in_set(ReactiveUiSystemSet::ProcessSliderValueChanges)
            .ambiguous_with(ReactiveUiSystemSet::ProcessSliderValueChanges)
            .chain(),
    );
}
