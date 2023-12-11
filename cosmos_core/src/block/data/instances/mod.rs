//! Instances of block data types - used for shared logic between server + client (generally deserialization)

use bevy::app::App;

pub mod storage;

pub(super) fn register(app: &mut App) {
    storage::register(app);
}
