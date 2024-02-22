//! An easy way to show/hide the cursor

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

/// A system that returns true if there are no open menus.
///
/// This is particularlly useful if you are dealing with realtime inputs
/// you don't want running with UIs open, such as movement.
///
/// Ex:
/// ```rs
/// app.add_systems(Update, process_movement.run_if(no_open_menus));
/// ```
pub fn no_open_menus(q_show_cursor: Query<(), With<ShowCursor>>) -> bool {
    q_show_cursor.is_empty()
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, show_cursor.in_set(UiSystemSet::FinishUi));
}
