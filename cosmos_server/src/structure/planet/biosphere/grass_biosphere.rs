//! Creates a grass planet

use bevy::prelude::{
    App, Commands, Component, Entity, EventReader, EventWriter, IntoSystemAppConfig, IntoSystemConfigs, OnEnter, OnUpdate, Query, Res,
};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::{Location, SECTOR_DIMENSIONS},
    registry::Registry,
    structure::{chunk::CHUNK_DIMENSIONS, ChunkInitEvent, Structure},
    utils::resource_wrapper::ResourceWrapper,
};
use noise::NoiseFn;

use crate::{init::init_world::ServerSeed, GameState};

use super::{
    biosphere_generation::{generate_planet, notify_when_done_generating_terrain, BlockRanges, GenerateChunkFeaturesEvent},
    register_biosphere, TBiosphere, TGenerateChunkEvent, TemperatureRange,
};

#[derive(Component, Debug, Default, Clone)]
/// Marks that this is for a grass biosphere
pub struct GrassBiosphereMarker;

/// Marks that a grass chunk needs generated
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
    commands.insert_resource(BlockRanges::<GrassBiosphereMarker>::new(vec![
        (block_registry.from_id("cosmos:stone").expect("Block missing").clone(), 5),
        (block_registry.from_id("cosmos:dirt").expect("Block missing").clone(), 1),
        (block_registry.from_id("cosmos:grass").expect("Block missing").clone(), 0),
    ]));
}

#[inline]
fn three_by_three_no_corners(
    (x, y, z): (usize, usize, usize),
    structure: &mut Structure,
    block: &Block,
    block_up: BlockFace,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    structure.set_block_at(x, y, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x + 1, y, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x - 1, y, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x, y, z + 1, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x, y, z - 1, block, block_up, blocks, Some(event_writer));
}

#[inline]
fn three_by_three(
    (x, y, z): (usize, usize, usize),
    structure: &mut Structure,
    block: &Block,
    block_up: BlockFace,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    for dz in 0..=2 {
        for dx in 0..=2 {
            structure.set_block_at(x + dx - 1, y, z + dz - 1, block, block_up, blocks, Some(event_writer));
        }
    }
}

#[inline]
fn three_by_three_plus(
    (x, y, z): (usize, usize, usize),
    structure: &mut Structure,
    block: &Block,
    block_up: BlockFace,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    three_by_three((x, y, z), structure, block, block_up, blocks, event_writer);
    structure.set_block_at(x + 2, y, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x - 2, y, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x, y, z + 2, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x, y, z - 2, block, block_up, blocks, Some(event_writer));
}

#[inline]
fn five_by_five_no_corners(
    (x, y, z): (usize, usize, usize),
    structure: &mut Structure,
    block: &Block,
    block_up: BlockFace,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    three_by_three_plus((x, y, z), structure, block, block_up, blocks, event_writer);
    structure.set_block_at(x + 2, y, z + 1, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x + 2, y, z - 1, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x - 2, y, z + 1, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x - 2, y, z - 1, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x + 1, y, z + 2, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x - 1, y, z + 2, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x + 1, y, z - 2, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x - 1, y, z - 2, block, block_up, blocks, Some(event_writer));
}

#[inline]
fn branch(
    (x, y, z): (i32, i32, i32),
    offsets: Vec<(i32, i32, i32, BlockFace)>,
    structure: &mut Structure,
    log: &Block,
    leaf: &Block,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    // Leaves. Must go first so they don't overwrite the logs.
    for (dx, dy, dz, block_up) in offsets.iter() {
        let (nx, ny, nz) = ((x + dx) as usize, (y + dy) as usize, (z + dz) as usize);
        structure.set_block_at(nx + 1, ny, nz, leaf, *block_up, blocks, Some(event_writer));
        structure.set_block_at(nx - 1, ny, nz, leaf, *block_up, blocks, Some(event_writer));
        structure.set_block_at(nx, ny + 1, nz, leaf, *block_up, blocks, Some(event_writer));
        structure.set_block_at(nx, ny - 1, nz, leaf, *block_up, blocks, Some(event_writer));
        structure.set_block_at(nx, ny, nz + 1, leaf, *block_up, blocks, Some(event_writer));
        structure.set_block_at(nx, ny, nz - 1, leaf, *block_up, blocks, Some(event_writer));
    }

    for (dx, dy, dz, block_up) in offsets {
        let (nx, ny, nz) = ((x + dx) as usize, (y + dy) as usize, (z + dz) as usize);
        structure.set_block_at(nx, ny, nz, log, block_up, blocks, Some(event_writer));
    }
}

#[inline]
// Plus-sign with the middle pushed up 1. Uses fewer lines to put this in a function bc of the formatter.
fn crown(
    (x, y, z): (usize, usize, usize),
    structure: &mut Structure,
    block: &Block,
    block_up: BlockFace,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    structure.set_block_at(x, y + 1, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x + 1, y, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x - 1, y, z, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x, y, z + 1, block, block_up, blocks, Some(event_writer));
    structure.set_block_at(x, y, z - 1, block, block_up, blocks, Some(event_writer));
}

const DELTA: f64 = 1.0;
const FOREST: f64 = 0.235;
const DIST_BETWEEN_TREES: usize = 5;
const SEGMENT_HEIGHT: usize = 10;
const BRANCH_START: f64 = 0.5;
const BETWEEN_BRANCHES: usize = 3;

// branch_leaves leaves overwriting nearby logs...

/// Sends a ChunkInitEvent for every chunk that's done generating, monitors when chunks are finished generating.
pub fn generate_chunk_features(
    mut event_reader: EventReader<GenerateChunkFeaturesEvent<GrassBiosphereMarker>>,
    mut init_event_writer: EventWriter<ChunkInitEvent>,
    mut block_event_writer: EventWriter<BlockChangedEvent>,
    mut structure_query: Query<(&mut Structure, &Location)>,
    blocks: Res<Registry<Block>>,
    noise_generator: Res<ResourceWrapper<noise::OpenSimplex>>,
) {
    let block_up = BlockFace::Top;
    for ev in event_reader.iter() {
        if let Ok((mut structure, location)) = structure_query.get_mut(ev.structure_entity) {
            let (cx, cy, cz) = ev.chunk_coords;
            let sx = cx * CHUNK_DIMENSIONS;
            let sy = cy * CHUNK_DIMENSIONS;
            let sz = cz * CHUNK_DIMENSIONS;

            let air = blocks.from_id("cosmos:air").unwrap();
            let grass = blocks.from_id("cosmos:grass").unwrap();
            let log = blocks.from_id("cosmos:redwood_log").unwrap();
            let leaf = blocks.from_id("cosmos:redwood_leaf").unwrap();

            let structure_coords = location.absolute_coords_f64();

            let noise_y = structure.blocks_height();
            let mut noise_cache = [[0.0; CHUNK_DIMENSIONS + DIST_BETWEEN_TREES * 2]; CHUNK_DIMENSIONS + DIST_BETWEEN_TREES * 2];
            for (z, slice) in noise_cache.iter_mut().enumerate() {
                let bz = sz + z;
                for (x, noise) in slice.iter_mut().enumerate() {
                    *noise = noise_generator.get([
                        ((sx + x) as f64 - DIST_BETWEEN_TREES as f64 + structure_coords.x) * DELTA,
                        (noise_y as f64 + structure_coords.y) * DELTA,
                        (bz as f64 - DIST_BETWEEN_TREES as f64 + structure_coords.z) * DELTA,
                    ]);
                }
            }

            for z in 0..CHUNK_DIMENSIONS {
                let bz = sz + z;
                'next: for x in 0..CHUNK_DIMENSIONS {
                    let bx = sx + x;
                    let mut y: i32 = CHUNK_DIMENSIONS as i32 - 1;
                    while y >= 0 && structure.block_at(bx, sy + y as usize, bz, &blocks) == air {
                        y -= 1;
                    }

                    let noise = noise_cache[z + DIST_BETWEEN_TREES][x + DIST_BETWEEN_TREES];
                    if y >= 0 && structure.block_at(bx, sy + y as usize, bz, &blocks) == grass && noise * noise > FOREST {
                        for dz in 0..=DIST_BETWEEN_TREES * 2 {
                            for dx in 0..=DIST_BETWEEN_TREES * 2 {
                                if noise < noise_cache[z + dz][x + dx] {
                                    continue 'next;
                                }
                            }
                        }
                        let by = sy + y as usize;

                        let height_noise = noise_generator.get([
                            (bx as f64 as f64 + structure_coords.x) * DELTA,
                            (by as f64 + structure_coords.y) * DELTA,
                            (bz as f64 as f64 + structure_coords.z) * DELTA,
                        ]);
                        let mut height = (4.0 * SEGMENT_HEIGHT as f64 + 4.0 * SEGMENT_HEIGHT as f64 * height_noise) as usize;
                        // Branches start the branch height from the bottom and spawn every 3 vertical blocks from the top.
                        let branch_height = (height as f64 * BRANCH_START) as usize;

                        // Trunk.
                        let mut dy = 1;

                        // 5x5 missing corners.
                        if height - dy >= SEGMENT_HEIGHT * 4 && dy == 1 {
                            height += SEGMENT_HEIGHT;
                        }
                        while height - dy >= SEGMENT_HEIGHT * 4 {
                            five_by_five_no_corners(
                                (bx, by + dy, bz),
                                &mut structure,
                                log,
                                BlockFace::Top,
                                &blocks,
                                &mut block_event_writer,
                            );
                            dy += 1;
                        }

                        // 3x3 with plus sign.
                        if height - dy >= SEGMENT_HEIGHT * 3 {
                            if dy == 1 {
                                height += SEGMENT_HEIGHT;
                            }
                        }
                        while height - dy >= SEGMENT_HEIGHT * 3 {
                            three_by_three_plus(
                                (bx, by + dy, bz),
                                &mut structure,
                                log,
                                BlockFace::Top,
                                &blocks,
                                &mut block_event_writer,
                            );
                            dy += 1;
                        }

                        // 3x3.
                        if height - dy >= SEGMENT_HEIGHT * 2 {
                            if dy == 1 {
                                height += SEGMENT_HEIGHT;
                            }
                        }
                        while height - dy >= SEGMENT_HEIGHT * 2 {
                            if dy >= branch_height && (height - dy) % BETWEEN_BRANCHES == 0 {
                                // 3 branch steps in each cardinal direction and 2 on each diagonal.
                                println!("Branch 4");
                                branch(
                                    (bx as i32, (by + dy) as i32, bz as i32),
                                    vec![
                                        (2, 0, 0, BlockFace::Right),
                                        (3, 0, 0, BlockFace::Right),
                                        (4, -1, 0, BlockFace::Right),
                                        (-2, 0, 0, BlockFace::Left),
                                        (-3, 0, 0, BlockFace::Left),
                                        (-4, -1, 0, BlockFace::Left),
                                        (0, 0, 2, BlockFace::Front),
                                        (0, 0, 3, BlockFace::Front),
                                        (0, -1, 4, BlockFace::Front),
                                        (0, 0, -2, BlockFace::Back),
                                        (0, 0, -3, BlockFace::Back),
                                        (0, -1, -4, BlockFace::Back),
                                        (2, 0, 2, BlockFace::Right),
                                        (3, -1, 3, BlockFace::Right),
                                        (-2, 0, 2, BlockFace::Front),
                                        (-3, -1, 3, BlockFace::Front),
                                        (2, 0, -2, BlockFace::Back),
                                        (3, -1, -3, BlockFace::Back),
                                        (-2, 0, -2, BlockFace::Left),
                                        (-3, -1, -3, BlockFace::Left),
                                    ],
                                    &mut structure,
                                    log,
                                    leaf,
                                    &blocks,
                                    &mut block_event_writer,
                                );
                            }
                            three_by_three((bx, by + dy, bz), &mut structure, log, block_up, &blocks, &mut block_event_writer);
                            dy += 1;
                        }

                        // 3x3 missing corners.
                        if height - dy >= SEGMENT_HEIGHT {
                            if dy == 1 {
                                height += SEGMENT_HEIGHT;
                            }
                        }
                        while height - dy >= SEGMENT_HEIGHT {
                            if dy >= branch_height && (height - dy) % BETWEEN_BRANCHES == 0 {
                                // 2 branch steps in each cardinal direction and 2 on each diagonal.
                                println!("Branch 3");
                                branch(
                                    (bx as i32, (by + dy) as i32, bz as i32),
                                    vec![
                                        (2, 0, 0, BlockFace::Right),
                                        (3, -1, 0, BlockFace::Right),
                                        (-2, 0, 0, BlockFace::Left),
                                        (-3, -1, 0, BlockFace::Left),
                                        (0, 0, 2, BlockFace::Front),
                                        (0, -1, 3, BlockFace::Front),
                                        (0, 0, -2, BlockFace::Back),
                                        (0, -1, -3, BlockFace::Back),
                                        (1, 0, 1, BlockFace::Right),
                                        (2, -1, 2, BlockFace::Right),
                                        (-1, 0, 1, BlockFace::Front),
                                        (-2, -1, 2, BlockFace::Front),
                                        (1, 0, -1, BlockFace::Back),
                                        (2, -1, -2, BlockFace::Back),
                                        (-1, 0, -1, BlockFace::Left),
                                        (-2, -1, -2, BlockFace::Left),
                                    ],
                                    &mut structure,
                                    log,
                                    leaf,
                                    &blocks,
                                    &mut block_event_writer,
                                );
                            }
                            three_by_three_no_corners((bx, by + dy, bz), &mut structure, log, block_up, &blocks, &mut block_event_writer);
                            dy += 1;
                        }

                        // 1x1.
                        while dy <= height {
                            structure.set_block_at(bx, by + dy, bz, log, block_up, &blocks, Some(&mut block_event_writer));
                            dy += 1;
                        }

                        // Top Segment Branches
                        crown(
                            (bx, by + height, bz),
                            &mut structure,
                            leaf,
                            block_up,
                            &blocks,
                            &mut block_event_writer,
                        );

                        // 4 1x1 branches.
                        branch(
                            (bx as i32, (by + height - BETWEEN_BRANCHES) as i32, bz as i32),
                            vec![
                                (1, 0, 0, BlockFace::Right),
                                (-1, 0, 0, BlockFace::Left),
                                (0, 0, 1, BlockFace::Front),
                                (0, 0, -1, BlockFace::Back),
                            ],
                            &mut structure,
                            log,
                            leaf,
                            &blocks,
                            &mut block_event_writer,
                        );

                        // for 1x1 trunk. Two branch steps in each cardinal direction.
                        for i in 2..=3 {
                            branch(
                                (bx as i32, (by + height - BETWEEN_BRANCHES * i) as i32, bz as i32),
                                vec![
                                    (1, 0, 0, BlockFace::Right),
                                    (2, -1, 0, BlockFace::Right),
                                    (-1, 0, 0, BlockFace::Left),
                                    (-2, -1, 0, BlockFace::Left),
                                    (0, 0, 1, BlockFace::Front),
                                    (0, -1, 2, BlockFace::Front),
                                    (0, 0, -1, BlockFace::Back),
                                    (0, -1, -2, BlockFace::Back),
                                ],
                                &mut structure,
                                log,
                                leaf,
                                &blocks,
                                &mut block_event_writer,
                            );
                        }
                    }
                }
            }

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
        TemperatureRange::new(0.0, 1000000000.0),
    );

    app.add_systems(
        (
            generate_planet::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>,
            notify_when_done_generating_terrain::<GrassBiosphereMarker>,
            generate_chunk_features,
        )
            .in_set(OnUpdate(GameState::Playing)),
    );

    app.add_system(make_block_ranges.in_schedule(OnEnter(GameState::PostLoading)));
}
