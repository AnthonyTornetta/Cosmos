use bevy::app::App;

mod pirate;

pub(super) fn register(app: &mut App) {
    pirate::register(app);
}
