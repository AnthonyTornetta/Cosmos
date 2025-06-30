//! Systems for spawning a completely new player

use bevy::prelude::*;

use cosmos_core::physics::location::{Location, SystemCoordinate};

use crate::universe::{SystemItem, UniverseSystems};

pub(super) fn find_new_player_location(universe_systems: &UniverseSystems) -> Option<(Location, Quat)> {
    let Some((shop, _)) = universe_systems
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
        .min_by_key(|x| x.1.unwrap_or(i64::MAX))
    else {
        return None;
    };

    let offset = Vec3::new(rand::random::<f32>() * 10.0 - 5.0, 3.0, rand::random::<f32>() * 10.0 - 5.0);

    Some((shop.location + shop.rotation * offset, shop.rotation))
}
