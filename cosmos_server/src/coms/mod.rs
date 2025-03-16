use bevy::prelude::*;

mod systems;

#[derive(Event)]
/// This NPC is being hailed by a player
pub struct RequestHailNpc {
    player: Entity,
}

pub(super) fn register(app: &mut App) {
    systems::register(app);

    app.add_event::<RequestHailNpc>();
}
