//! Common multiblock functionality

use bevy::prelude::{App, States};

pub mod reactor;

// enum Multiblock {
//     Blueprint { layout: Vec<String>, key: HashMap<String, String> },
//     Multiblock { size: BlockCoordinate, blocks: Vec<u16> },
// }

pub(super) fn register<T: States>(app: &mut App, post_loading_state: T) {
    reactor::register(app, post_loading_state);
}
