//! Events that occur when ships need their movement changed

use bevy::prelude::{in_state, App, Entity, Event, EventReader, IntoSystemConfigs, Query, Update};
use cosmos_core::structure::ship::ship_movement::ShipMovement;

use crate::state::game_state::GameState;

/// If this event is received, the movement of that ship should be changed
#[derive(Debug, Event)]
pub struct SetShipMovementEvent {
    /// The ship's entity
    pub ship_entity: Entity,
    /// What the movement should be
    pub ship_movement: ShipMovement,
}

fn update_ship_movement(mut event_reader: EventReader<SetShipMovementEvent>, mut query: Query<&mut ShipMovement>) {
    for ev in event_reader.iter() {
        if let Ok(mut movement) = query.get_mut(ev.ship_entity) {
            movement.set(&ev.ship_movement);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<SetShipMovementEvent>()
        .add_systems(Update, update_ship_movement.run_if(in_state(GameState::Playing)));
}
