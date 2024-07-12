//! Contains logic related to the localized formation of terrain

use bevy::{
    ecs::{entity::Entity, event::Event},
    prelude::{App, IntoSystemConfigs, OnExit, ResMut, SystemSet},
    state::state::OnEnter,
    utils::HashSet,
};
use cosmos_core::{
    registry::Registry,
    structure::{
        coordinates::ChunkCoordinate,
        planet::generation::biome::{Biome, BiosphereBiomesRegistry},
    },
};

use crate::state::GameState;

use super::BiosphereMarkerComponent;

pub mod desert;
pub mod ice;
pub mod molten;
pub mod ocean;
pub mod plains;

fn construct_lookup_tables(mut registry: ResMut<Registry<BiosphereBiomesRegistry>>) {
    for registry in registry.iter_mut() {
        registry.construct_lookup_table();
    }
}

#[derive(SystemSet, Debug, Hash, PartialEq, Eq, Clone)]
/// System set to avoid ambiguity issues
pub enum CreateBiosphereSet {
    /// Responsible for adding biospheres to the [`Registry<BiosphereBiomesRegistry>`].
    CreateBiospheres,
}

fn create_biosphere_registry<T: BiosphereMarkerComponent>(mut registry: ResMut<Registry<BiosphereBiomesRegistry>>) {
    registry.register(BiosphereBiomesRegistry::new(T::unlocalized_name()));
}

/// This will setup the biosphere registry and construct the lookup tables at the end of [`GameState::PostLoading`]
///
/// You don't normally have to call this manually, because is automatically called in `register_biosphere`
pub fn create_biosphere_biomes_registry<T: BiosphereMarkerComponent>(app: &mut App) {
    app.add_systems(
        OnEnter(GameState::PreLoading),
        create_biosphere_registry::<T>
            .in_set(CreateBiosphereSet::CreateBiospheres)
            .ambiguous_with(CreateBiosphereSet::CreateBiospheres),
    );
}

#[derive(Event)]
/// This event is sent whenever a chunk needs its features generated
pub struct GenerateChunkFeaturesEvent {
    /// The biomes that should generate features for this chunk
    pub included_biomes: HashSet<u16>,
    // pub biome_ids: Box<[u16; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE]>,
    /// The chunk that needs its features generated
    pub chunk: ChunkCoordinate,
    /// The structure the chunk is on
    pub structure_entity: Entity,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum RegisterBiomesSet {
    RegisterBiomes,
}

pub(super) fn register(app: &mut App) {
    app.add_event::<GenerateChunkFeaturesEvent>()
        .add_systems(OnExit(GameState::PostLoading), construct_lookup_tables);

    app.configure_sets(OnEnter(GameState::PreLoading), CreateBiosphereSet::CreateBiospheres);

    app.configure_sets(OnExit(GameState::Loading), RegisterBiomesSet::RegisterBiomes);

    ice::register(app);
    molten::register(app);
    desert::register(app);
    plains::register(app);
    ocean::register(app);
}
