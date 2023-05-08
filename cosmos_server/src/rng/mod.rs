//! Contains useful features for randomly generated numbers

use rand::SeedableRng;
use rand_chacha::ChaCha8Rng;

use crate::init::init_world::ServerSeed;

/// Generates a seed given the sector & the server's base seed.
///
/// If you just need a random number generator, prefer to use [`get_rng_for_sector`]
pub fn get_seed_for_sector_u64(server_seed: &ServerSeed, sector: (i64, i64, i64)) -> u64 {
    let (sx, sy, sz) = sector;

    (server_seed.as_u64() as i64)
        .wrapping_add(sx)
        .wrapping_mul(if sy != 0 { sy } else { 1 })
        .wrapping_add(sy)
        .wrapping_mul(if sx != 0 { sx } else { 1 })
        .wrapping_add(sy)
        .wrapping_mul(if sy != 0 { sz } else { 1 })
        .wrapping_add(sz).unsigned_abs()
}

/// Generates a seed given the sector & the server's base seed.
///
/// If you just need a random number generator, prefer to use [`get_rng_for_sector`]
pub fn get_seed_for_sector_u32(server_seed: &ServerSeed, sector: (i64, i64, i64)) -> u32 {
    get_seed_for_sector_u64(server_seed, sector) as u32
}

/// Generates a random number generator given the sector & the server's base seed.
pub fn get_rng_for_sector(server_seed: &ServerSeed, sector: (i64, i64, i64)) -> ChaCha8Rng {
    ChaCha8Rng::seed_from_u64(get_seed_for_sector_u64(server_seed, sector))
}
