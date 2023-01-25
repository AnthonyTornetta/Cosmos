use bevy::prelude::App;

pub mod receiver;
pub mod sync;

pub(crate) fn register(app: &mut App) {
    sync::register(app);
    receiver::register(app);
}
