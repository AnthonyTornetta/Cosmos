use bevy::prelude::App;

pub mod change_pilot_event;

pub fn register(app: &mut App) {
    change_pilot_event::register(app);
}
