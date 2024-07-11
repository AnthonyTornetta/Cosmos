//! Creates a molten planet

use bevy::{
    log::warn,
    prelude::{App, Component, Entity, Event, IntoSystemConfigs, OnEnter, Res, ResMut},
    reflect::TypePath,
};
use cosmos_core::{
    registry::Registry,
    structure::{
        coordinates::ChunkCoordinate,
        planet::generation::biome::{Biome, BiomeParameters, BiosphereBiomesRegistry},
    },
};

use crate::GameState;

use super::{register_biosphere, BiosphereMarkerComponent, RegisterBiomesSet, TBiosphere, TGenerateChunkEvent, TemperatureRange};

#[derive(Component, Debug, Default, Clone, Copy, TypePath)]
/// Marks that this is for a grass biosphere
pub struct MoltenBiosphereMarker;

impl BiosphereMarkerComponent for MoltenBiosphereMarker {
    fn unlocalized_name() -> &'static str {
        "cosmos:molten"
    }
}

/// Marks that a grass chunk needs generated
#[derive(Debug, Event)]
pub struct MoltenChunkNeedsGeneratedEvent {
    coords: ChunkCoordinate,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for MoltenChunkNeedsGeneratedEvent {
    fn new(coords: ChunkCoordinate, structure_entity: Entity) -> Self {
        Self { coords, structure_entity }
    }

    fn get_structure_entity(&self) -> Entity {
        self.structure_entity
    }

    fn get_chunk_coordinates(&self) -> ChunkCoordinate {
        self.coords
    }
}

#[derive(Default, Debug)]
/// Creates a molten planet
pub struct MoltenBiosphere;

impl TBiosphere<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedEvent> for MoltenBiosphere {
    fn get_marker_component(&self) -> MoltenBiosphereMarker {
        MoltenBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, coords: ChunkCoordinate, structure_entity: Entity) -> MoltenChunkNeedsGeneratedEvent {
        MoltenChunkNeedsGeneratedEvent::new(coords, structure_entity)
    }
}

// fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
//     commands.insert_resource(
//         BlockLayers::default()
//             .with_sea_level_block("cosmos:lava", &block_registry, 620)
//             .expect("Cheese missing!")
//             .add_noise_layer("cosmos:molten_stone", &block_registry, 160, 0.10, 7.0, 9)
//             .expect("Molten Stone missing"),
//     );
// }

// // Fills the chunk at the given coordinates with spikes
// fn generate_spikes(
//     coords: ChunkCoordinate,
//     structure: &mut Structure,
//     location: &Location,
//     block_event_writer: &mut EventWriter<BlockChangedEvent>,
//     blocks: &Registry<Block>,
//     seed: ServerSeed,
// ) {
//     let sc = coords.first_structure_block();

//     let Structure::Dynamic(planet) = structure else {
//         panic!("A planet must be dynamic!");
//     };

//     let s_dimension = planet.block_dimensions();
//     let s_dimensions = structure.block_dimensions();
//     let molten_stone = blocks.from_id("cosmos:molten_stone").expect("Missing molten_stone");

//     let structure_coords = location.absolute_coords_f64();

//     let faces = Planet::chunk_planet_faces(sc, s_dimension);
//     for block_up in faces.iter() {
//         // Getting the noise value for every block in the chunk, to find where to put spikes.
//         let noise_height = match block_up {
//             BlockFace::Front | BlockFace::Top | BlockFace::Right => s_dimension,
//             _ => 0,
//         };

//         for z in 0..CHUNK_DIMENSIONS {
//             for x in 0..CHUNK_DIMENSIONS {
//                 let (nx, ny, nz) = match block_up {
//                     BlockFace::Front | BlockFace::Back => ((sc.x + x) as f64, (sc.y + z) as f64, noise_height as f64),
//                     BlockFace::Top | BlockFace::Bottom => ((sc.x + x) as f64, noise_height as f64, (sc.z + z) as f64),
//                     BlockFace::Right | BlockFace::Left => (noise_height as f64, (sc.y + x) as f64, (sc.z + z) as f64),
//                 };

//                 let rng = seed
//                     .chaos_hash(nx + structure_coords.x, ny + structure_coords.y, nz + structure_coords.z)
//                     .abs()
//                     % 20;

//                 if rng == 0 {
//                     let rng = seed
//                         .chaos_hash(
//                             2000.0 + nx + structure_coords.x,
//                             2000.0 + ny + structure_coords.y,
//                             2000.0 + nz + structure_coords.z,
//                         )
//                         .abs()
//                         % 4;

//                     let coords: BlockCoordinate = match block_up {
//                         BlockFace::Front | BlockFace::Back => (sc.x + x, sc.y + z, sc.z),
//                         BlockFace::Top | BlockFace::Bottom => (sc.x + x, sc.y, sc.z + z),
//                         BlockFace::Right | BlockFace::Left => (sc.x, sc.y + x, sc.z + z),
//                     }
//                     .into();

//                     if let Ok(start_checking) = rotate(
//                         coords,
//                         UnboundBlockCoordinate::new(0, CHUNK_DIMENSIONS as UnboundCoordinateType - 1, 0),
//                         s_dimensions,
//                         block_up,
//                     ) {
//                         'spike_placement: for dy_down in 0..CHUNK_DIMENSIONS as UnboundCoordinateType {
//                             if let Ok(rotated) = rotate(start_checking, UnboundBlockCoordinate::new(0, -dy_down, 0), s_dimensions, block_up)
//                             {
//                                 if structure.block_at(rotated, blocks) == molten_stone {
//                                     for dy in 1..=rng {
//                                         if let Ok(rel_pos) = rotate(
//                                             start_checking,
//                                             UnboundBlockCoordinate::new(0, dy as UnboundCoordinateType - dy_down, 0),
//                                             s_dimensions,
//                                             block_up,
//                                         ) {
//                                             structure.set_block_at(rel_pos, molten_stone, block_up, blocks, Some(block_event_writer));
//                                         }
//                                     }
//                                     break 'spike_placement;
//                                 }
//                             }
//                         }
//                     }
//                 }
//             }
//         }
//     }
// }

fn register_biosphere_biomes(
    biome_registry: Res<Registry<Biome>>,
    mut biosphere_biomes_registry: ResMut<Registry<BiosphereBiomesRegistry>>,
) {
    let biosphere_registry = biosphere_biomes_registry
        .from_id_mut(MoltenBiosphereMarker::unlocalized_name())
        .expect("Missing molten biosphere registry!");

    if let Some(plains) = biome_registry.from_id("cosmos:molten") {
        biosphere_registry.register(
            plains,
            BiomeParameters {
                ideal_elevation: 30.0,
                ideal_humidity: 30.0,
                ideal_temperature: 60.0,
            },
        );
    } else {
        warn!("Missing molten biome!");
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<MoltenBiosphereMarker, MoltenChunkNeedsGeneratedEvent>(
        app,
        TemperatureRange::new(450.0, f32::MAX),
        0.75,
        Some("cosmos:lava"),
    );

    app.add_systems(
        OnEnter(GameState::PostLoading),
        register_biosphere_biomes
            .in_set(RegisterBiomesSet::RegisterBiomes)
            .ambiguous_with(RegisterBiomesSet::RegisterBiomes),
    );
}
