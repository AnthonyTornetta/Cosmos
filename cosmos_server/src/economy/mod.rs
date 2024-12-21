use bevy::prelude::App;
use cosmos_core::economy::Credits;

use crate::persistence::make_persistent::{make_persistent, PersistentComponent};

impl PersistentComponent for Credits {}

pub(super) fn register(app: &mut App) {
    make_persistent::<Credits>(app);
}
