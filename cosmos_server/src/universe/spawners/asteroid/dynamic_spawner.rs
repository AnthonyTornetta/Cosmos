//! Responsible for spawning asteroids that move, based on the random timing of them spawning

use bevy::{
    log::error,
    prelude::{
        App, Commands, Component, Entity, EventWriter, IntoSystemConfigs, Query, Res, Resource, Update, Vec3, With, Without, in_state,
    },
    reflect::Reflect,
    time::Time,
    platform::collections::HashMap,
};
use bevy_rapier3d::prelude::Velocity;
use cosmos_core::{
    block::Block,
    entities::player::Player,
    netty::{sync::IdentifiableComponent, system_sets::NetworkingSystemsSet},
    physics::location::Location,
    prelude::{Asteroid, BlockCoordinate},
    registry::Registry,
    state::GameState,
    structure::{
        ChunkInitEvent, Structure, asteroid::MovingAsteroid, coordinates::ChunkCoordinate, full_structure::FullStructure,
        loading::ChunksNeedLoaded, structure_iterator::ChunkIteratorResult,
    },
    utils::{quat_math::random_quat, random::random_range, timer::UtilsTimer},
};
use noise::NoiseFn;
use rand::seq::IteratorRandom;
use serde::{Deserialize, Serialize};

use crate::{
    init::init_world::Noise,
    persistence::{
        make_persistent::{DefaultPersistentComponent, make_persistent},
        saving::NeverSave,
    },
    structure::asteroid::generator::AsteroidGenerationSet,
};

#[derive(Component, Serialize, Deserialize, Reflect)]
struct NextDynamicAsteroidSpawnTime(f32);

impl IdentifiableComponent for NextDynamicAsteroidSpawnTime {
    fn get_component_unlocalized_name() -> &'static str {
        "cosmos:next_dynamic_asteroid_spawn_time"
    }
}

impl DefaultPersistentComponent for NextDynamicAsteroidSpawnTime {}

impl NextDynamicAsteroidSpawnTime {
    const MIN_SPAWN_TIME: f32 = 1000.0;
    const MAX_SPAWN_TIME: f32 = 3000.0;
    fn generate_next_spawn_time(&mut self) {
        self.0 += random_range(Self::MIN_SPAWN_TIME, Self::MAX_SPAWN_TIME);
    }
}

#[derive(Component)]
struct SmallAsteroidNeedsCreated {
    id: &'static str,
}

fn spawn_tiny_asteroids(
    mut q_spawns: Query<(&Location, &mut NextDynamicAsteroidSpawnTime)>,
    // settings: Res<ServerSettings>,
    mut commands: Commands,
    time: Res<Time>,
    asteroids: Res<SmallAsteroidTypes>,
) {
    // TODO: Make a different setting for these
    // if !settings.spawn_asteroids {
    //     return;
    // }

    for (loc, mut next_spawn_time) in q_spawns.iter_mut() {
        if next_spawn_time.0 != 0.0 {
            next_spawn_time.0 = (next_spawn_time.0 - time.delta_secs()).max(0.0);
            continue;
        }

        next_spawn_time.generate_next_spawn_time();

        let n_asteroids = random_range(1.0, 2.0).round() as usize;

        let mut rng = rand::rng();

        let random_dir = random_quat(&mut rng);
        let variation_dir = random_quat(&mut rng);

        const MIN_DISTANCE: f32 = 20_000.0;
        const MAX_DISTANCE: f32 = 25_000.0;
        let delta = random_dir * Vec3::new(0.0, 0.0, random_range(MIN_DISTANCE, MAX_DISTANCE));

        let mut spawn_loc = *loc + delta;

        for _ in 0..n_asteroids {
            let structure = Structure::Full(FullStructure::new(ChunkCoordinate::new(2, 2, 2)));

            spawn_loc = spawn_loc + variation_dir * Vec3::new(0.0, 0.0, random_range(400.0, 1000.0));

            // temperature is meaningless for now
            let temperature = 100.0;

            const ANGVEL_MAX: f32 = 0.05;

            const FUDGE_AMOUNT: f32 = 0.05;

            let velocity = Velocity {
                linvel: -(delta.normalize()
                    + Vec3::new(
                        random_range(-FUDGE_AMOUNT, FUDGE_AMOUNT),
                        random_range(-FUDGE_AMOUNT, FUDGE_AMOUNT),
                        random_range(-FUDGE_AMOUNT, FUDGE_AMOUNT),
                    ))
                    * random_range(10.0, 100.0),
                angvel: Vec3::new(
                    random_range(-ANGVEL_MAX, ANGVEL_MAX),
                    random_range(-ANGVEL_MAX, ANGVEL_MAX),
                    random_range(-ANGVEL_MAX, ANGVEL_MAX),
                ),
            };

            let (random_type, _) = asteroids.0.iter().choose(&mut rand::rng()).expect("No tiny asteroids :(");
            commands.spawn((
                Asteroid::new(temperature),
                spawn_loc,
                structure,
                NeverSave,
                SmallAsteroidNeedsCreated { id: random_type },
                MovingAsteroid,
                velocity,
            ));
        }
    }
}

fn send_done_generating_event(
    mut q_needs_gen: Query<(Entity, &mut Structure), With<SmallAsteroidNeedsCreated>>,
    mut commands: Commands,
    mut chunk_init_event_writer: EventWriter<ChunkInitEvent>,
) {
    for (ent, mut s) in q_needs_gen.iter_mut() {
        if let Structure::Full(structure) = s.as_mut() {
            structure.set_loaded();
        } else {
            panic!("Asteroid must be a full structure!");
        }

        let itr = s.all_chunks_iter(false);

        commands
            .entity(ent)
            .insert(ChunksNeedLoaded { amount_needed: itr.len() })
            .remove::<SmallAsteroidNeedsCreated>();

        for res in itr {
            // This will always be true because include_empty is false
            if let ChunkIteratorResult::FilledChunk { position, chunk: _ } = res {
                chunk_init_event_writer.write(ChunkInitEvent {
                    structure_entity: ent,
                    coords: position,
                    serialized_block_data: None,
                });
            }
        }
    }
}

fn add_next_dynamic_asteroid_spawn_time(
    mut commands: Commands,
    q_players: Query<Entity, (With<Player>, Without<NextDynamicAsteroidSpawnTime>)>,
) {
    for ent in q_players.iter() {
        let mut next_spawn_time = NextDynamicAsteroidSpawnTime(0.0);
        next_spawn_time.generate_next_spawn_time();
        commands.entity(ent).insert(next_spawn_time);
    }
}

#[derive(Debug, Clone)]
/// A block that may generate on the asteroid
pub struct SmallAsteroidBlockEntry {
    /// The unlocalized name for the block you want to generate
    pub block_id: &'static str,
    /// 1.0 = common
    /// 0.3 = rare
    ///
    /// Note that the more things you have generating, the less likely any given block is going to
    /// be chosen.
    /// Anything lower than 0.3 may not even show up on a given asteroid.
    pub rarity: f32,
}

#[derive(Resource, Default)]
struct SmallAsteroidTypes(HashMap<&'static str, Vec<SmallAsteroidBlockEntry>>);

fn register_small_asteroid_generator(app: &mut App, id: &'static str, asteroid_blocks: Vec<SmallAsteroidBlockEntry>) {
    let world = app.world_mut();
    if !world.contains_resource::<SmallAsteroidTypes>() {
        world.init_resource::<SmallAsteroidTypes>();
    }

    world.resource_mut::<SmallAsteroidTypes>().0.insert(id, asteroid_blocks);
}

fn register_small_asteroid_generation(app: &mut App, id: &'static str, block_entries: Vec<SmallAsteroidBlockEntry>) {
    register_small_asteroid_generator(app, id, block_entries);

    let start_generating_molten_asteroid = move |mut q_asteroids: Query<(&mut Structure, &Location, &SmallAsteroidNeedsCreated)>,
                                                 noise: Res<Noise>,
                                                 asteroid_types: Res<SmallAsteroidTypes>,
                                                 blocks: Res<Registry<Block>>| {
        for (mut structure, loc, needs_created) in q_asteroids.iter_mut() {
            let Some(block_entries) = asteroid_types.0.get(needs_created.id) else {
                error!("Invalid asteroid type: {}", needs_created.id);
                continue;
            };

            let (local_x, local_y, local_z) = (loc.local.x as f64, loc.local.y as f64, loc.local.z as f64);

            let (bx, by, bz) = structure.block_dimensions().into();

            let noise = noise.clone();

            let distance_threshold = (bz as f64 / 4.0 * (noise.get([local_x, local_y, local_z]).abs() + 1.0).min(25.0)) as f32;

            let timer = UtilsTimer::start();

            let ore_blocks = block_entries
                .iter()
                .map(|x| {
                    (
                        blocks.from_id(x.block_id).unwrap_or_else(|| panic!("Missing block {}", x.block_id)),
                        x.rarity,
                    )
                })
                .collect::<Vec<_>>();

            for z in 0..bz {
                for y in 0..by {
                    for x in 0..bx {
                        let x_pos = x as f32 - bx as f32 / 2.0;
                        let y_pos = y as f32 - by as f32 / 2.0;
                        let z_pos = z as f32 - bz as f32 / 2.0;

                        let noise_here = (noise.get([
                            x_pos as f64 * 0.03 + local_x,
                            y_pos as f64 * 0.03 + local_y,
                            z_pos as f64 * 0.03 + local_z,
                        ]) * 150.0) as f32;

                        let dist = x_pos * x_pos + y_pos * y_pos + z_pos * z_pos + noise_here * noise_here;

                        let distance_threshold = distance_threshold + noise_here / 3.0;

                        if dist < distance_threshold * distance_threshold {
                            let coords = BlockCoordinate::new(x, y, z);
                            let (block, _) = ore_blocks
                                .iter()
                                .map(|&(block, rarity)| (block, rand::random::<f32>() * rarity))
                                .max_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
                                .expect("No blocks present for tiny asteroid!");

                            structure.set_block_at(coords, block, Default::default(), &blocks, None);
                        }
                    }
                }
            }

            timer.log_duration(&format!("Tiny Asteroid {bx}x{by}x{bz} generation time: {bx}:"));
        }
    };

    app.add_systems(
        Update,
        start_generating_molten_asteroid
            .in_set(AsteroidGenerationSet::GenerateAsteroid)
            .ambiguous_with(AsteroidGenerationSet::GenerateAsteroid)
            .run_if(in_state(GameState::Playing)),
    );
}

pub(super) fn register(app: &mut App) {
    app.init_resource::<SmallAsteroidTypes>();

    make_persistent::<NextDynamicAsteroidSpawnTime>(app);

    register_small_asteroid_generation(
        app,
        "cosmos:gravitron",
        vec![
            SmallAsteroidBlockEntry {
                block_id: "cosmos:gravitron_crystal_ore",
                rarity: 1.0,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:iron_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:copper_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:energite_crystal_ore",
                rarity: 0.4,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:stone",
                rarity: 1.0,
            },
        ],
    );

    register_small_asteroid_generation(
        app,
        "cosmos:photonium",
        vec![
            SmallAsteroidBlockEntry {
                block_id: "cosmos:photonium_crystal_ore",
                rarity: 1.0,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:iron_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:copper_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:gravitron_crystal_ore",
                rarity: 0.4,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:stone",
                rarity: 1.0,
            },
        ],
    );

    register_small_asteroid_generation(
        app,
        "cosmos:energite",
        vec![
            SmallAsteroidBlockEntry {
                block_id: "cosmos:energite_crystal_ore",
                rarity: 1.0,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:iron_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:copper_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:photonium_crystal_ore",
                rarity: 0.4,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:stone",
                rarity: 1.0,
            },
        ],
    );

    register_small_asteroid_generation(
        app,
        "cosmos:lead",
        vec![
            SmallAsteroidBlockEntry {
                block_id: "cosmos:lead_ore",
                rarity: 1.0,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:iron_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:copper_ore",
                rarity: 0.2,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:uranium_ore",
                rarity: 0.4,
            },
            SmallAsteroidBlockEntry {
                block_id: "cosmos:stone",
                rarity: 1.0,
            },
        ],
    );

    app.add_systems(
        Update,
        (
            spawn_tiny_asteroids.in_set(AsteroidGenerationSet::StartGeneratingAsteroid),
            send_done_generating_event.in_set(AsteroidGenerationSet::NotifyFinished),
        )
            .chain()
            .run_if(in_state(GameState::Playing)),
    )
    .register_type::<NextDynamicAsteroidSpawnTime>()
    .add_systems(Update, add_next_dynamic_asteroid_spawn_time.in_set(NetworkingSystemsSet::Between));
}
