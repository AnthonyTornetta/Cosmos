//! Client-related economy logic

use bevy::{app::App, log::error};
use cosmos_core::economy::Credits;

use crate::ui::reactivity::{ReactableValue, add_reactable_type};

impl ReactableValue for Credits {
    fn as_value(&self) -> String {
        format!("{}", self.amount())
    }

    fn set_from_value(&mut self, new_value: &str) {
        let Ok(val) = new_value.parse::<u64>() else {
            error!("Unable to parse '{new_value}' to u64!");
            return;
        };

        self.set_amount(val);
    }
}

pub(super) fn register(app: &mut App) {
    add_reactable_type::<Credits>(app);
}
