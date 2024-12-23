use bevy::prelude::App;
use cosmos_core::economy::Credits;

use crate::persistence::make_persistent::{make_persistent, DefaultPersistentComponent};

impl DefaultPersistentComponent for Credits {}

pub(super) fn register(app: &mut App) {
    make_persistent::<Credits>(app);
}
