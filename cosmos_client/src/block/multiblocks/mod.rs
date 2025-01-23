use bevy::app::App;

mod reactor;

pub(super) fn register(app: &mut App) {
    reactor::register(app);
}
