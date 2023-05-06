//! Responsible for building planets for the client.

use bevy::{
    ecs::system::EntityCommands,
    pbr::NotShadowCaster,
    prelude::{
        shape::UVSphere, Added, App, Assets, Color, Commands, ComputedVisibility, Entity, Mesh,
        Query, ResMut, StandardMaterial, Visibility,
    },
};
use cosmos_core::{
    physics::location::Location,
    structure::{
        planet::{planet_builder::PlanetBuilder, planet_builder::TPlanetBuilder, Planet},
        Structure,
    },
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

/// Responsible for building planets for the client.
pub struct ClientPlanetBuilder {
    planet_builder: PlanetBuilder<ClientStructureBuilder>,
}

impl Default for ClientPlanetBuilder {
    fn default() -> Self {
        Self {
            planet_builder: PlanetBuilder::new(ClientStructureBuilder::default()),
        }
    }
}

impl TPlanetBuilder for ClientPlanetBuilder {
    fn insert_planet(
        &self,
        entity: &mut EntityCommands,
        location: Location,
        structure: &mut Structure,
        planet: Planet,
    ) {
        self.planet_builder
            .insert_planet(entity, location, structure, planet);
    }
}

fn added_planet(
    query: Query<(Entity, &Structure, &Planet), Added<Planet>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (ent, structure, _) in query.iter() {
        commands.entity(ent).insert((
            meshes.add(
                UVSphere {
                    radius: structure.blocks_width() as f32 / 1.8,
                    sectors: 128,
                    stacks: 128,
                }
                .into(),
            ),
            materials.add(StandardMaterial {
                base_color: Color::GREEN,
                ..Default::default()
            }),
            Visibility::default(),
            ComputedVisibility::default(),
            NotShadowCaster,
        ));
    }
}

pub(super) fn register(app: &mut App) {
    app.add_system(added_planet);
}
