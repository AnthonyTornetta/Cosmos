//! Molten biome

use bevy::prelude::{App, EventWriter, OnExit, Res, ResMut};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::CHUNK_DIMENSIONS,
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        planet::Planet,
        rotate, Structure,
    },
};

use crate::{
    init::init_world::{Noise, ServerSeed},
    state::GameState,
    structure::planet::biosphere::biosphere_generation::BlockLayers,
};

use super::{biome_registry::RegisteredBiome, Biome};

/// Sandy without any features
pub struct MoltenBiome {
    id: u16,
    unlocalized_name: String,
    block_layers: BlockLayers,
}

impl MoltenBiome {
    /// Creates a new Molten biome
    pub fn new(name: impl Into<String>, block_layers: BlockLayers) -> Self {
        Self {
            id: 0,
            block_layers,
            unlocalized_name: name.into(),
        }
    }
}

const MAX_CACTUS_HEIGHT: CoordinateType = 3;
const MAX_CACTUS_ITERATIONS_PER_FACE: i64 = 200;

impl Biome for MoltenBiome {
    fn block_layers(&self) -> &BlockLayers {
        &self.block_layers
    }

    fn id(&self) -> u16 {
        self.id
    }

    fn set_numeric_id(&mut self, id: u16) {
        self.id = id;
    }

    fn unlocalized_name(&self) -> &str {
        &self.unlocalized_name
    }

    fn generate_chunk_features(
        &self,
        block_event_writer: &mut EventWriter<BlockChangedEvent>,
        coords: ChunkCoordinate,
        structure: &mut Structure,
        location: &Location,
        blocks: &Registry<Block>,
        _noise_generator: &Noise,
        seed: &ServerSeed,
    ) {
        let Structure::Dynamic(planet) = structure else {
            panic!("A planet must be dynamic!");
        };

        let first_block_coords = coords.first_structure_block();
        let s_dimension = planet.block_dimensions();
        let s_dims = structure.block_dimensions();

        let air = blocks.from_id("cosmos:air").unwrap();
        let cactus = blocks.from_id("cosmos:cactus").unwrap();
        let sand = blocks.from_id("cosmos:sand").unwrap();

        let faces = Planet::chunk_planet_faces(first_block_coords, s_dimension);
        for block_up in faces.iter() {
            let abs_coords = location.absolute_coords_f64();

            let (sx, sy, sz) = (
                abs_coords.x + first_block_coords.x as f64,
                abs_coords.y + first_block_coords.y as f64,
                abs_coords.z + first_block_coords.z as f64,
            );

            let rng = seed.chaos_hash(sx, sy, sz) % MAX_CACTUS_ITERATIONS_PER_FACE;
            for rng_changer in 0..rng {
                let x = seed
                    .chaos_hash(
                        sx + 456.0 * rng_changer as f64,
                        sy + 4645.0 * rng_changer as f64,
                        sz + 354.0 * rng_changer as f64,
                    )
                    .unsigned_abs()
                    % CHUNK_DIMENSIONS;

                let z = seed
                    .chaos_hash(
                        sx + 678.0 * rng_changer as f64,
                        sy + 87.0 * rng_changer as f64,
                        sz + 456.0 * rng_changer as f64,
                    )
                    .unsigned_abs()
                    % CHUNK_DIMENSIONS;

                let coords: BlockCoordinate = match block_up {
                    BlockFace::Front | BlockFace::Back => (first_block_coords.x + x, first_block_coords.y + z, first_block_coords.z),
                    BlockFace::Top | BlockFace::Bottom => (first_block_coords.x + x, first_block_coords.y, first_block_coords.z + z),
                    BlockFace::Right | BlockFace::Left => (first_block_coords.x, first_block_coords.y + x, first_block_coords.z + z),
                }
                .into();

                let mut height = CHUNK_DIMENSIONS as UnboundCoordinateType - 1;
                while height >= 0
                    && rotate(coords, UnboundBlockCoordinate::new(0, height, 0), s_dims, block_up)
                        .map(|rotated| structure.block_at(rotated, blocks) == air)
                        .unwrap_or(false)
                {
                    height -= 1;
                }

                // No sand block to grow cactus from.
                if let Ok(rotated) = rotate(coords, UnboundBlockCoordinate::new(0, height, 0), s_dims, block_up) {
                    let block = structure.block_at(rotated, blocks);
                    if height < 0 || block != sand || structure.block_rotation(rotated) != block_up {
                        continue;
                    }

                    let height = seed
                        .chaos_hash(
                            sx + 561.0 * rng_changer as f64,
                            sy + 456.0 * rng_changer as f64,
                            sz + 786.0 * rng_changer as f64,
                        )
                        .unsigned_abs()
                        % MAX_CACTUS_HEIGHT
                        + 1;

                    for dy in 1..=height {
                        if let Ok(cactus_coord) = rotate(
                            coords,
                            UnboundBlockCoordinate::new(0, dy as UnboundCoordinateType, 0),
                            s_dims,
                            block_up,
                        ) {
                            structure.set_block_at(cactus_coord, cactus, block_up, blocks, Some(block_event_writer));
                        }
                    }
                }
            }
        }
    }
}

fn register_biome(mut registry: ResMut<Registry<RegisteredBiome>>, block_registry: Res<Registry<Block>>) {
    registry.register(RegisteredBiome::new(Box::new(MoltenBiome::new(
        "cosmos:molten",
        BlockLayers::default()
            .add_noise_layer("cosmos:ice", &block_registry, 0, 0.05, 7.0, 9)
            .expect("Sand missing"),
    ))));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnExit(GameState::Loading), register_biome);
}
