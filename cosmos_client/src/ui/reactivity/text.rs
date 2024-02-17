use bevy::{
    app::{App, Update},
    ecs::{query::With, system::Query},
    log::warn,
    text::Text,
};

use super::{add_reactable_component_type, BindValue, NeedsValueFetched, ReactValueAsString};

pub struct TextBinding;

fn on_need_update(
    q_react_value: Query<&ReactValueAsString>,
    mut q_changed_value: Query<(&mut Text, &BindValue<TextBinding>), With<NeedsValueFetched>>,
) {
    for (mut text, bind_value) in q_changed_value.iter_mut() {
        let Some(sec) = text.sections.get_mut(0) else {
            warn!("Text needs at least one section to be updated properly!");
            continue;
        };
        let Ok(value) = q_react_value.get(bind_value.bound_entity) else {
            warn!("Missing bound value for text entity.");
            continue;
        };

        sec.value = value.0.to_owned();
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_component_type::<TextBinding>(app);

    app.add_systems(Update, on_need_update);
}
