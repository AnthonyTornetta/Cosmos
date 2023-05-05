use bevy::prelude::*;
use cosmos_core::utils::resource_wrapper::ResourceWrapper;
use noise::Seedable;

pub(super) fn register(app: &mut App) {
    let noise = noise::OpenSimplex::default();

    noise.set_seed(rand::random());

    app.insert_resource(ResourceWrapper(noise));
}
