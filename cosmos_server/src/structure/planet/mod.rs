//! Contains server-side logic for the planet

use bevy::prelude::*;
use cosmos_core::physics::location::Sector;

use crate::init::init_world::ServerSeed;

pub mod biosphere;
pub mod chunk;
pub mod generation;
pub mod persistence;
mod planet_rotation;
mod sync;

/// Produces a stable, well-mixed terrain seed from the server seed and planet sector.
pub(crate) fn terrain_seed_for(server_seed: &ServerSeed, sector: &Sector) -> u64 {
    let mut value = server_seed.as_u64();
    value ^= (sector.x() as u64).wrapping_mul(0xD6E8_FEB8_6659_FD93);
    value ^= (sector.y() as u64).wrapping_mul(0xA5A3_56E4_E27F_886D);
    value ^= (sector.z() as u64).wrapping_mul(0x9E37_79B9_7F4A_7C15);
    value = value.wrapping_add(0x94D0_49BB_1331_11EB);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

pub(super) fn register(app: &mut App) {
    planet_rotation::register(app);
    biosphere::register(app);
    persistence::register(app);
    sync::register(app);
    generation::register(app);
    chunk::register(app);
}
