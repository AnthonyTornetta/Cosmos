//! Systems for spawning a completely new player

use std::i64;

use bevy::prelude::*;

use cosmos_core::physics::location::{Location, Sector};

use crate::universe::generation::{SystemItem, UniverseSystems};

const DEFAULT_STARTING_SECTOR: Location = Location::new(Vec3::new(0.0, 2000.0, 0.0), Sector::new(25, 25, 25));

pub(super) fn find_new_player_location(universe_systems: &UniverseSystems) -> Location {
    let Some((shop, _)) = universe_systems
        .iter()
        .flat_map(|(_, x)| x.iter())
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
        warn!("No shops found in universe! Starting player at fallback sector.");
        return DEFAULT_STARTING_SECTOR;
    };

    let offset = Vec3::new(rand::random::<f32>() * 10.0 - 5.0, 3.0, rand::random::<f32>() * 10.0 - 5.0);

    shop.location + offset
}
