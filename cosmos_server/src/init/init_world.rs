//! Handles the initialization of the server world

use std::{
    fs,
    num::Wrapping,
    sync::{Arc, RwLock, RwLockReadGuard},
};

use bevy::prelude::*;
use cosmos_core::netty::cosmos_encoder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Resource, Deref, Serialize, Deserialize, Clone, Copy)]
/// This sets the seed the server uses to generate the universe
pub struct ServerSeed(u64);

impl ServerSeed {
    /// Gets the u64 representation of this seed
    pub fn as_u64(&self) -> u64 {
        self.0
    }

    /// Gets the u32 representation of this seed
    pub fn as_u32(&self) -> u32 {
        self.0 as u32
    }

    /// Computes a "random" number at the given x, y, z coordinates.
    ///
    /// This randomness is based off a hash of the coordinates with this seed.
    pub fn chaos_hash(&self, x: f64, y: f64, z: f64) -> i64 {
        let wrapping_seed = Wrapping(self.0 as i64);

        let mut h =
            wrapping_seed + (Wrapping((x * 374761393.0) as i64) + Wrapping((y * 668265263.0) as i64) + Wrapping((z * 1610612741.0) as i64)); //all constants are prime

        h = (h ^ (h >> 13)) * Wrapping(1274126177);
        (h ^ Wrapping(h.0 >> 16)).0
    }
}

#[derive(Resource, Debug, Clone, Deref, DerefMut)]
/// A pre-seeded structure to create noise values. Uses simplex noise as the backend
///
/// This cannot be sent across threads - use ReadOnlyNoise to send use across threads.
pub struct Noise(noise::OpenSimplex);

#[derive(Resource, Debug, Clone)]
/// A thread-safe pre-seeded structure to create noise values. Uses simplex noise as the backend
///
/// To use across threads, just clone this and call the `inner` method to get the Noise struct this encapsulates
pub struct ReadOnlyNoise(Arc<RwLock<Noise>>);

impl ReadOnlyNoise {
    /// Returns the `Noise` instance this encapsulates
    pub fn inner(&self) -> RwLockReadGuard<Noise> {
        self.0.read().expect("Failed to read")
    }
}

pub(super) fn register(app: &mut App) {
    let server_seed = if let Ok(seed) = fs::read("./world/seed.dat") {
        cosmos_encoder::deserialize::<ServerSeed>(&seed).expect("Unable to understand './world/seed.dat' seed file. Is it corrupted?")
    } else {
        let seed = ServerSeed(rand::random());

        fs::create_dir("./world/").expect("Error creating world directory!");
        fs::write("./world/seed.dat", cosmos_encoder::serialize(&seed)).expect("Error writing file './world/seed.dat'");

        seed
    };

    let noise = Noise(noise::OpenSimplex::new(server_seed.as_u32()));
    let read_noise = ReadOnlyNoise(Arc::new(RwLock::new(noise.clone())));

    app.insert_resource(noise).insert_resource(read_noise).insert_resource(server_seed);
}
