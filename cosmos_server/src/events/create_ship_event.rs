//! Handles the event of ships being created by the player

use bevy::prelude::*;
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::physics::location::Location;
use cosmos_core::physics::player_world::PlayerWorld;
use cosmos_core::structure::{ship::ship_builder::TShipBuilder, Structure};

use crate::structure::ship::{loading::ShipNeedsCreated, server_ship_builder::ServerShipBuilder};
use crate::GameState;

/// This event is done when a ship is being created
pub struct CreateShipEvent {
    /// Starting location of the ship
    pub ship_location: Location,
    /// The rotation of the ship
    pub rotation: Quat,
}

fn event_reader(
    mut event_reader: EventReader<CreateShipEvent>,
    player_worlds: Query<&Location, With<PlayerWorld>>,
    mut commands: Commands,
) {
    for ev in event_reader.iter() {
        let mut best_loc = None;
        let mut best_dist = f32::INFINITY;

        for loc in player_worlds.iter() {
            let dist = loc.distance_sqrd(&ev.ship_location);
            if dist < best_dist {
                best_dist = dist;
                best_loc = Some(loc);
            }
        }

        if let Some(world_location) = best_loc {
            let mut entity = commands.spawn_empty();

            let mut structure = Structure::new(10, 10, 10);

            let builder = ServerShipBuilder::default();

            builder.insert_ship(
                &mut entity,
                ev.ship_location,
                world_location,
                Velocity::zero(),
                &mut structure,
            );

            entity.insert(structure).insert(ShipNeedsCreated);
        }
    }
}

pub(super) fn register(app: &mut App) {
    app.add_event::<CreateShipEvent>()
        .add_system(event_reader.in_set(OnUpdate(GameState::Playing)));
}
