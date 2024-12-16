//! Systems for spawning a completely new player

use bevy::prelude::*;

use cosmos_core::physics::location::{Location, Sector};

use crate::universe::generation::{SystemItem, UniverseSystems};

const DEFAULT_STARTING_SECTOR: Location = Location::new(Vec3::new(0.0, 2000.0, 0.0), Sector::new(25, 25, 25));

pub(super) fn find_new_player_location(universe_systems: &UniverseSystems) -> Location {
    let Some(shop) = universe_systems
        .iter()
        .flat_map(|(_, x)| x.iter())
        .find(|x| matches!(x.item, SystemItem::Shop))
    else {
        warn!("No shops found in universe! Starting player at fallback sector.");
        return DEFAULT_STARTING_SECTOR;
    };

    let offset = Vec3::new(rand::random::<f32>() * 10.0 - 5.0, 3.0, rand::random::<f32>() * 10.0 - 5.0);

    shop.location + offset
}
