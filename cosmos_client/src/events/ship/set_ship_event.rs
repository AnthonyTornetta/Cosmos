use bevy::prelude::{App, Entity, EventReader, IntoSystemConfig, OnUpdate, Query};
use cosmos_core::structure::ship::ship_movement::ShipMovement;

use crate::state::game_state::GameState;

pub struct SetShipMovementEvent {
    pub ship_entity: Entity,
    pub ship_movement: ShipMovement,
}

fn update_ship_movement(
    mut event_reader: EventReader<SetShipMovementEvent>,
    mut query: Query<&mut ShipMovement>,
) {
    for ev in event_reader.iter() {
        if let Ok(mut movement) = query.get_mut(ev.ship_entity) {
            movement.set(&ev.ship_movement);
        }
    }
}

pub fn register(app: &mut App) {
    app.add_event::<SetShipMovementEvent>()
        .add_system(update_ship_movement.in_set(OnUpdate(GameState::Playing)));
}
