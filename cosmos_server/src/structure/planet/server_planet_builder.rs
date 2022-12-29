use std::marker::PhantomData;

use bevy::prelude::Component;
use cosmos_core::structure::planet::{
    planet_builder::PlanetBuilder, planet_builder::TPlanetBuilder,
};

use crate::structure::server_structure_builder::ServerStructureBuilder;

use super::{
    biosphere::{TBiosphere, TGenerateChunkEvent},
    generation::planet_generator::NeedsGenerated,
};

pub struct ServerPlanetBuilder<K: Component, V: TGenerateChunkEvent, T: TBiosphere<K, V>> {
    builder: PlanetBuilder<ServerStructureBuilder>,
    biosphere: T,

    _phantom_1: PhantomData<K>,
    _phantom_2: PhantomData<V>,
}

impl<K: Component, V: TGenerateChunkEvent, T: TBiosphere<K, V>> ServerPlanetBuilder<K, V, T> {
    pub fn new(biosphere: T) -> Self {
        Self {
            builder: PlanetBuilder::new(ServerStructureBuilder::default()),
            biosphere,
            _phantom_1: PhantomData::default(),
            _phantom_2: PhantomData::default(),
        }
    }
}

impl<K: Component, V: TGenerateChunkEvent, T: TBiosphere<K, V>> TPlanetBuilder
    for ServerPlanetBuilder<K, V, T>
{
    fn insert_planet(
        &self,
        entity: &mut bevy::ecs::system::EntityCommands,
        transform: bevy::prelude::Transform,
        structure: &mut cosmos_core::structure::Structure,
    ) {
        self.builder.insert_planet(entity, transform, structure);

        entity.insert(NeedsGenerated);
        entity.insert(self.biosphere.get_marker_component());
    }
}
