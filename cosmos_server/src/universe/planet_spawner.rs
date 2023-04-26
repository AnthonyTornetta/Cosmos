//! Responsible for spawning planets near stars, but for now just spawns a planet at 0, 0, 0.

use bevy::prelude::{in_state, App, Commands, IntoSystemConfig, Query, With};
use cosmos_core::{
    entities::player::Player,
    physics::location::Location,
    structure::{
        planet::{planet_builder::TPlanetBuilder, Planet},
        Structure,
    },
};

use crate::{
    persistence::is_sector_loaded,
    state::GameState,
    structure::planet::{
        biosphere::{test_all_stone_biosphere::TestStoneBiosphere, TBiosphere},
        server_planet_builder::ServerPlanetBuilder,
    },
};

fn spawn_planet(
    query: Query<&Location, With<Planet>>,
    players: Query<&Location, With<Player>>,
    mut commands: Commands,
) {
    if !players
        .iter()
        .any(|l| l.distance_sqrd(&Location::default()) < 100000.0)
    {
        return;
    }

    if is_sector_loaded((0, 0, 0)) {
        return;
    }

    for loc in query.iter() {
        if loc.sector_x == 0 && loc.sector_y == 0 && loc.sector_z == 0 {
            return;
        }
    }

    let mut entity_cmd = commands.spawn_empty();

    let mut structure = Structure::new(16, 16, 16);

    let biosphere = TestStoneBiosphere::default();
    let marker = biosphere.get_marker_component();
    let builder = ServerPlanetBuilder::default();

    builder.insert_planet(&mut entity_cmd, Location::default(), &mut structure);

    entity_cmd.insert(structure).insert(marker);
}

pub(super) fn register(app: &mut App) {
    app.add_system(spawn_planet.run_if(in_state(GameState::Playing)));
}
