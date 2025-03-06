use bevy::prelude::*;
use cosmos_core::faction::Factions;

pub(super) fn register(app: &mut App) {
    app.init_resource::<Factions>();
}
