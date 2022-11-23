use std::time::SystemTime;

use bevy::prelude::*;
use cosmos_core::{
    structure::{
        events::StructureCreated, planet::planet_builder::TPlanetBuilder, structure::Structure,
    },
    utils::resource_wrapper::ResourceWrapper,
};
use noise::Seedable;

use crate::structure::planet::{
    biosphere::grass_biosphere::GrassBiosphere, server_planet_builder::ServerPlanetBuilder,
};

pub fn register(app: &mut App) {
    let noise = noise::OpenSimplex::default();

    noise.set_seed(
        (SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_millis()
            % u32::MAX as u128) as u32,
    );

    app.insert_resource(ResourceWrapper(noise))
        .add_startup_system(create_world);
}

fn create_world(mut commands: Commands, mut event_writer: EventWriter<StructureCreated>) {
    let mut entity_cmd = commands.spawn_empty();

    let mut structure = Structure::new(20, 1, 20, false, entity_cmd.id());

    let builder = ServerPlanetBuilder::new(GrassBiosphere::default());

    builder.insert_planet(
        &mut entity_cmd,
        Transform::from_xyz(0.0, 0.0, 0.0),
        &mut structure,
    );

    entity_cmd.insert(structure);

    event_writer.send(StructureCreated {
        entity: entity_cmd.id(),
    });
}
