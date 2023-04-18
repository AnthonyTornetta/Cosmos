use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    structure::{planet::planet_builder::TPlanetBuilder, Structure},
    utils::resource_wrapper::ResourceWrapper,
};
use noise::Seedable;

use crate::structure::planet::{
    biosphere::{grass_biosphere::GrassBiosphere, TBiosphere},
    generation::planet_generator::NeedsGenerated,
    server_planet_builder::ServerPlanetBuilder,
};

pub(super) fn register(app: &mut App) {
    let noise = noise::OpenSimplex::default();

    noise.set_seed(rand::random());

    app.insert_resource(ResourceWrapper(noise))
        // .add_startup_system(create_world); // go to player_loading.rs and uncomment the section specified if this is active.
    ;
}

#[allow(dead_code)]
fn create_world(mut commands: Commands) {
    let mut entity_cmd = commands.spawn_empty();

    let mut structure = Structure::new(16, 4, 16);

    let biosphere = GrassBiosphere::default();
    let marker = biosphere.get_marker_component();
    let builder = ServerPlanetBuilder::default();

    builder.insert_planet(&mut entity_cmd, Location::default(), &mut structure);

    entity_cmd
        .insert(structure)
        .insert(NeedsGenerated)
        .insert(marker);
}
