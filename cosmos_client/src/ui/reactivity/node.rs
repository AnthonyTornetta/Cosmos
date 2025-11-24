//! Reactivity for nodes

use super::{BindValues, NeedsValueFetched, ReactableFields, ReactableValue, ReactiveUiSystemSet};
use bevy::prelude::*;

fn on_update_bound_values<T: ReactableValue>(
    q_react_value: Query<&T>,
    mut ev_reader: MessageReader<NeedsValueFetched>,
    mut q_changed_value: Query<(&mut Node, &BindValues<T>)>,
) {
    for ev in ev_reader.read() {
        let Ok((mut node, bind_values)) = q_changed_value.get_mut(ev.0) else {
            continue;
        };

        for bind_value in bind_values.iter() {
            let Ok(react_value) = q_react_value.get(bind_value.bound_entity) else {
                warn!("Missing bound value for text entity.");
                continue;
            };

            match &bind_value.field {
                ReactableFields::Visibility {
                    hidden_value,
                    visibile_value,
                } => {
                    let value = react_value.as_value();

                    if &value == hidden_value {
                        node.display = Display::None;
                    } else {
                        node.display = *visibile_value;
                    }
                }
                _ => {}
            }
        }
    }
}

pub(super) fn register<T: ReactableValue>(app: &mut App) {
    app.add_systems(Update, (on_update_bound_values::<T>,).chain());
}
