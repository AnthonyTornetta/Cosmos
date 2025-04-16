//! Used to create asteroids on the client

use bevy::{
    ecs::system::EntityCommands,
    prelude::{Added, App, Commands, Entity, Query, Update},
};
use cosmos_core::{
    physics::location::Location,
    structure::{
        Structure,
        asteroid::{
            Asteroid,
            asteroid_builder::{AsteroidBuilder, TAsteroidBuilder},
        },
    },
};

use crate::structure::{chunk_retreiver::NeedsPopulated, client_structure_builder::ClientStructureBuilder};

/// Builds a client asteroid
pub struct ClientAsteroidBuilder {
    builder: AsteroidBuilder<ClientStructureBuilder>,
}

impl ClientAsteroidBuilder {
    /// ClientAsteroidBuilder::default()
    pub fn new() -> Self {
        Self::default()
    }
}

impl Default for ClientAsteroidBuilder {
    fn default() -> Self {
        Self {
            builder: AsteroidBuilder::new(ClientStructureBuilder::default()),
        }
    }
}

impl TAsteroidBuilder for ClientAsteroidBuilder {
    fn insert_asteroid(&self, entity: &mut EntityCommands, location: Location, structure: &mut Structure, temperature: f32) {
        self.builder.insert_asteroid(entity, location, structure, temperature);
    }
}

fn on_add_asteroid(query: Query<Entity, Added<Asteroid>>, mut commands: Commands) {
    for entity in query.iter() {
        commands.entity(entity).insert(NeedsPopulated);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(Update, on_add_asteroid);
}
