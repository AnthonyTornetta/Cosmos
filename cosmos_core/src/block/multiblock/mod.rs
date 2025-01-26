//! Common multiblock functionality

use bevy::prelude::App;

pub mod reactor;

// enum Multiblock {
//     Blueprint { layout: Vec<String>, key: HashMap<String, String> },
//     Multiblock { size: BlockCoordinate, blocks: Vec<u16> },
// }

pub(super) fn register(app: &mut App) {
    reactor::register(app);
}
