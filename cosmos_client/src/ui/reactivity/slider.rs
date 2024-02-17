use bevy::{app::App, ecs::system::Query, text::Text};

use crate::ui::components::text_input::InputValue;

use super::{NeedsValueFetched, ReactValueAsString};

fn on_need_update(q_react_value: Query<&ReactValueAsString>, mut q_changed_value: Query<(&mut InputValue, &NeedsValueFetched)>) {
    for (mut text, value_holder) in q_changed_value.iter_mut() {
        let Some(sec) = text.sections.get_mut(0) else {
            warn!("Text needs at least one section to be updated properly!");
            continue;
        };
        let Ok(value) = q_react_value.get(value_holder.storage_entity) else {
            warn!("Missing bound value for text entity.");
            continue;
        };

        sec.value = value.0.to_owned();
    }
}

pub(super) fn register(app: &mut App) {}
