//! Reactivity for text

use super::{BindValues, NeedsValueFetched, ReactableFields, ReactableValue, ReactiveUiSystemSet};
use bevy::{
    app::{App, Update},
    ecs::{event::EventReader, system::Query},
    log::{error, warn},
    prelude::{Entity, IntoSystemConfigs, Text, TextUiWriter, With},
};

fn on_need_update_value<T: ReactableValue>(
    q_react_value: Query<&T>,
    mut ev_reader: EventReader<NeedsValueFetched>,
    mut q_changed_value: Query<(Entity, &BindValues<T>), With<Text>>,
    mut writer: TextUiWriter,
) {
    for ev in ev_reader.read() {
        let Ok((mut text_input_entity, bind_values)) = q_changed_value.get_mut(ev.0) else {
            continue;
        };

        for bind_value in bind_values.iter() {
            let Ok(react_value) = q_react_value.get(bind_value.bound_entity) else {
                warn!("Missing bound value for text entity.");
                continue;
            };

            if let ReactableFields::Text { section } = bind_value.field {
                if let Some(mut section) = writer.get_text(text_input_entity, section) {
                    *section = react_value.as_value();
                } else {
                    error!("Text missing {section} section but is bound to value!");
                }
            }
        }
    }
}

pub(super) fn register<T: ReactableValue>(app: &mut App) {
    app.add_systems(
        Update,
        on_need_update_value::<T>
            .in_set(ReactiveUiSystemSet::ProcessSliderValueChanges)
            .ambiguous_with(ReactiveUiSystemSet::ProcessSliderValueChanges),
    );
}
