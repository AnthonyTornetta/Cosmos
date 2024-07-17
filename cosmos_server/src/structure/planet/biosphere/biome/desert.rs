//! Desert biome

use bevy::{
    app::Update,
    ecs::{event::EventReader, schedule::IntoSystemConfigs, system::Query},
    prelude::{App, EventWriter, OnExit, Res, ResMut},
};
use cosmos_core::{
    block::{block_face::BlockFace, Block},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::{identifiable::Identifiable, Registry},
    structure::{
        chunk::CHUNK_DIMENSIONS,
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        planet::{generation::block_layers::BlockLayers, Planet},
        rotate, Structure,
    },
};

use crate::{
    init::init_world::{Noise, ServerSeed},
    state::GameState,
    structure::planet::biosphere::biosphere_generation::BiosphereGenerationSet,
};

use super::{Biome, GenerateChunkFeaturesEvent};

const MAX_CACTUS_HEIGHT: CoordinateType = 3;
const MAX_CACTUS_ITERATIONS_PER_FACE: i64 = 200;

fn desert_generate_chunk_features(
    mut ev_reader: EventReader<GenerateChunkFeaturesEvent>,
    mut ev_writer: EventWriter<BlockChangedEvent>,
    mut q_structure: Query<(&Location, &mut Structure)>,
    biomes: Res<Registry<Biome>>,
    blocks: Res<Registry<Block>>,
    noise_generator: Res<Noise>,
    seed: Res<ServerSeed>,
) {
    for ev in ev_reader.read() {
        let Some(desert) = biomes.from_id("cosmos:desert") else {
            return;
        };

        if ev.included_biomes.contains(&desert.id()) {
            let Ok((location, mut structure)) = q_structure.get_mut(ev.structure_entity) else {
                continue;
            };

            generate_chunk_features(&mut ev_writer, ev.chunk, &mut structure, location, &blocks, &noise_generator, &seed);
        }
    }
}

fn generate_chunk_features(
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
                BlockFace::Back | BlockFace::Front => (first_block_coords.x + x, first_block_coords.y + z, first_block_coords.z),
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
                if height < 0 || block != sand || structure.block_rotation(rotated).face_pointing_pos_y != block_up {
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
                        structure.set_block_at(cactus_coord, cactus, block_up.into(), blocks, Some(block_event_writer));
                    }
                }
            }
        }
    }
}

fn register_biome(mut registry: ResMut<Registry<Biome>>, block_registry: Res<Registry<Block>>) {
    registry.register(Biome::new(
        "cosmos:desert",
        BlockLayers::default()
            .add_fixed_layer("cosmos:sand", &block_registry, 0)
            .expect("Sand missing")
            .add_fixed_layer("cosmos:stone", &block_registry, 4)
            .expect("Stone missing"),
    ));
}

pub(super) fn register(app: &mut App) {
    app.add_systems(OnExit(GameState::Loading), register_biome).add_systems(
        Update,
        desert_generate_chunk_features.in_set(BiosphereGenerationSet::GenerateChunkFeatures),
    );
}
