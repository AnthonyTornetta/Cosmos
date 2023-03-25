use std::time::SystemTime;

use bevy::prelude::*;
use cosmos_core::{
    physics::location::Location,
    structure::{planet::planet_builder::TPlanetBuilder, Structure},
    utils::resource_wrapper::ResourceWrapper,
};
use noise::Seedable;

use crate::{
    persistence::{loading::NeedsLoaded, EntityId, SaveFileIdentifier},
    structure::planet::{
        biosphere::{grass_biosphere::GrassBiosphere, TBiosphere},
        generation::planet_generator::NeedsGenerated,
        server_planet_builder::ServerPlanetBuilder,
    },
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
        // .add_startup_system(load_world);
        // .add_startup_system(create_world);
        ;
}

#[allow(dead_code)]
fn load_world(mut commands: Commands) {
    commands.spawn((
        SaveFileIdentifier {
            sector: Some((0, 0, 0)),
            entity_id: EntityId::new(
                "2WErFbnBgQTDownNFElKPeTC6FJeLyTC0SU6DJXEPlgC0R7YysVYrS6DBgAlI1gR",
            ),
        },
        NeedsLoaded,
    ));
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
