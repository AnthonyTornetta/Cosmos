use bevy::prelude::*;
use bevy_app_compute::prelude::*;
use bevy_rapier3d::na::Vector3;
use bytemuck::{Pod, Zeroable};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{
        block_storage::BlockStorer,
        chunk::{Chunk, CHUNK_DIMENSIONS_USIZE},
        coordinates::{ChunkBlockCoordinate, ChunkCoordinate, CoordinateType},
        Structure,
    },
    utils::array_utils::expand,
};
use noise::NoiseFn;
use rand::{
    distributions::{Distribution, Standard},
    Rng,
};
use std::marker::PhantomData;

use crate::init::init_world::{Noise, ReadOnlyNoise};

use super::{
    biome::BiosphereBiomesRegistry, biosphere_generation_old::GenerateChunkFeaturesEvent, BiomeDecider, BiosphereMarkerComponent,
    BiosphereSeaLevel, TGenerateChunkEvent,
};

#[derive(Debug)]
pub struct NeedGeneratedChunk<T> {
    chunk: Chunk,
    structure_entity: Entity,
    chunk_pos: Vec3,
    structure_dimensions: CoordinateType,
    structure_location: Location,
    _phantom: PhantomData<T>,
}

#[derive(Resource, Debug, Default)]
pub struct NeedGeneratedChunks<T>(Vec<NeedGeneratedChunk<T>>);

#[derive(Resource, Debug, Default)]
pub struct GeneratingChunks<T>(Vec<NeedGeneratedChunk<T>>);

const TABLE_SIZE: usize = 256;

/// A seed table, required by all noise functions.
///
/// Table creation is expensive, so in most circumstances you'll only want to
/// create one of these per generator.
#[derive(Copy, Clone)]
pub struct PermutationTable {
    values: [u8; TABLE_SIZE],
}

fn hash(perm_table: &PermutationTable, to_hash: &[isize]) -> usize {
    let index = to_hash
        .iter()
        .map(|&a| (a & 0xff) as usize)
        .reduce(|a, b| perm_table.values[a] as usize ^ b)
        .unwrap();
    perm_table.values[index] as usize
}

#[inline(always)]
#[rustfmt::skip]
pub(crate) fn grad3(index: usize) -> Vector3<f64> {
    // Vectors are combinations of -1, 0, and 1
    // Precompute the normalized elements
    const DIAG : f64 = core::f64::consts::FRAC_1_SQRT_2;
    const DIAG2 : f64 = 0.577_350_269_189_625_8;

    match index % 32 {
        // 12 edges repeated twice then 8 corners
        0  | 12 => Vector3::new(  DIAG,   DIAG,    0.0),
        1  | 13 => Vector3::new( -DIAG,   DIAG,    0.0),
        2  | 14 => Vector3::new(  DIAG,  -DIAG,    0.0),
        3  | 15 => Vector3::new( -DIAG,  -DIAG,    0.0),
        4  | 16 => Vector3::new(  DIAG,    0.0,   DIAG),
        5  | 17 => Vector3::new( -DIAG,    0.0,   DIAG),
        6  | 18 => Vector3::new(  DIAG,    0.0,  -DIAG),
        7  | 19 => Vector3::new( -DIAG,    0.0,  -DIAG),
        8  | 20 => Vector3::new(   0.0,   DIAG,   DIAG),
        9  | 21 => Vector3::new(   0.0,  -DIAG,   DIAG),
        10 | 22 => Vector3::new(   0.0,   DIAG,  -DIAG),
        11 | 23 => Vector3::new(   0.0,  -DIAG,  -DIAG),
        24      => Vector3::new( DIAG2,  DIAG2,  DIAG2),
        25      => Vector3::new(-DIAG2,  DIAG2,  DIAG2),
        26      => Vector3::new( DIAG2, -DIAG2,  DIAG2),
        27      => Vector3::new(-DIAG2, -DIAG2,  DIAG2),
        28      => Vector3::new( DIAG2,  DIAG2, -DIAG2),
        29      => Vector3::new(-DIAG2,  DIAG2, -DIAG2),
        30      => Vector3::new( DIAG2, -DIAG2, -DIAG2),
        31      => Vector3::new(-DIAG2, -DIAG2, -DIAG2),
        _       => panic!("Attempt to access gradient {} of 32", index % 32),
    }
}

fn floor_vec3(v: Vector3<f64>) -> Vector3<f64> {
    Vector3::new(v.x.floor(), v.y.floor(), v.z.floor())
}

fn create_perm_table() -> PermutationTable {
    PermutationTable { values: [21; TABLE_SIZE] }
}

pub fn open_simplex_3d<NH>(point: [f64; 3]) -> f64 {
    let perm_table = create_perm_table();

    const STRETCH_CONSTANT: f64 = -1.0 / 6.0; //(1/Math.sqrt(3+1)-1)/3;
    const SQUISH_CONSTANT: f64 = 1.0 / 3.0; //(Math.sqrt(3+1)-1)/3;
    const NORM_CONSTANT: f64 = 1.0 / 14.0;

    fn surflet(index: usize, point: Vector3<f64>) -> f64 {
        let t = 2.0 - point.magnitude_squared();

        if t > 0.0 {
            let gradient = Vector3::from(grad3(index));
            t.powi(4) * point.dot(&gradient)
        } else {
            0.0
        }
    }

    let point = Vector3::from(point);

    // Place input coordinates on simplectic honeycomb.
    let stretch_offset = point.sum() * STRETCH_CONSTANT;
    let stretched = point.map(|v| v + stretch_offset);

    // Floor to get simplectic honeycomb coordinates of rhombohedron
    // (stretched cube) super-cell origin.
    let stretched_floor = floor_vec3(stretched);

    // Skew out to get actual coordinates of rhombohedron origin. We'll need
    // these later.
    let squish_offset = stretched_floor.sum() * SQUISH_CONSTANT;
    let origin = stretched_floor.map(|v| v + squish_offset);

    // Compute simplectic honeycomb coordinates relative to rhombohedral origin.
    let rel_coords = stretched - stretched_floor;

    // Sum those together to get a value that determines which region we're in.
    let region_sum = rel_coords.sum();

    // Positions relative to origin point.
    let rel_pos = point - origin;

    macro_rules! contribute (
            ($x:expr, $y:expr, $z:expr) => {
                {
                    let offset = Vector3::new($x, $y, $z);
                    let vertex = stretched_floor + offset;
                    let index = hash(&perm_table, &[vertex.x as isize, vertex.y as isize, vertex.z as isize]);
                    let dpos = rel_pos - (Vector3::from_element(SQUISH_CONSTANT) * offset.sum()) - offset;

                    surflet(index, dpos)
                }
            }
        );

    let mut value = 0.0;

    if region_sum <= 1.0 {
        // We're inside the tetrahedron (3-Simplex) at (0, 0, 0)

        // Contribution at (0, 0, 0)
        value += contribute!(0.0, 0.0, 0.0);

        // Contribution at (1, 0, 0)
        value += contribute!(1.0, 0.0, 0.0);

        // Contribution at (0, 1, 0)
        value += contribute!(0.0, 1.0, 0.0);

        // Contribution at (0, 0, 1)
        value += contribute!(0.0, 0.0, 1.0);
    } else if region_sum >= 2.0 {
        // We're inside the tetrahedron (3-Simplex) at (1, 1, 1)

        // Contribution at (1, 1, 0)
        value += contribute!(1.0, 1.0, 0.0);

        // Contribution at (1, 0, 1)
        value += contribute!(1.0, 0.0, 1.0);

        // Contribution at (0, 1, 1)
        value += contribute!(0.0, 1.0, 1.0);

        // Contribution at (1, 1, 1)
        value += contribute!(1.0, 1.0, 1.0);
    } else {
        // We're inside the octahedron (Rectified 3-Simplex) inbetween.

        // Contribution at (1, 0, 0)
        value += contribute!(1.0, 0.0, 0.0);

        // Contribution at (0, 1, 0)
        value += contribute!(0.0, 1.0, 0.0);

        // Contribution at (0, 0, 1)
        value += contribute!(0.0, 0.0, 1.0);

        // Contribution at (1, 1, 0)
        value += contribute!(1.0, 1.0, 0.0);

        // Contribution at (1, 0, 1)
        value += contribute!(1.0, 0.0, 1.0);

        // Contribution at (0, 1, 1)
        value += contribute!(0.0, 1.0, 1.0);
    }

    value * NORM_CONSTANT
}

pub fn send_and_read_chunks_gpu<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut needs_generated_chunks: ResMut<NeedGeneratedChunks<T>>,
    mut currently_generating_chunks: ResMut<GeneratingChunks<T>>,
    noise_generator: Res<Noise>,
    // biosphere_biomes: Res<BiosphereBiomesRegistry<T>>,
    // biome_decider: Res<BiomeDecider<T>>,
    sea_level: Option<Res<BiosphereSeaLevel<T>>>,
    mut worker: ResMut<AppComputeWorker<ShaderWorker<T>>>,
    mut ev_writer: EventWriter<GenerateChunkFeaturesEvent<T>>,
    blocks: Res<Registry<Block>>,
    mut q_structure: Query<&mut Structure>,
) {
    if worker.ready() {
        if let Some(mut needs_generated_chunk) = currently_generating_chunks.0.pop() {
            if let Ok(mut structure) = q_structure.get_mut(needs_generated_chunk.structure_entity) {
                let v: Vec<f32> = worker.try_read_vec("values").expect("Failed to read values!");

                let b = blocks.from_id("cosmos:stone").expect("Missing stone?");

                for (i, value) in v.into_iter().enumerate() {
                    let (x, y, z) = expand(i, CHUNK_DIMENSIONS_USIZE, CHUNK_DIMENSIONS_USIZE);

                    if value != 0.0 {
                        needs_generated_chunk.chunk.set_block_at(
                            ChunkBlockCoordinate::new(x as CoordinateType, y as CoordinateType, z as CoordinateType),
                            &b,
                            BlockFace::Top,
                        )
                    }
                }

                ev_writer.send(GenerateChunkFeaturesEvent {
                    chunk_coords: needs_generated_chunk.chunk.chunk_coordinates(),
                    structure_entity: needs_generated_chunk.structure_entity,
                    _phantom: Default::default(),
                });

                structure.set_chunk(needs_generated_chunk.chunk);
            }
        } else {
            warn!("Something wacky happened");
        }
    }

    if currently_generating_chunks.0.is_empty() {
        if let Some(todo) = needs_generated_chunks.0.pop() {
            let structure_loc = todo.structure_location.absolute_coords_f32();

            let params = GenerationParams {
                chunk_coords: Vec4::new(todo.chunk_pos.x, todo.chunk_pos.y, todo.chunk_pos.z, 0.0),
                scale: Vec4::splat(1.0),
                sea_level: Vec4::splat((sea_level.map(|x| x.level).unwrap_or(0.75) * (todo.structure_dimensions / 2) as f32) as f32),
                structure_pos: Vec4::new(structure_loc.x, structure_loc.y, structure_loc.z, 0.0),
            };

            worker.write("params", &params);

            let vals: Vec<f32> = vec![0.0; CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE * CHUNK_DIMENSIONS_USIZE];

            worker.write_slice("values", &vals);

            currently_generating_chunks.0.push(todo);
            worker.execute();
        }
    }
}

/// Calls generate_face_chunk, generate_edge_chunk, and generate_corner_chunk to generate the chunks of a planet.
pub fn generate_planet<T: BiosphereMarkerComponent, E: TGenerateChunkEvent>(
    mut query: Query<(&mut Structure, &Location)>,
    mut events: EventReader<E>,

    mut needs_generated_chunks: ResMut<NeedGeneratedChunks<T>>,
) {
    let chunks = events
        .read()
        .filter_map(|ev| {
            let structure_entity = ev.get_structure_entity();
            let coords = ev.get_chunk_coordinates();

            if let Ok((mut structure, _)) = query.get_mut(structure_entity) {
                let Structure::Dynamic(planet) = structure.as_mut() else {
                    panic!("A planet must be dynamic!");
                };
                Some((structure_entity, planet.take_or_create_chunk_for_loading(coords)))
            } else {
                None
            }
        })
        .collect::<Vec<(Entity, Chunk)>>();

    // let thread_pool = AsyncComputeTaskPool::get();

    needs_generated_chunks
        .0
        .extend(chunks.into_iter().flat_map(|(structure_entity, chunk)| {
            let Ok((structure, location)) = query.get(structure_entity) else {
                return None;
            };

            let Structure::Dynamic(planet) = structure else {
                panic!("A planet must be dynamic!");
            };

            let s_dimensions = planet.block_dimensions();
            let location = *location;
            let chunk_rel_pos = planet.chunk_relative_position(chunk.chunk_coordinates());

            Some(NeedGeneratedChunk {
                chunk,
                chunk_pos: chunk_rel_pos,
                structure_dimensions: s_dimensions,
                structure_entity,
                structure_location: location,
                _phantom: Default::default(),
            })
        }));
}

#[derive(Debug, ShaderType, Pod, Zeroable, Clone, Copy)]
#[repr(C)]
struct GenerationParams {
    // Everythihng has to be a vec4 because padding. Otherwise things get super wack
    chunk_coords: Vec4,
    structure_pos: Vec4,
    sea_level: Vec4,
    scale: Vec4,
}

#[derive(TypePath, Default)]
struct ComputeShaderInstance<T: BiosphereMarkerComponent + TypePath>(PhantomData<T>);

impl<T: BiosphereMarkerComponent + TypePath> ComputeShader for ComputeShaderInstance<T> {
    fn shader() -> ShaderRef {
        "cosmos/shaders/compute.wgsl".into()
    }
}

// If you change this, make sure to modify the '32' values in the shader aswell.
const SIZE: u32 = 32;
// If you change this, make sure to modify the '512' values in the shader aswell.
const WORKGROUP_SIZE: u32 = 512;

#[derive(Default)]
pub struct ShaderWorker<T: BiosphereMarkerComponent>(PhantomData<T>);

impl<T: BiosphereMarkerComponent + TypePath> ComputeWorker for ShaderWorker<T> {
    fn build(world: &mut bevy::prelude::World) -> AppComputeWorker<Self> {
        const DIMS: usize = (SIZE * SIZE * SIZE) as usize;

        // let noise = noise::OpenSimplex::new(1596);
        // noise.
        // let perm_table = PermutationTable;

        let params = GenerationParams {
            chunk_coords: Vec4::splat(13.0),
            structure_pos: Vec4::splat(12.0),
            sea_level: Vec4::splat(11.0),
            scale: Vec4::splat(10.0),
        };

        let icrs = vec![1.0; DIMS];

        assert!((SIZE * SIZE * SIZE) % WORKGROUP_SIZE == 0);

        let worker = AppComputeWorkerBuilder::new(world)
            .one_shot()
            // .add_empty_uniform("params", std::mem::size_of::<GenerationParams>() as u64) // GenerationParams
            .add_uniform("params", &params) // GenerationParams
            .add_staging("values", &icrs) // Vec<f32>
            .add_pass::<ComputeShaderInstance<T>>(
                [SIZE * SIZE * SIZE / WORKGROUP_SIZE, 1, 1], //SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE, SIZE / WORKGROUP_SIZE
                &["params", "values"],
            )
            .build();

        worker
    }
}

pub(super) fn register<T: BiosphereMarkerComponent>(app: &mut App) {
    app.add_plugins(AppComputeWorkerPlugin::<ShaderWorker<T>>::default())
        .init_resource::<NeedGeneratedChunks<T>>()
        .init_resource::<GeneratingChunks<T>>();
}
