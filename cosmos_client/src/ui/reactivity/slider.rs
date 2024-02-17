use bevy::{
    app::{App, Update},
    ecs::{query::With, system::Query},
    log::{error, warn},
};

use crate::ui::components::{slider::Slider, text_input::InputValue};

use super::{add_reactable_component_type, BindValue, NeedsValueFetched, ReactValueAsString};

struct SliderValueBinding;
struct SliderMinBinding;
struct SliderMaxBinding;

fn on_need_update_value(
    q_react_value: Query<&ReactValueAsString>,
    mut q_changed_value: Query<(&mut InputValue, &BindValue<SliderValueBinding>), With<NeedsValueFetched>>,
) {
    for (mut text_input_value, bind_value) in q_changed_value.iter_mut() {
        let Ok(react_value) = q_react_value.get(bind_value.bound_entity) else {
            warn!("Missing bound value for text entity.");
            continue;
        };

        text_input_value.set_value(react_value.0.to_owned());
    }
}

fn on_need_update_min_bound(
    q_react_value: Query<&ReactValueAsString>,
    mut q_changed_value: Query<(&mut Slider, &BindValue<SliderMinBinding>), With<NeedsValueFetched>>,
) {
    for (mut slider, bind_value) in q_changed_value.iter_mut() {
        let Ok(react_value) = q_react_value.get(bind_value.bound_entity) else {
            warn!("Missing bound value for text entity.");
            continue;
        };

        let Ok(val) = react_value.0.to_owned().parse::<i64>() else {
            error!("Invalid i64 value: {}", react_value.0);
            continue;
        };

        slider.min = val;
    }
}

fn on_need_update_max_bound(
    q_react_value: Query<&ReactValueAsString>,
    mut q_changed_value: Query<(&mut Slider, &BindValue<SliderMaxBinding>), With<NeedsValueFetched>>,
) {
    for (mut slider, bind_value) in q_changed_value.iter_mut() {
        let Ok(react_value) = q_react_value.get(bind_value.bound_entity) else {
            warn!("Missing bound value for text entity.");
            continue;
        };

        let Ok(val) = react_value.0.to_owned().parse::<i64>() else {
            error!("Invalid i64 value: {}", react_value.0);
            continue;
        };

        slider.max = val;
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_component_type::<SliderValueBinding>(app);
    add_reactable_component_type::<SliderMinBinding>(app);
    add_reactable_component_type::<SliderMaxBinding>(app);

    app.add_systems(Update, (on_need_update_value, on_need_update_min_bound, on_need_update_max_bound));
}
