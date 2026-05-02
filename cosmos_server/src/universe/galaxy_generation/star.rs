use bevy::prelude::*;
use rand::RngExt;

use crate::universe::UniverseSystems;

use super::*;

const GALAXY_THICKNESS: u32 = 2;

const CORE_X_DIST: u32 = 5;
const CORE_Y_DIST: u32 = 5;

const ARM_X_DIST: u32 = 10;
const ARM_Y_DIST: u32 = 5;
const ARM_X_MEAN: u32 = 20;
const ARM_Y_MEAN: u32 = 10;

const SPIRAL: u32 = 3;
const ARMS: u32 = 3;

// TY: https://www.youtube.com/watch?v=rd_VCToelw4

fn spiral(x: f32, y: f32, z: f32, offset: f32) -> Vec3 {
    let r = (x * x + y * y).sqrt();
    let mut theta = offset;
    theta += if x > 0.0 { (y / x).atan() } else { (y / x).atan() + PI };
    theta += (r / ARM_X_DIST as f32) * SPIRAL as f32;

    Vec3::new(r * theta.cos(), r * theta.sin(), z)
}

fn guassian_random(rng: &mut ChaCha8Rng, mean: f32, stdev: f32) -> f32 {
    let u = 1.0 - rng.random::<f32>();
    let v = rng.random::<f32>();
    let z = (-2.0 * u.ln()).sqrt() * (TAU * v).cos();

    z * stdev + mean
}

fn generate_stars(rng: &mut ChaCha8Rng, n_stars: u32) -> HashSet<SystemCoordinate> {
    let mut stars = HashSet::new();

    for _ in 0..n_stars / 2 {
        let pos = Vec3::new(
            guassian_random(rng, 0.0, CORE_X_DIST as f32),
            guassian_random(rng, 0.0, CORE_Y_DIST as f32),
            guassian_random(rng, 0.0, GALAXY_THICKNESS as f32),
        );

        stars.insert(SystemCoordinate::new(
            pos.x.round() as SystemUnit,
            pos.z.round() as SystemUnit,
            pos.y.round() as SystemUnit,
        ));
    }

    for arm in 0..ARMS {
        for _ in 0..n_stars / 2 {
            let pos = spiral(
                guassian_random(rng, ARM_X_MEAN as f32, ARM_X_DIST as f32),
                guassian_random(rng, ARM_Y_MEAN as f32, ARM_Y_DIST as f32),
                guassian_random(rng, 0.0, GALAXY_THICKNESS as f32),
                arm as f32 * TAU / ARMS as f32,
            );

            stars.insert(SystemCoordinate::new(
                pos.x.round() as SystemUnit,
                pos.z.round() as SystemUnit,
                pos.y.round() as SystemUnit,
            ));
        }
    }

    stars
}

fn generate_galaxy_stars_system(
    mut q_galaxy: Query<&mut Galaxy>,
    seed: Res<ServerSeed>,
    mut mr_generate_galaxy: MessageReader<GenerateGalaxyMessage>,
    mut universe_systems: ResMut<UniverseSystems>,
) {
    for m in mr_generate_galaxy.read() {
        let Ok(mut galaxy) = q_galaxy.get_mut(m.0) else {
            return;
        };
        let mut rng = get_rng_for_sector(&seed, &Sector::ZERO);

        let mut stars = generate_stars(&mut rng, 1_000);

        // always there's never a star near the center black hole
        for z in -2..=2 {
            for y in -2..=2 {
                for x in -2..=2 {
                    stars.remove(&SystemCoordinate::new(x, y, z));
                }
            }
        }

        let mut spawn = None;

        for system in stars {
            let dist = system.abs().max_element();
            if spawn.is_none() && dist == (ARM_X_MEAN / 2) as i64 {
                spawn = Some(system);
            }

            let rand = 1.0 - (1.0 - rng.random::<f32>()).sqrt();
            let temperature = (rand * (MAX_TEMPERATURE - MIN_TEMPERATURE)) + MIN_TEMPERATURE;

            let star = Star::new(temperature);

            galaxy.stars.insert(
                system,
                GalaxyStar {
                    location: Location::new(
                        Vec3::ZERO,
                        Sector::splat((SYSTEM_SECTORS / 2) as SectorUnit) + system.negative_most_sector(),
                    ),
                    star,
                },
            );
        }

        let spawn = spawn.unwrap_or_else(|| *galaxy.iter_stars().next().unwrap().0);

        galaxy.set_spawn_system(spawn);
        universe_systems.set_spawn_system(spawn);
    }
}

pub(super) fn register(app: &mut App) {
    app.add_systems(
        GENERATE_GALAXY_SCHEDULE,
        generate_galaxy_stars_system.in_set(GalaxyGenerationOrder::StarsGeneration),
    );
}
