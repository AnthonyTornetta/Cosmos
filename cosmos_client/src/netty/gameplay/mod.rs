use bevy::prelude::App;

mod receiver;
mod sync;

pub(super) fn register(app: &mut App) {
    sync::register(app);
    receiver::register(app);
}
