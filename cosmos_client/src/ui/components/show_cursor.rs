use bevy::{
    app::{App, Update},
    ecs::{
        component::Component,
        query::With,
        schedule::IntoSystemConfigs,
        system::{Query, ResMut},
    },
};

use crate::{ui::UiSystemSet, window::setup::CursorFlags};

#[derive(Component, Default, Debug, Clone, Copy)]
/// If any entity has this component, the cursor will be shown.
///
/// Otherwise, the cursor will be hidden.
pub struct ShowCursor;

fn show_cursor(mut cursor_flags: ResMut<CursorFlags>, q_show_cursor: Query<(), With<ShowCursor>>) {
    if q_show_cursor.iter().len() == 0 {
        if cursor_flags.is_cursor_shown() {
            cursor_flags.hide();
        }
    } else if !cursor_flags.is_cursor_shown() {
        cursor_flags.show();
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, show_cursor.in_set(UiSystemSet::FinishUi));
}
