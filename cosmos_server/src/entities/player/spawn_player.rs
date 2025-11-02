//! Utilities around spawning a new player

use bevy::prelude::*;

#[cfg(doc)]
use cosmos_core::ecs::sets::FixedUpdateSet;

use cosmos_core::physics::location::{Location, SystemCoordinate};

use crate::universe::{SystemItem, UniverseSystems};

/// Sent whenever a new player is being created
///
/// Should be read after [`FixedUpdateSet::Main`]
#[derive(Debug, Message)]
pub struct CreateNewPlayerMessage(Entity);

impl CreateNewPlayerMessage {
    pub(super) fn new(player_ent: Entity) -> Self {
        Self(player_ent)
    }

    /// Returns the entity of the player this is discussing
    pub fn player(&self) -> Entity {
        self.0
    }
}

pub(super) fn find_new_player_location(universe_systems: &UniverseSystems) -> Option<(Location, Quat)> {
    let (shop, _) = universe_systems
        .system(SystemCoordinate::default())
        .iter()
        .flat_map(|x| x.iter())
        .filter(|x| matches!(x.item, SystemItem::Shop))
        .map(|shop| {
            (
                shop,
                universe_systems
                    .iter()
                    .flat_map(|(_, x)| x.iter())
                    .filter(|x| matches!(x.item, SystemItem::Asteroid(_)))
                    .map(|x| x.location.distance_sqrd(&shop.location) as i64)
                    .min(),
            )
        })
        .min_by_key(|x| x.1.unwrap_or(i64::MAX))?;

    let offset = Vec3::new(rand::random::<f32>() * 10.0 - 5.0, 3.0, rand::random::<f32>() * 10.0 - 5.0);

    Some((shop.location + shop.rotation * offset, shop.rotation))
}

pub(super) fn register(app: &mut App) {
    app.add_event::<CreateNewPlayerMessage>();
}
