use bevy::prelude::*;

pub(super) mod coms_request;
mod main_ui;

pub(super) fn register(app: &mut App) {
    main_ui::register(app);
    coms_request::register(app);
}
