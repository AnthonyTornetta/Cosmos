//! Creates a grass planet

use bevy::prelude::{
    in_state, App, Commands, Component, Entity, Event, EventReader, EventWriter, IntoSystemConfigs, OnEnter, Query, Res, Update,
};
use cosmos_core::{
    block::{Block, BlockFace},
    events::block_events::BlockChangedEvent,
    physics::location::Location,
    registry::Registry,
    structure::{
        chunk::CHUNK_DIMENSIONS,
        coordinates::{BlockCoordinate, ChunkCoordinate, CoordinateType, UnboundBlockCoordinate, UnboundCoordinateType},
        planet::Planet,
        rotate, ChunkInitEvent, Structure,
    },
    utils::resource_wrapper::ResourceWrapper,
};
use noise::NoiseFn;

use crate::GameState;

use super::{
    biosphere_generation::{
        generate_planet, notify_when_done_generating_terrain, BlockLayers, DefaultBiosphereGenerationStrategy, GenerateChunkFeaturesEvent,
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
    coords: ChunkCoordinate,
    structure_entity: Entity,
}

impl TGenerateChunkEvent for GrassChunkNeedsGeneratedEvent {
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
/// Creates a grass planet
pub struct GrassBiosphere;

impl TBiosphere<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent> for GrassBiosphere {
    fn get_marker_component(&self) -> GrassBiosphereMarker {
        GrassBiosphereMarker {}
    }

    fn get_generate_chunk_event(&self, coords: ChunkCoordinate, structure_entity: Entity) -> GrassChunkNeedsGeneratedEvent {
        GrassChunkNeedsGeneratedEvent::new(coords, structure_entity)
    }
}

fn make_block_ranges(block_registry: Res<Registry<Block>>, mut commands: Commands) {
    commands.insert_resource(
        BlockLayers::<GrassBiosphereMarker>::default()
            .add_noise_layer("cosmos:short_grass", &block_registry, 160, 0.05, 7.0, 9)
            .expect("Short Grass missing")
            .add_fixed_layer("cosmos:grass", &block_registry, 1)
            .expect("Grass missing")
            .add_fixed_layer("cosmos:dirt", &block_registry, 1)
            .expect("Dirt missing")
            .add_fixed_layer("cosmos:stone", &block_registry, 4)
            .expect("Stone missing"),
    );
}

#[inline]
fn branch(
    origin: BlockCoordinate,
    logs: Vec<(UnboundBlockCoordinate, BlockFace)>,
    planet_face: BlockFace,
    structure: &mut Structure,
    log: &Block,
    leaf: &Block,
    blocks: &Registry<Block>,
    event_writer: &mut EventWriter<BlockChangedEvent>,
) {
    let s_dims = structure.block_dimensions();

    // Leaves. Must go first so they don't overwrite the logs.
    for (delta, block_up) in logs.iter().copied() {
        if let Ok(rotated) = rotate(origin, delta, s_dims, planet_face) {
            fill(
                rotated,
                &[
                    (1, 0, 0).into(),
                    (-1, 0, 0).into(),
                    (0, 1, 0).into(),
                    (0, -1, 0).into(),
                    (0, 0, 1).into(),
                    (0, 0, -1).into(),
                ],
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
    for (delta, block_up) in logs {
        if let Ok(rotated) = rotate(origin, delta, s_dims, planet_face) {
            structure.set_block_at(
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
    coords: BlockCoordinate,
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
        (coords.x as f64 + structure_coords.x) * DELTA,
        (coords.y as f64 + structure_coords.y) * DELTA,
        (coords.z as f64 + structure_coords.z) * DELTA,
    ]);
    let mut height = (4.0 * SEGMENT_HEIGHT as f64 + 4.0 * SEGMENT_HEIGHT as f64 * height_noise) as CoordinateType;
    // Branches start the branch height from the bottom and spawn every 3 vertical blocks from the top.
    let branch_height = (height as f64 * BRANCH_START) as CoordinateType;

    // Top Segment Branches - Shifted up one segment bc the height gets shifted somewhere later between segments.
    // Leaf crown at the top of the tree.
    branch(
        coords,
        vec![(
            UnboundBlockCoordinate::new(0, (height + SEGMENT_HEIGHT) as UnboundCoordinateType, 0),
            BlockFace::Top,
        )],
        planet_face,
        structure,
        log,
        leaf,
        blocks,
        block_event_writer,
    );

    // 4 1x1 branches.
    let h = (height + SEGMENT_HEIGHT - BETWEEN_BRANCHES) as UnboundCoordinateType;
    branch(
        coords,
        vec![
            ((1, h, 0).into(), BlockFace::Right),
            ((-1, h, 0).into(), BlockFace::Left),
            ((0, h, 1).into(), BlockFace::Front),
            ((0, h, -1).into(), BlockFace::Back),
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
        let h = (height + SEGMENT_HEIGHT - BETWEEN_BRANCHES * i) as UnboundCoordinateType;
        branch(
            coords,
            vec![
                ((1, h, 0).into(), BlockFace::Right),
                ((2, h - 1, 0).into(), BlockFace::Right),
                ((-1, h, 0).into(), BlockFace::Left),
                ((-2, h - 1, 0).into(), BlockFace::Left),
                ((0, h, 1).into(), BlockFace::Front),
                ((0, h - 1, 2).into(), BlockFace::Front),
                ((0, h, -1).into(), BlockFace::Back),
                ((0, h - 1, -2).into(), BlockFace::Back),
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
        let h = dy as UnboundCoordinateType;
        fill(
            coords,
            &[
                (0, h, 0).into(),
                (1, h, 0).into(),
                (-1, h, 0).into(),
                (0, h, 1).into(),
                (0, h, -1).into(),
                (1, h, 1).into(),
                (1, h, -1).into(),
                (-1, h, 1).into(),
                (-1, h, -1).into(),
                (2, h, 0).into(),
                (2, h, 1).into(),
                (2, h, -1).into(),
                (-2, h, 0).into(),
                (-2, h, 1).into(),
                (-2, h, -1).into(),
                (0, h, 2).into(),
                (1, h, 2).into(),
                (-1, h, 2).into(),
                (0, h, -2).into(),
                (1, h, -2).into(),
                (-1, h, -2).into(),
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
        let h = dy as UnboundCoordinateType;
        fill(
            coords,
            &[
                (0, h, 0).into(),
                (1, h, 0).into(),
                (-1, h, 0).into(),
                (0, h, 1).into(),
                (0, h, -1).into(),
                (1, h, 1).into(),
                (1, h, -1).into(),
                (-1, h, 1).into(),
                (-1, h, -1).into(),
                (2, h, 0).into(),
                (-2, h, 0).into(),
                (0, h, 2).into(),
                (0, h, -2).into(),
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
            let h = dy as UnboundCoordinateType;
            branch(
                coords,
                vec![
                    ((2, h, 0).into(), BlockFace::Right),
                    ((3, h, 0).into(), BlockFace::Right),
                    ((4, h - 1, 0).into(), BlockFace::Right),
                    ((-2, h, 0).into(), BlockFace::Left),
                    ((-3, h, 0).into(), BlockFace::Left),
                    ((-4, h - 1, 0).into(), BlockFace::Left),
                    ((0, h, 2).into(), BlockFace::Front),
                    ((0, h, 3).into(), BlockFace::Front),
                    ((0, h - 1, 4).into(), BlockFace::Front),
                    ((0, h, -2).into(), BlockFace::Back),
                    ((0, h, -3).into(), BlockFace::Back),
                    ((0, h - 1, -4).into(), BlockFace::Back),
                    ((2, h, 2).into(), BlockFace::Right),
                    ((3, h - 1, 3).into(), BlockFace::Right),
                    ((-2, h, 2).into(), BlockFace::Front),
                    ((-3, h - 1, 3).into(), BlockFace::Front),
                    ((2, h, -2).into(), BlockFace::Back),
                    ((3, h - 1, -3).into(), BlockFace::Back),
                    ((-2, h, -2).into(), BlockFace::Left),
                    ((-3, h - 1, -3).into(), BlockFace::Left),
                ],
                planet_face,
                structure,
                log,
                leaf,
                blocks,
                block_event_writer,
            );
        }
        let h = dy as UnboundCoordinateType;
        fill(
            coords,
            &[
                (0, h, 0).into(),
                (1, h, 0).into(),
                (-1, h, 0).into(),
                (0, h, 1).into(),
                (0, h, -1).into(),
                (1, h, 1).into(),
                (1, h, -1).into(),
                (-1, h, 1).into(),
                (-1, h, -1).into(),
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
            let h = dy as UnboundCoordinateType;
            branch(
                coords,
                vec![
                    ((2, h, 0).into(), BlockFace::Right),
                    ((3, h - 1, 0).into(), BlockFace::Right),
                    ((-2, h, 0).into(), BlockFace::Left),
                    ((-3, h - 1, 0).into(), BlockFace::Left),
                    ((0, h, 2).into(), BlockFace::Front),
                    ((0, h - 1, 3).into(), BlockFace::Front),
                    ((0, h, -2).into(), BlockFace::Back),
                    ((0, h - 1, -3).into(), BlockFace::Back),
                    ((1, h, 1).into(), BlockFace::Right),
                    ((2, h - 1, 2).into(), BlockFace::Right),
                    ((-1, h, 1).into(), BlockFace::Front),
                    ((-2, h - 1, 2).into(), BlockFace::Front),
                    ((1, h, -1).into(), BlockFace::Back),
                    ((2, h - 1, -2).into(), BlockFace::Back),
                    ((-1, h, -1).into(), BlockFace::Left),
                    ((-2, h - 1, -2).into(), BlockFace::Left),
                ],
                planet_face,
                structure,
                log,
                leaf,
                blocks,
                block_event_writer,
            );
        }
        let h = dy as UnboundCoordinateType;
        fill(
            coords,
            &[
                (0, h, 0).into(),
                (1, h, 0).into(),
                (-1, h, 0).into(),
                (0, h, 1).into(),
                (0, h, -1).into(),
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

    let s_dims = structure.block_dimensions();

    // 1x1 trunk.
    while dy <= height {
        if let Ok(rotated) = rotate(
            coords,
            UnboundBlockCoordinate::new(0, dy as UnboundCoordinateType, 0),
            s_dims,
            planet_face,
        ) {
            structure.set_block_at(
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
    coords: ChunkCoordinate,
    structure: &mut Structure,
    location: &Location,
    block_event_writer: &mut EventWriter<BlockChangedEvent>,
    blocks: &Registry<Block>,
    noise_generator: &ResourceWrapper<noise::OpenSimplex>,
) {
    let Structure::Dynamic(planet) = structure else {
        panic!("A planet must be dynamic!");
    };

    let first_block_coords = coords.first_structure_block();
    let s_dimension = planet.block_dimensions();
    let s_dims = structure.block_dimensions();

    let air = blocks.from_id("cosmos:air").unwrap();
    let short_grass = blocks.from_id("cosmos:short_grass").unwrap();
    let grass = blocks.from_id("cosmos:grass").unwrap();

    let structure_coords = location.absolute_coords_f64();

    let faces = Planet::chunk_planet_faces(first_block_coords, s_dimension);
    for block_up in faces.iter() {
        // Getting the noise value for every block in the chunk, to find where to put trees.
        let noise_height = match block_up {
            BlockFace::Front | BlockFace::Top | BlockFace::Right => s_dimension,
            _ => 0,
        };

        let mut noise_cache =
            [[0.0; (CHUNK_DIMENSIONS + DIST_BETWEEN_TREES * 2) as usize]; (CHUNK_DIMENSIONS + DIST_BETWEEN_TREES * 2) as usize];

        for (z, slice) in noise_cache.iter_mut().enumerate().map(|(z, noise)| (z as CoordinateType, noise)) {
            for (x, noise) in slice.iter_mut().enumerate().map(|(x, noise)| (x as CoordinateType, noise)) {
                let (nx, ny, nz) = match block_up {
                    BlockFace::Front | BlockFace::Back => (
                        (first_block_coords.x + x) as f64 - DIST_BETWEEN_TREES as f64,
                        (first_block_coords.z + z) as f64 - DIST_BETWEEN_TREES as f64,
                        noise_height as f64,
                    ),
                    BlockFace::Top | BlockFace::Bottom => (
                        (first_block_coords.x + x) as f64 - DIST_BETWEEN_TREES as f64,
                        noise_height as f64,
                        (first_block_coords.z + z) as f64 - DIST_BETWEEN_TREES as f64,
                    ),
                    BlockFace::Right | BlockFace::Left => (
                        noise_height as f64,
                        (first_block_coords.y + x) as f64 - DIST_BETWEEN_TREES as f64,
                        (first_block_coords.z + z) as f64 - DIST_BETWEEN_TREES as f64,
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
                let noise = noise_cache[(z + DIST_BETWEEN_TREES) as usize][(x + DIST_BETWEEN_TREES) as usize];

                // Noise value not in forest range.
                if noise * noise <= FOREST {
                    continue 'next;
                }

                // Noise value not a local maximum of enough blocks.
                for dz in 0..=DIST_BETWEEN_TREES * 2 {
                    for dx in 0..=DIST_BETWEEN_TREES * 2 {
                        if noise < noise_cache[(z + dz) as usize][(x + dx) as usize] {
                            continue 'next;
                        }
                    }
                }

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

                // // No grass block to grow tree from.
                if let Ok(rotated) = rotate(coords, UnboundBlockCoordinate::new(0, height, 0), s_dims, block_up) {
                    let block = structure.block_at(rotated, blocks);
                    if height < 0 || (block != grass && block != short_grass) || structure.block_rotation(rotated) != block_up {
                        continue 'next;
                    }

                    if let Ok(lowered_rotated) = rotate(coords, UnboundBlockCoordinate::new(0, height - 4, 0), s_dims, block_up) {
                        redwood_tree(
                            lowered_rotated,
                            block_up,
                            structure,
                            location,
                            block_event_writer,
                            blocks,
                            noise_generator,
                        );
                    }
                }
            }
        }
    }
}

const DELTA: f64 = 1.0;
const FOREST: f64 = 0.235;
const DIST_BETWEEN_TREES: CoordinateType = 5;
const SEGMENT_HEIGHT: CoordinateType = 10;
const BRANCH_START: f64 = 0.5;
const BETWEEN_BRANCHES: CoordinateType = 3;

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
            let coords = ev.chunk_coords;
            trees(coords, &mut structure, location, &mut block_event_writer, &blocks, &noise_generator);

            init_event_writer.send(ChunkInitEvent {
                structure_entity: ev.structure_entity,
                coords,
            });
        }
    }
}

pub(super) fn register(app: &mut App) {
    register_biosphere::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent>(
        app,
        "cosmos:biosphere_grass",
        TemperatureRange::new(50.0, 5000.0),
    );

    app.add_systems(
        Update,
        (
            generate_planet::<GrassBiosphereMarker, GrassChunkNeedsGeneratedEvent, DefaultBiosphereGenerationStrategy>,
            notify_when_done_generating_terrain::<GrassBiosphereMarker>,
            generate_chunk_features,
        )
            .run_if(in_state(GameState::Playing)),
    );

    app.add_systems(OnEnter(GameState::PostLoading), make_block_ranges);
}
