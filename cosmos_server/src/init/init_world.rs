//! Handles the initialization of the server world

use bevy::prelude::*;
use cosmos_core::utils::resource_wrapper::ResourceWrapper;
use noise::Seedable;

#[derive(Debug, Resource, Deref)]
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
}

pub(super) fn register(app: &mut App) {
    let noise = noise::OpenSimplex::default();

    let server_seed = ServerSeed(rand::random());

    noise.set_seed(server_seed.as_u32());

    app.insert_resource(ResourceWrapper(noise))
        .insert_resource(server_seed);
}
