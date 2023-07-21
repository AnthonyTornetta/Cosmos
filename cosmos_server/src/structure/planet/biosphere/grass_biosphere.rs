//! Creates a grass planet

use bevy::prelude::{
    in_state, App, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, Update,
};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, planet::Planet, rotate, ChunkInitEvent, Structure},
    utils::resource_wrapper::ResourceWrapper,
};
use noise::NoiseFn;

use crate::GameState;

use super::{
    biosphere_generation::{
        generate_planet, notify_when_done_generating_terrain, BlockRanges, DefaultBiosphereGenerationStrategy, GenerateChunkFeaturesEvent,
        GenerationParemeters,
    },
    generation_tools::fill,
    register_biosphere, TBiosphere, TGenerateChunkEvent, TemperatureRange,
};

#[derive(Component, Debug, Default, Clone)]
/// Marks that this is for a grass biosphere
pub struct GrassBiosphereMarker;

/// Marks that a grass chunk needs generated
#[derive(Debug, Event)]
pub struct GrassChunkNeedsGeneratedEvent {
    x: usize,
    y: usize,
    z: usize,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for GrassChunkNeedsGeneratedEvent {
    fn new(x: usize, y: usize, z: usize, structure_entity: Entity) -> Self {
        Self { x, y, z, structure_entity }
    }

    fn get_structure_entity(&self) -> Entity {
        self.structure_entity
    }

    fn get_chunk_coordinates(&self) -> (usize, usize, usize) {
        (self.x, self.y, self.z)
    }
}

#[derive(Default, Debug)]
/// Creates a grass planet
pub struct GrassBiosphere;

impl TBiosphere<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent> for GrassBiosphere {
    fn get_marker_component(&self) -> GrassBiosphereMarker {
        GrassBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, x: usize, y: usize, z: usize, structure_entity: Entity) -> GrassChunkNeedsGeneratedEvent {
        GrassChunkNeedsGeneratedEvent::new(x, y, z, structure_entity)
    }
}

fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
    commands.insert_resource(
        BlockRanges::<GrassBiosphereMarker>::default()
            .with_range("cosmos:stone", &block_registry, 5)
            .expect("Stone missing")
            .with_range("cosmos:dirt", &block_registry, 1)
            .expect("Dirt missing")
            .with_range("cosmos:grass", &block_registry, 0)
            .expect("Grass missing"),
    );
}

#[inline]
fn branch(
    origin: (usize, usize, usize),
    logs: Vec<(i32, i32, i32, BlockFace)>,
    planet_face: BlockFace,
    structure: &mut Structure,
    log: &Block,
    leaf: &Block,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    let s_dims = (structure.blocks_width(), structure.blocks_height(), structure.blocks_length());

    // Leaves. Must go first so they don't overwrite the logs.
    for (dx, dy, dz, block_up) in logs.iter().copied() {
        if let Ok(rotated) = rotate(origin, (dx, dy, dz), s_dims, planet_face) {
            fill(
                rotated,
                &[(1, 0, 0), (-1, 0, 0), (0, 1, 0), (0, -1, 0), (0, 0, 1), (0, 0, -1)],
                leaf,
                block_up,
                planet_face,
                structure,
                blocks,
                event_writer,
            );
        }
    }

    // Logs, like the map from BTD6. dan you have a problem seek help
    for (dx, dy, dz, block_up) in logs {
        if let Ok(rotated) = rotate(origin, (dx, dy, dz), s_dims, planet_face) {
            structure.set_block_at_tuple(
                rotated,
                log,
                BlockFace::rotate_face(block_up, planet_face),
                blocks,
                Some(event_writer),
            );
        }
    }
}

/// Generates a redwood tree at the given coordinates.
fn redwood_tree(
    (bx, by, bz): (usize, usize, usize),
    planet_face: BlockFace,
    structure: &mut Structure,
    location: &Location,
    block_event_writer: &mut EventWriter<BlockChangedEvent>,
    blocks: &Registry<Block>,
    noise_generator: &ResourceWrapper<noise::OpenSimplex>,
) {
    let log = blocks.from_id("cosmos:redwood_log").unwrap();
    let leaf = blocks.from_id("cosmos:redwood_leaf").unwrap();

    let structure_coords = location.absolute_coords_f64();

    let height_noise = noise_generator.get([
        (bx as f64 + structure_coords.x) * DELTA,
        (by as f64 + structure_coords.y) * DELTA,
        (bz as f64 + structure_coords.z) * DELTA,
    ]);
    let mut height = (4.0 * SEGMENT_HEIGHT as f64 + 4.0 * SEGMENT_HEIGHT as f64 * height_noise) as usize;
    // Branches start the branch height from the bottom and spawn every 3 vertical blocks from the top.
    let branch_height = (height as f64 * BRANCH_START) as usize;

    // Top Segment Branches - Shifted up one segment bc the height gets shifted somewhere later between segments.
    // Leaf crown at the top of the tree.
    branch(
        (bx, by, bz),
        vec![(0, (height + SEGMENT_HEIGHT) as i32, 0, BlockFace::Top)],
        planet_face,
        structure,
        log,
        leaf,
        blocks,
        block_event_writer,
    );

    // 4 1x1 branches.
    let h = (height + SEGMENT_HEIGHT - BETWEEN_BRANCHES) as i32;
    branch(
        (bx, by, bz),
        vec![
            (1, h, 0, BlockFace::Right),
            (-1, h, 0, BlockFace::Left),
            (0, h, 1, BlockFace::Front),
            (0, h, -1, BlockFace::Back),
        ],
        planet_face,
        structure,
        log,
        leaf,
        blocks,
        block_event_writer,
    );

    // for 1x1 trunk. Two branch steps in each cardinal direction.
    for i in 2..=3 {
        let h = (height + SEGMENT_HEIGHT - BETWEEN_BRANCHES * i) as i32;
        branch(
            (bx, by, bz),
            vec![
                (1, h, 0, BlockFace::Right),
                (2, h - 1, 0, BlockFace::Right),
                (-1, h, 0, BlockFace::Left),
                (-2, h - 1, 0, BlockFace::Left),
                (0, h, 1, BlockFace::Front),
                (0, h - 1, 2, BlockFace::Front),
                (0, h, -1, BlockFace::Back),
                (0, h - 1, -2, BlockFace::Back),
            ],
            planet_face,
            structure,
            log,
            leaf,
            blocks,
            block_event_writer,
        );
    }

    // Trunk.
    let mut dy = 1;

    // 5x5 missing corners.
    if height - dy >= SEGMENT_HEIGHT * 4 && dy == 1 {
        height += SEGMENT_HEIGHT;
    }
    while height - dy >= SEGMENT_HEIGHT * 4 {
        let h = dy as i32;
        fill(
            (bx, by, bz),
            &[
                (0, h, 0),
                (1, h, 0),
                (-1, h, 0),
                (0, h, 1),
                (0, h, -1),
                (1, h, 1),
                (1, h, -1),
                (-1, h, 1),
                (-1, h, -1),
                (2, h, 0),
                (2, h, 1),
                (2, h, -1),
                (-2, h, 0),
                (-2, h, 1),
                (-2, h, -1),
                (0, h, 2),
                (1, h, 2),
                (-1, h, 2),
                (0, h, -2),
                (1, h, -2),
                (-1, h, -2),
            ],
            log,
            BlockFace::Top,
            planet_face,
            structure,
            blocks,
            block_event_writer,
        );
        dy += 1;
    }

    // 3x3 with plus sign.
    if height - dy >= SEGMENT_HEIGHT * 3 && dy == 1 {
        height += SEGMENT_HEIGHT;
    }
    while height - dy >= SEGMENT_HEIGHT * 3 {
        let h = dy as i32;
        fill(
            (bx, by, bz),
            &[
                (0, h, 0),
                (1, h, 0),
                (-1, h, 0),
                (0, h, 1),
                (0, h, -1),
                (1, h, 1),
                (1, h, -1),
                (-1, h, 1),
                (-1, h, -1),
                (2, h, 0),
                (-2, h, 0),
                (0, h, 2),
                (0, h, -2),
            ],
            log,
            BlockFace::Top,
            planet_face,
            structure,
            blocks,
            block_event_writer,
        );
        dy += 1;
    }

    // 3x3 trunk.
    if height - dy >= SEGMENT_HEIGHT * 2 && dy == 1 {
        height += SEGMENT_HEIGHT;
    }
    while height - dy >= SEGMENT_HEIGHT * 2 {
        if dy >= branch_height && (height - dy) % BETWEEN_BRANCHES == 0 {
            // 3 branch steps in each cardinal direction and 2 on each diagonal.
            let h: i32 = dy as i32;
            branch(
                (bx, by, bz),
                vec![
                    (2, h, 0, BlockFace::Right),
                    (3, h, 0, BlockFace::Right),
                    (4, h - 1, 0, BlockFace::Right),
                    (-2, h, 0, BlockFace::Left),
                    (-3, h, 0, BlockFace::Left),
                    (-4, h - 1, 0, BlockFace::Left),
                    (0, h, 2, BlockFace::Front),
                    (0, h, 3, BlockFace::Front),
                    (0, h - 1, 4, BlockFace::Front),
                    (0, h, -2, BlockFace::Back),
                    (0, h, -3, BlockFace::Back),
                    (0, h - 1, -4, BlockFace::Back),
                    (2, h, 2, BlockFace::Right),
                    (3, h - 1, 3, BlockFace::Right),
                    (-2, h, 2, BlockFace::Front),
                    (-3, h - 1, 3, BlockFace::Front),
                    (2, h, -2, BlockFace::Back),
                    (3, h - 1, -3, BlockFace::Back),
                    (-2, h, -2, BlockFace::Left),
                    (-3, h - 1, -3, BlockFace::Left),
                ],
                planet_face,
                structure,
                log,
                leaf,
                blocks,
                block_event_writer,
            );
        }
        let h = dy as i32;
        fill(
            (bx, by, bz),
            &[
                (0, h, 0),
                (1, h, 0),
                (-1, h, 0),
                (0, h, 1),
                (0, h, -1),
                (1, h, 1),
                (1, h, -1),
                (-1, h, 1),
                (-1, h, -1),
            ],
            log,
            BlockFace::Top,
            planet_face,
            structure,
            blocks,
            block_event_writer,
        );
        dy += 1;
    }

    // 3x3 missing corners trunk.
    if height - dy >= SEGMENT_HEIGHT && dy == 1 {
        height += SEGMENT_HEIGHT;
    }
    while height - dy >= SEGMENT_HEIGHT {
        if dy >= branch_height && (height - dy) % BETWEEN_BRANCHES == 0 {
            // 2 branch steps in each cardinal direction and 2 on each diagonal.
            let h = dy as i32;
            branch(
                (bx, by, bz),
                vec![
                    (2, h, 0, BlockFace::Right),
                    (3, h - 1, 0, BlockFace::Right),
                    (-2, h, 0, BlockFace::Left),
                    (-3, h - 1, 0, BlockFace::Left),
                    (0, h, 2, BlockFace::Front),
                    (0, h - 1, 3, BlockFace::Front),
                    (0, h, -2, BlockFace::Back),
                    (0, h - 1, -3, BlockFace::Back),
                    (1, h, 1, BlockFace::Right),
                    (2, h - 1, 2, BlockFace::Right),
                    (-1, h, 1, BlockFace::Front),
                    (-2, h - 1, 2, BlockFace::Front),
                    (1, h, -1, BlockFace::Back),
                    (2, h - 1, -2, BlockFace::Back),
                    (-1, h, -1, BlockFace::Left),
                    (-2, h - 1, -2, BlockFace::Left),
                ],
                planet_face,
                structure,
                log,
                leaf,
                blocks,
                block_event_writer,
            );
        }
        let h = dy as i32;
        fill(
            (bx, by, bz),
            &[(0, h, 0), (1, h, 0), (-1, h, 0), (0, h, 1), (0, h, -1)],
            log,
            BlockFace::Top,
            planet_face,
            structure,
            blocks,
            block_event_writer,
        );
        dy += 1;
    }

    let s_dims = (structure.blocks_width(), structure.blocks_height(), structure.blocks_length());

    // 1x1 trunk.
    while dy <= height {
        if let Ok(rotated) = rotate((bx, by, bz), (0, dy as i32, 0), s_dims, planet_face) {
            structure.set_block_at_tuple(
                rotated,
                log,
                BlockFace::rotate_face(BlockFace::Top, planet_face),
                blocks,
                Some(block_event_writer),
            );
        }
        dy += 1;
    }
}

// Fills the chunk at the given coordinates with redwood trees.
fn trees(
    (cx, cy, cz): (usize, usize, usize),
    structure: &mut Structure,
    location: &Location,
    block_event_writer: &mut EventWriter<BlockChangedEvent>,
    blocks: &Registry<Block>,
    noise_generator: &ResourceWrapper<noise::OpenSimplex>,
) {
    let (sx, sy, sz) = (cx * CHUNK_DIMENSIONS, cy * CHUNK_DIMENSIONS, cz * CHUNK_DIMENSIONS);
    let s_dimension = structure.blocks_height();
    let s_dims = (s_dimension, s_dimension, s_dimension);

    let air = blocks.from_id("cosmos:air").unwrap();
    let grass = blocks.from_id("cosmos:grass").unwrap();

    let structure_coords = location.absolute_coords_f64();

    let faces = Planet::chunk_planet_faces((sx, sy, sz), s_dimension);
    for block_up in faces.iter() {
        // Getting the noise value for every block in the chunk, to find where to put trees.
        let noise_height = match block_up {
            BlockFace::Front | BlockFace::Top | BlockFace::Right => structure.blocks_height(),
            _ => 0,
        };

        let mut noise_cache = [[0.0; CHUNK_DIMENSIONS + DIST_BETWEEN_TREES * 2]; CHUNK_DIMENSIONS + DIST_BETWEEN_TREES * 2];
        for (z, slice) in noise_cache.iter_mut().enumerate() {
            for (x, noise) in slice.iter_mut().enumerate() {
                let (nx, ny, nz) = match block_up {
                    BlockFace::Front | BlockFace::Back => (
                        (sx + x) as f64 - DIST_BETWEEN_TREES as f64,
                        (sy + z) as f64 - DIST_BETWEEN_TREES as f64,
                        noise_height as f64,
                    ),
                    BlockFace::Top | BlockFace::Bottom => (
                        (sx + x) as f64 - DIST_BETWEEN_TREES as f64,
                        noise_height as f64,
                        (sz + z) as f64 - DIST_BETWEEN_TREES as f64,
                    ),
                    BlockFace::Right | BlockFace::Left => (
                        noise_height as f64,
                        (sy + x) as f64 - DIST_BETWEEN_TREES as f64,
                        (sz + z) as f64 - DIST_BETWEEN_TREES as f64,
                    ),
                };
                *noise = noise_generator.get([
                    (nx + structure_coords.x) * DELTA,
                    (ny + structure_coords.y) * DELTA,
                    (nz + structure_coords.z) * DELTA,
                ]);
            }
        }

        for z in 0..CHUNK_DIMENSIONS {
            'next: for x in 0..CHUNK_DIMENSIONS {
                let noise = noise_cache[z + DIST_BETWEEN_TREES][x + DIST_BETWEEN_TREES];

                // Noise value not in forest range.
                if noise * noise <= FOREST {
                    continue 'next;
                }

                // Noise value not a local maximum of enough blocks.
                for dz in 0..=DIST_BETWEEN_TREES * 2 {
                    for dx in 0..=DIST_BETWEEN_TREES * 2 {
                        if noise < noise_cache[z + dz][x + dx] {
                            continue 'next;
                        }
                    }
                }

                let (bx, by, bz) = match block_up {
                    BlockFace::Front | BlockFace::Back => (sx + x, sy + z, sz),
                    BlockFace::Top | BlockFace::Bottom => (sx + x, sy, sz + z),
                    BlockFace::Right | BlockFace::Left => (sx, sy + x, sz + z),
                };
                let mut height: i32 = CHUNK_DIMENSIONS as i32 - 1;
                while height >= 0
                    && rotate((bx, by, bz), (0, height, 0), s_dims, block_up)
                        .map(|rotated| structure.block_at_tuple(rotated, blocks) == air)
                        .unwrap_or(false)
                {
                    height -= 1;
                }

                // No grass block to grow tree from.
                if let Ok(rotated) = rotate((bx, by, bz), (0, height, 0), s_dims, block_up) {
                    if height < 0
                        || structure.block_at_tuple(rotated, blocks) != grass
                        || structure.block_rotation_tuple(rotated) != block_up
                    {
                        continue 'next;
                    }

                    redwood_tree(rotated, block_up, structure, location, block_event_writer, blocks, noise_generator);
                }
            }
        }
    }
}

const DELTA: f64 = 1.0;
const FOREST: f64 = 0.235;
const DIST_BETWEEN_TREES: usize = 5;
const SEGMENT_HEIGHT: usize = 10;
const BRANCH_START: f64 = 0.5;
const BETWEEN_BRANCHES: usize = 3;

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating, makes trees.
pub fn generate_chunk_features(
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<GrassBiosphereMarker>>,
    mut init_event_writer: EventWriter<ChunkInitEvent>,
    mut block_event_writer: EventWriter<BlockChangedEvent>,
    mut structure_query: Query<(&mut Structure, &Location)>,
    blocks: Res<Registry<Block>>,
    noise_generator: Res<ResourceWrapper<noise::OpenSimplex>>,
) {
    for ev in event_reader.iter() {
        if let Ok((mut structure, location)) = structure_query.get_mut(ev.structure_entity) {
            let (cx, cy, cz) = ev.chunk_coords;
            trees(
                (cx, cy, cz),
                &mut structure,
                location,
                &mut block_event_writer,
                &blocks,
                &noise_generator,
            );

            init_event_writer.send(ChunkInitEvent {
                structure_entity: ev.structure_entity,
                x: cx,
                y: cy,
                z: cz,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_grass",
        TemperatureRange::new(255.0, 500.0),
    );

    app.add_systems(
        Update,
        (
            generate_planet::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent, DefaultBiosphereGenerationStrategy>,
            notify_when_done_generating_terrain::<GrassBiosphereMarker>,
            generate_chunk_features,
        )
            .run_if(in_state(GameState::Playing)),
    )
    .insert_resource(GenerationParemeters::<GrassBiosphereMarker>::new(0.05, 7.0, 9));

    app.add_systems(OnEnter(GameState::PostLoading), make_block_ranges);
}
