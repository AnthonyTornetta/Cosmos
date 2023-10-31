use bevy::prelude::App;

pub mod reactor;

pub(super) fn register(app: &mut App) {
    reactor::register(app);
}
