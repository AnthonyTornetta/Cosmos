//! Server-side coms logic

use bevy::prelude::*;

mod systems;

#[derive(Event)]
/// This NPC is being hailed by a player
struct RequestHailNpc {
    pub player_ship: Entity,
}

pub(super) fn register(app: &mut App) {
    systems::register(app);

    app.add_event::<RequestHailNpc>();
}
