//! Responsible for building planets for the client.

use bevy::{
    ecs::system::EntityCommands,
    pbr::NotShadowCaster,
    prelude::{
        shape::UVSphere, Added, App, Assets, Color, Commands, ComputedVisibility, Entity, Mesh,
        Query, Res, ResMut, StandardMaterial, Visibility,
    },
};
use cosmos_core::{
    registry::Registry,
    structure::{
        planet::{
            biosphere::BiosphereMarker, planet_builder::PlanetBuilder,
            planet_builder::TPlanetBuilder, Planet,
        },
        Structure,
    },
};

use crate::structure::client_structure_builder::ClientStructureBuilder;

use super::biosphere::BiosphereColor;

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
        structure: &mut Structure,
        planet: Planet,
    ) {
        self.planet_builder.insert_planet(entity, structure, planet);
    }
}

fn added_planet(
    query: Query<(Entity, &Structure, &BiosphereMarker), Added<Planet>>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    color_registry: Res<Registry<BiosphereColor>>,
) {
    for (ent, structure, marker) in query.iter() {
        let blocks_radius = structure.blocks_width() as f32 / 2.0;

        commands.entity(ent).insert((
            meshes.add(
                UVSphere {
                    radius: (blocks_radius * blocks_radius * 2.0).sqrt() * 1.1,
                    sectors: 128,
                    stacks: 128,
                }
                .into(),
            ),
            materials.add(StandardMaterial {
                base_color: color_registry
                    .from_id(marker.biosphere_name())
                    .map(|x| x.color())
                    .unwrap_or(Color::WHITE),
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
